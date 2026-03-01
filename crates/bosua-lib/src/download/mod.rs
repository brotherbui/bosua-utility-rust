//! Download manager with multi-stage resume, retry, file locking, and progress display.
//!
//! Reads URLs from the configured `InputLinksFile` (`~/Downloads/links.txt`),
//! acquires a `FileLock` to prevent concurrent download conflicts, and retries
//! failed downloads up to `max_retries` from `DynamicConfig`.
//!
//! FShare downloads use aria2 with the 25% VIP link renewal trick:
//! 1. Resolve FShare link to VIP download URL
//! 2. Start download via aria2 with speed limit for first part
//! 3. At 25% progress, stop download and get a new VIP link
//! 4. Resume download with the new VIP link (aria2 auto-resumes)
//! 5. Continue until 100%

pub mod aria2;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

use futures_util::StreamExt;

use crate::cloud::fshare::FShareClient;
use crate::config::dynamic::DynamicConfig;
use crate::config::manager::DynamicConfigManager;
use crate::errors::{BosuaError, Result};
use crate::fileops::lock::FileLock;
use crate::http_client::HttpClient;
use crate::output;
use crate::output::progress::create_download_progress;
use crate::text::sanitize_filename;

use self::aria2::Aria2Client;

/// Aria2 RPC secret token (matches Go's `token:welovephongblack`).
const ARIA2_TOKEN: &str = "welovephongblack";

/// Result of a single download operation.
#[derive(Debug)]
pub struct DownloadResult {
    /// Local path where the file was saved.
    pub path: PathBuf,
    /// Total bytes written (including any previously resumed bytes).
    pub total_bytes: u64,
    /// Whether the download was resumed from a partial file.
    pub resumed: bool,
}

/// Manages file downloads with retry, resume, locking, and progress display.
pub struct DownloadManager {
    http_client: HttpClient,
    config: Arc<DynamicConfigManager>,
    lock: FileLock,
    download_dir: PathBuf,
    input_links_file: PathBuf,
}

impl DownloadManager {
    /// Create a new `DownloadManager`.
    ///
    /// - `http_client` — shared HTTP client for making requests
    /// - `config` — dynamic config manager (provides `max_retries`, `retry_delay`)
    /// - `download_lock_path` — path for the advisory lock file
    /// - `download_dir` — directory where downloaded files are saved
    /// - `input_links_file` — path to the file containing URLs to download
    pub fn new(
        http_client: HttpClient,
        config: Arc<DynamicConfigManager>,
        download_lock_path: impl Into<PathBuf>,
        download_dir: impl Into<PathBuf>,
        input_links_file: impl Into<PathBuf>,
    ) -> Self {
        Self {
            http_client,
            config,
            lock: FileLock::new(download_lock_path),
            download_dir: download_dir.into(),
            input_links_file: input_links_file.into(),
        }
    }

    /// Download all URLs listed in the input links file.
    ///
    /// Acquires the download lock, reads URLs from `input_links_file`, and
    /// downloads each one sequentially. Returns results for all successful
    /// downloads.
    pub async fn download(&self, is_cron: bool, skip_size: u64) -> Result<Vec<DownloadResult>> {
        let token = CancellationToken::new();
        self.download_with_context(token, is_cron, skip_size).await
    }

    /// Download all URLs with cancellation support.
    ///
    /// Same as [`download`](Self::download) but accepts a `CancellationToken`
    /// so callers (e.g. signal handler) can interrupt the operation.
    pub async fn download_with_context(
        &self,
        token: CancellationToken,
        is_cron: bool,
        skip_size: u64,
    ) -> Result<Vec<DownloadResult>> {
        // Acquire file lock to prevent concurrent downloads.
        let _lock_guard = self.lock.acquire()?;

        let urls = self.read_links().await?;
        if urls.is_empty() {
            if !is_cron {
                output::info("No URLs found in links file.");
            }
            return Ok(Vec::new());
        }

        let cfg = self.config.get_config().await;
        let total = urls.len();
        if !is_cron {
            output::info(&format!("Starting download of {} URL(s)...", total));
        }

        let mut results = Vec::new();

        for (idx, url) in urls.iter().enumerate() {
            if token.is_cancelled() {
                output::warning("Download cancelled by signal.");
                break;
            }

            if !is_cron {
                output::info(&format!("[{}/{}] {}", idx + 1, total, url));
            }

            match self
                .download_single_with_retry(&token, url, skip_size, &cfg)
                .await
            {
                Ok(result) => {
                    if !is_cron {
                        output::success(&format!(
                            "Downloaded: {} ({} bytes)",
                            result.path.display(),
                            result.total_bytes
                        ));
                    }
                    results.push(result);
                }
                Err(e) => {
                    output::error(&format!("Failed to download {}: {}", url, e));
                }
            }
        }

        if !is_cron {
            output::info(&format!(
                "Completed: {}/{} downloads succeeded.",
                results.len(),
                total
            ));
        }

        Ok(results)
    }

