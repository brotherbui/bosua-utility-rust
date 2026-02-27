//! Download manager with multi-stage resume, retry, file locking, and progress display.
//!
//! Reads URLs from the configured `InputLinksFile` (`~/Downloads/links.txt`),
//! acquires a `FileLock` to prevent concurrent download conflicts, and retries
//! failed downloads up to `max_retries` from `DynamicConfig`.

pub mod aria2;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

use futures_util::StreamExt;

use crate::config::dynamic::DynamicConfig;
use crate::config::manager::DynamicConfigManager;
use crate::errors::{BosuaError, Result};
use crate::fileops::lock::FileLock;
use crate::http_client::HttpClient;
use crate::output;
use crate::output::progress::create_download_progress;
use crate::text::sanitize_filename;

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