    /// Download explicit URLs with cancellation support.
    ///
    /// Like [`download_with_context`](Self::download_with_context) but takes
    /// URLs directly instead of reading from the links file.
    pub async fn download_urls_with_context(
        &self,
        token: CancellationToken,
        urls: &[String],
        is_cron: bool,
        skip_size: u64,
    ) -> Result<Vec<DownloadResult>> {
        let _lock_guard = self.lock.acquire()?;

        if urls.is_empty() {
            if !is_cron {
                output::info("No URLs to download.");
            }
            return Ok(Vec::new());
        }

        let cfg = self.config.get_config().await;
        let total = urls.len();
        if !is_cron {
            output::info(&format!("Starting download of {} URL(s)...", total));
        }

        let mut results = Vec::new();

        for (idx, url) in urls.iter().enumerate() {
            if token.is_cancelled() {
                output::warning("Download cancelled by signal.");
                break;
            }

            if !is_cron {
                output::info(&format!("[{}/{}] {}", idx + 1, total, url));
            }

            match self
                .download_single_with_retry(&token, url, skip_size, &cfg)
                .await
            {
                Ok(result) => {
                    if !is_cron {
                        output::success(&format!(
                            "Downloaded: {} ({} bytes)",
                            result.path.display(),
                            result.total_bytes
                        ));
                    }
                    results.push(result);
                }
                Err(e) => {
                    output::error(&format!("Failed to download {}: {}", url, e));
                }
            }
        }

        if !is_cron {
            output::info(&format!(
                "Completed: {}/{} downloads succeeded.",
                results.len(),
                total
            ));
        }

        Ok(results)
    }

    /// Read URLs from the input links file, one per line.
    /// Skips empty lines and lines starting with `#`.
    async fn read_links(&self) -> Result<Vec<String>> {
        let content = match tokio::fs::read_to_string(&self.input_links_file).await {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(BosuaError::Download(format!(
                    "Links file not found: {}",
                    self.input_links_file.display()
                )));
            }
            Err(e) => return Err(BosuaError::Io(e)),
        };

        let urls: Vec<String> = content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();

        Ok(urls)
    }

    /// Download a single URL with retry logic.
    async fn download_single_with_retry(
        &self,
        token: &CancellationToken,
        url: &str,
        skip_size: u64,
        cfg: &DynamicConfig,
    ) -> Result<DownloadResult> {
        let max_retries = cfg.max_retries;
        let retry_delay = Duration::from_secs(cfg.retry_delay as u64);

        let mut last_err: Option<BosuaError> = None;

        for attempt in 0..=max_retries {
            if token.is_cancelled() {
                return Err(BosuaError::Download("Download cancelled".into()));
            }

            if attempt > 0 {
                output::warning(&format!(
                    "Retry {}/{} for {}",
                    attempt, max_retries, url
                ));
                tokio::time::sleep(retry_delay).await;
            }

            match self.download_single(token, url, skip_size).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!(
                        url = url,
                        attempt = attempt + 1,
                        max_retries = max_retries,
                        "Download attempt failed: {}",
                        e
                    );
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| {
            BosuaError::Download(format!("Download failed after {} retries: {}", max_retries, url))
        }))
    }

    /// Download a single URL with resume support and progress display.
    async fn download_single(
        &self,
        token: &CancellationToken,
        url: &str,
        skip_size: u64,
    ) -> Result<DownloadResult> {
        let filename = filename_from_url(url);
        let dest = self.download_dir.join(&filename);

        // Check existing file size for resume.
        let existing_size = tokio::fs::metadata(&dest)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        let resumed = existing_size > 0 && existing_size >= skip_size;
        let resume_from = if resumed { existing_size } else { 0 };

        let client = self.http_client.get_client().await;

        // Build request with optional Range header for resume.
        let mut req = client.get(url);
        if resume_from > 0 {
            req = req.header("Range", format!("bytes={}-", resume_from));
            tracing::info!(
                url = url,
                resume_from = resume_from,
                "Resuming download"
            );
        }

        let response = req.send().await.map_err(BosuaError::Http)?;
        let status = response.status();

        if !status.is_success() && status.as_u16() != 206 {
            return Err(BosuaError::Download(format!(
                "HTTP {} for {}",
                status, url
            )));
        }

        // Determine total size for progress bar.
        let content_length = response.content_length().unwrap_or(0);
        let total_size = if resume_from > 0 {
            resume_from + content_length
        } else {
            content_length
        };

        // Skip files smaller than skip_size.
        if skip_size > 0 && total_size > 0 && total_size < skip_size {
            return Err(BosuaError::Download(format!(
                "File too small ({} bytes < {} skip_size): {}",
                total_size, skip_size, url
            )));
        }

        // Ensure download directory exists.
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Open file for writing (append if resuming).
        let mut file = if resume_from > 0 {
            tokio::fs::OpenOptions::new()
                .append(true)
                .open(&dest)
                .await?
        } else {
            tokio::fs::File::create(&dest).await?
        };

        // Set up progress bar.
        let pb = create_download_progress(total_size);
        if resume_from > 0 {
            pb.set_position(resume_from);
        }

        // Stream the response body.
        let mut bytes_written = resume_from;
        let mut stream = response.bytes_stream();

        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    pb.abandon_with_message("Cancelled");
                    file.flush().await?;
                    return Err(BosuaError::Download("Download cancelled".into()));
                }
                chunk = stream.next() => {
                    match chunk {
                        Some(Ok(data)) => {
                            file.write_all(&data).await?;
                            bytes_written += data.len() as u64;
                            pb.set_position(bytes_written);
                        }
                        Some(Err(e)) => {
                            pb.abandon_with_message("Error");
                            file.flush().await?;
                            return Err(BosuaError::Http(e));
                        }
                        None => break, // Stream finished
                    }
                }
            }
        }

        file.flush().await?;
        pb.finish_with_message("Done");

        Ok(DownloadResult {
            path: dest,
            total_bytes: bytes_written,
            resumed: resume_from > 0,
        })
    }

    // -----------------------------------------------------------------------
    // FShare download via aria2 with 25% VIP link renewal
    // -----------------------------------------------------------------------

    /// Download an FShare link using aria2 with the 25% VIP link renewal trick.
    ///
    /// Matches Go's `DoDownloadWithContext`:
    /// 1. Resolve FShare link → VIP URL
    /// 2. HEAD to get file size and filename
    /// 3. Start download via aria2 (with speed limit for first part if < 100MB)
    /// 4. Poll aria2 status every second
    /// 5. At 25% progress, remove download, get new VIP link, resume
    /// 6. Continue until 100%
    pub async fn do_fshare_download(
        &self,
        fshare_link: &str,
        fshare_client: &FShareClient,
        token: &CancellationToken,
        is_cron: bool,
        skip_size: u64,
    ) -> Result<DownloadResult> {
        // 1. Resolve VIP link
        let vip1 = fshare_client.resolve_vip_link(fshare_link).await?;
        if vip1.is_empty() {
            return Err(BosuaError::Download("Empty VIP link returned".into()));
        }

        // 2. HEAD request to get file size and filename
        let client = self.http_client.get_client().await;
        let head_resp = client.head(&vip1).send().await.map_err(BosuaError::Http)?;
        if !head_resp.status().is_success() {
            output::error("File not found");
            return Err(BosuaError::Download("File not found".into()));
        }

        let file_size: i64 = head_resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let skip_bytes = skip_size as i64 * 1_048_576;
        if skip_size > 0 && file_size > 0 && file_size <= skip_bytes {
            return Err(BosuaError::Download(format!(
                "File too small ({} bytes), skipping",
                file_size
            )));
        }

        // Extract filename from VIP URL
        let raw_name = vip1
            .rsplit('/')
            .next()
            .and_then(|s| s.split('?').next())
            .unwrap_or("download");
        let decoded_name = url_decode(raw_name);
        let filename = crate::text::normalize(&decoded_name);

        let small_file = file_size > 0 && file_size < 10_000_000 && is_cron;

        println!("Downloading {}...", filename);

        // 3. Create aria2 client
        let aria2 = Aria2Client::new(
            self.http_client.clone(),
            None,
            Some(ARIA2_TOKEN.to_string()),
        );

        let download_dir = self.download_dir.to_string_lossy().to_string();

        // Small file: simple aria2 download without 25% trick
        if small_file {
            let gid = self.aria2_start_download(&aria2, &vip1, &filename, &download_dir, true).await?;
            let result = self.aria2_wait_for_completion(&aria2, &gid, token, Duration::from_secs(300)).await;
            if !result {
                let _ = aria2.remove(&gid).await;
                return Err(BosuaError::Download("Small file download failed".into()));
            }
            output::success(&format!("Download {} finished successfully", filename));
            return Ok(DownloadResult {
                path: self.download_dir.join(&filename),
                total_bytes: file_size as u64,
                resumed: false,
            });
        }

        // 4. Start download via aria2 with speed limit for first part
        let pb = create_download_progress(file_size as u64);
        let gid = self.aria2_start_download(&aria2, &vip1, &filename, &download_dir, true).await?;

        // 5. Poll status, at 25% get new VIP link and resume
        let mut download_completed = false;
        loop {
            if token.is_cancelled() {
                let _ = aria2.remove(&gid).await;
                pb.abandon_with_message("Cancelled");
                return Err(BosuaError::Download("Download cancelled".into()));
            }

            let (progress, _info, speed) = self.aria2_get_status(&aria2, &gid).await;

            if progress == 9999.0 {
                pb.abandon_with_message("File not found");
                return Err(BosuaError::Download("File not found".into()));
            }

            if file_size > 0 {
                let completed = (progress * file_size as f64) as u64;
                pb.set_position(completed);
                if !speed.is_empty() {
                    pb.set_message(format!("{}/s", speed));
                }
            }

            if progress >= 0.25 && !download_completed {
                // 25% reached: remove download, get new VIP link, resume
                let _ = aria2.remove(&gid).await;

                let vip2 = match fshare_client.resolve_vip_link(fshare_link).await {
                    Ok(v) => v,
                    Err(e) => {
                        pb.abandon_with_message("VIP renewal failed");
                        return Err(BosuaError::Download(format!("VIP link renewal failed: {e}")));
                    }
                };

                // Start new download (aria2 auto-resumes from existing partial file)
                let new_gid = self.aria2_start_download(&aria2, &vip2, &filename, &download_dir, false).await?;

                // Continue polling the new download
                let result = self.aria2_poll_with_progress(&aria2, &new_gid, token, file_size, &pb).await;
                if !result {
                    let _ = aria2.remove(&new_gid).await;
                    pb.abandon_with_message("Failed");
                    return Err(BosuaError::Download("Download failed after VIP renewal".into()));
                }
                download_completed = true;
                break;
            }

            if progress >= 1.0 {
                download_completed = true;
                break;
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        if download_completed {
            pb.finish_with_message("Done");
        }

        Ok(DownloadResult {
            path: self.download_dir.join(&filename),
            total_bytes: file_size as u64,
            resumed: false,
        })
    }

    // -----------------------------------------------------------------------
    // Aria2 helpers
    // -----------------------------------------------------------------------

    /// Start a download via aria2 addUri. Returns the GID.
    async fn aria2_start_download(
        &self,
        aria2: &Aria2Client,
        url: &str,
        filename: &str,
        output_dir: &str,
        is_first_part: bool,
    ) -> Result<String> {
        let mut options = serde_json::json!({
            "max-connection-per-server": "16",
            "split": "16",
            "min-split-size": "1M",
            "max-concurrent-downloads": "8",
            "dir": output_dir,
            "out": filename,
            "user-agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17 Safari/605.1.15",
        });

        // For first part of files < 100MB, limit speed to fileSize/5
        if is_first_part {
            let client = self.http_client.get_client().await;
            if let Ok(resp) = client.head(url).send().await {
                if resp.status().is_success() {
                    let file_size: i64 = resp
                        .headers()
                        .get("content-length")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    if file_size > 0 && file_size < 104_857_600 {
                        let speed_limit = file_size / 5;
                        options["max-download-limit"] = serde_json::json!(format!("{speed_limit}"));
                    }
                }
            }
        }

        let result = aria2
            .add_uri(vec![url.to_string()], Some(options))
            .await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| BosuaError::Download("aria2 addUri returned no GID".into()))
    }

    /// Get download status from aria2. Returns (progress 0.0-1.0, info_string, speed_string).
    /// Returns (9999.0, ...) if file not found.
    async fn aria2_get_status(&self, aria2: &Aria2Client, gid: &str) -> (f64, String, String) {
        let result = match aria2.tell_status(gid, None).await {
            Ok(v) => v,
            Err(_) => return (0.0, String::new(), String::new()),
        };

        let obj = match result.as_object() {
            Some(o) => o,
            None => return (0.0, String::new(), String::new()),
        };

        // Check for error code 3 (file not found)
        if let Some(code) = obj.get("errorCode").and_then(|v| v.as_str()) {
            if code == "3" {
                return (9999.0, "failed".into(), "0".into());
            }
        }

        let speed: f64 = obj
            .get("downloadSpeed")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        let completed: f64 = obj
            .get("completedLength")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        let total: f64 = obj
            .get("totalLength")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        let progress = if total > 0.0 { completed / total } else { 0.0 };
        let speed_str = humanize_bytes(speed as u64);

        (progress, format!("{:.2}%", progress * 100.0), speed_str)
    }

    /// Wait for an aria2 download to complete (simple polling, no progress bar).
    async fn aria2_wait_for_completion(
        &self,
        aria2: &Aria2Client,
        gid: &str,
        token: &CancellationToken,
        timeout: Duration,
    ) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if token.is_cancelled() { return false; }
            if tokio::time::Instant::now() > deadline { return false; }

            let (progress, _, _) = self.aria2_get_status(aria2, gid).await;
            if progress == 9999.0 { return false; }
            if progress >= 1.0 { return true; }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    /// Poll aria2 status with progress bar updates until completion.
    async fn aria2_poll_with_progress(
        &self,
        aria2: &Aria2Client,
        gid: &str,
        token: &CancellationToken,
        file_size: i64,
        pb: &indicatif::ProgressBar,
    ) -> bool {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30 * 60);
        loop {
            if token.is_cancelled() {
                let _ = aria2.remove(gid).await;
                return false;
            }
            if tokio::time::Instant::now() > deadline {
                output::error("Download timeout after 30 minutes");
                return false;
            }

            let (progress, _, speed) = self.aria2_get_status(aria2, gid).await;
            if progress == 9999.0 { return false; }

            if file_size > 0 {
                let completed = (progress * file_size as f64) as u64;
                pb.set_position(completed);
                if !speed.is_empty() {
                    pb.set_message(format!("{}/s", speed));
                }
            }

            if progress >= 1.0 { return true; }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

/// Extract a filename from a URL, falling back to a sanitized version of the
/// last path segment or "download" if the URL has no path.
fn filename_from_url(url: &str) -> String {
    url.rsplit('/')
        .next()
        .and_then(|seg| {
            // Strip query string if present.
            let name = seg.split('?').next().unwrap_or(seg);
            if name.is_empty() {
                None
            } else {
                Some(sanitize_filename(name))
            }
        })
        .unwrap_or_else(|| "download".to_string())
}

/// Simple URL percent-decoding (e.g. `%20` → ` `).
fn url_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""),
                16,
            ) {
                result.push(byte as char);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

/// Convert bytes to human-readable string (e.g. `1.5 MB`).
fn humanize_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filename_from_url_simple() {
        assert_eq!(
            filename_from_url("https://example.com/file.zip"),
            "file.zip"
        );
    }

    #[test]
    fn test_filename_from_url_with_query() {
        assert_eq!(
            filename_from_url("https://example.com/file.zip?token=abc"),
            "file.zip"
        );
    }

    #[test]
    fn test_filename_from_url_no_path() {
        assert_eq!(filename_from_url("https://example.com/"), "download");
    }

    #[test]
    fn test_filename_from_url_special_chars() {
        assert_eq!(
            filename_from_url("https://example.com/my file (1).zip"),
            "my_file__1_.zip"
        );
    }

    #[test]
    fn test_filename_from_url_nested_path() {
        assert_eq!(
            filename_from_url("https://cdn.example.com/a/b/c/data.tar.gz"),
            "data.tar.gz"
        );
    }
}
