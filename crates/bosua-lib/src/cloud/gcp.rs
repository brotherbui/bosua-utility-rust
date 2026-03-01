//! Google Cloud Platform integration client.
//!
//! Provides VM file browsing, download operations, and media playback
//! from GCP instances. Uses the GCP IP/domain from `SimplifiedConfig`
//! and `DynamicConfig` to connect to the remote instance's HTTP API.

use std::path::Path;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::cloud::CloudClient;
use crate::config::dynamic::DynamicConfig;
use crate::errors::{BosuaError, Result};
use crate::http_client::HttpClient;

/// Default API key matching Go's `constants.DefaultAPIKey`.
pub const DEFAULT_API_KEY: &str = "ConGaNoHayLaCaOlala";
pub const ENV_BOSUA_API_KEY: &str = "BOSUA_API_KEY";

const VIDEO_EXTENSIONS: &[&str] = &[".mkv", ".mp4", ".mov", ".avi", ".wmv", ".flv", ".webm", ".m4v"];

/// Check if a filename has a video extension.
pub fn is_video_file_ext(name: &str) -> bool {
    let lower = name.to_lowercase();
    VIDEO_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Collect signed URLs for video files from a file listing.
pub fn collect_video_urls(files: &[GcpFile]) -> Vec<String> {
    files
        .iter()
        .filter(|f| !f.is_directory())
        .filter(|f| {
            f.mime_type.as_deref().map_or(false, |m| m.starts_with("video/"))
                || is_video_file_ext(&f.name)
        })
        .filter_map(|f| f.signed_url.clone())
        .filter(|u| !u.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// Data models
// ---------------------------------------------------------------------------

/// Metadata for a file on a GCP VM (mirrors Go's `RemoteFile`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpFile {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(rename = "isDirectory", default)]
    pub is_directory: bool,
    /// Legacy alias kept for backward compat with older server responses.
    #[serde(rename = "isDir", default)]
    pub is_dir: bool,
    #[serde(rename = "mimeType", default)]
    pub mime_type: Option<String>,
    #[serde(rename = "signedUrl", default)]
    pub signed_url: Option<String>,
    #[serde(rename = "modTime", default)]
    pub mod_time: Option<String>,
}

impl GcpFile {
    /// Returns true if this entry is a directory (checks both fields).
    pub fn is_directory(&self) -> bool {
        self.is_directory || self.is_dir
    }
}

/// Response from the GCP VM file listing API (mirrors Go's `RemoteListResponse`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpFileList {
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub message: String,
    pub files: Vec<GcpFile>,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub total: usize,
    #[serde(default)]
    pub error: String,
}

/// Response from the `/resolve-episode` API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveEpisodeResponse {
    pub success: bool,
    #[serde(default)]
    pub message: String,
    #[serde(rename = "signedUrl", default)]
    pub signed_url: Option<String>,
    #[serde(rename = "resolvedName", default)]
    pub resolved_name: Option<String>,
    #[serde(rename = "episodeNumber", default)]
    pub episode_number: Option<String>,
    #[serde(rename = "seasonNumber", default)]
    pub season_number: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Result of a push (upload) operation to a GCP VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpPushResult {
    pub success: bool,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub remote_path: String,
}

// ---------------------------------------------------------------------------
// GcpClient
// ---------------------------------------------------------------------------

/// GCP client for VM file browsing, downloads, and media playback.
///
/// Connects to a remote GCP instance via its HTTP API using the IP/domain
/// from configuration. The base URL is derived from `DynamicConfig.gcp_ip`
/// (or `gcp_domain`) falling back to `SimplifiedConfig` values.
pub struct GcpClient {
    http: HttpClient,
    base_url: String,
}

impl GcpClient {
    /// Create a new `GcpClient`.
    ///
    /// The base URL is derived from `DynamicConfig` fields (`gcp_ip` / `gcp_domain`),
    /// falling back to `SimplifiedConfig` values when the dynamic fields are empty.
    pub fn new(http: HttpClient, config: &DynamicConfig) -> Self {
        let base_url = Self::derive_base_url(config);
        Self { http, base_url }
    }

    /// Create a `GcpClient` using `SimplifiedConfig` static values directly.
    pub fn from_simplified(http: HttpClient) -> Self {
        let sc = crate::config::simplified::SimplifiedConfig::get();
        let host = if !sc.gcp_domain.is_empty() {
            &sc.gcp_domain
        } else if !sc.gcp_ip.is_empty() {
            &sc.gcp_ip
        } else {
            "localhost"
        };
        Self {
            http,
            base_url: format!("https://{host}"),
        }
    }

    /// Update the base URL when `DynamicConfig` changes.
    pub fn update_from_config(&mut self, config: &DynamicConfig) {
        self.base_url = Self::derive_base_url(config);
    }

    /// Derive the base URL from config, preferring domain over IP.
    /// Falls back to environment variables (matching Go's `ComputeGCPAPIUrl`).
    fn derive_base_url(config: &DynamicConfig) -> String {
        // Check dynamic config first
        if !config.gcp_domain.is_empty() {
            return format!("https://{}", config.gcp_domain);
        }
        if !config.gcp_ip.is_empty() {
            return format!("https://{}", config.gcp_ip);
        }
        // Fall back to environment variables (matching Go behavior)
        let sc = crate::config::simplified::SimplifiedConfig::get();
        if !sc.gcp_domain.is_empty() {
            return format!("https://{}", sc.gcp_domain);
        }
        if !sc.gcp_ip.is_empty() {
            return format!("https://{}", sc.gcp_ip);
        }
        // Last resort: check BACKEND_IP / BACKEND_DOMAIN
        if !sc.server_domain.is_empty() {
            return format!("https://{}", sc.server_domain);
        }
        if !sc.server_ip.is_empty() {
            let ip = &sc.server_ip;
            if ip.contains(':') {
                return format!("http://{ip}");
            }
            return format!("http://{ip}:8080");
        }
        "https://localhost".to_string()
    }

    /// Get the current base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    // -----------------------------------------------------------------------
    // File browsing
    // -----------------------------------------------------------------------

    /// List files on the GCP VM at the given remote path.
    pub async fn list_files(&self, remote_path: Option<&str>) -> Result<GcpFileList> {
        let client = self.http.get_client().await;
        let url = format!("{}/files", self.base_url);

        let mut params: Vec<(&str, &str)> = Vec::new();
        if let Some(p) = remote_path {
            params.push(("path", p));
        }

        let resp = client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "gcp".into(),
                message: format!("list_files failed ({status}): {body}"),
            });
        }

        resp.json::<GcpFileList>().await.map_err(BosuaError::Http)
    }

    /// Browse files interactively on the GCP VM.
    ///
    /// Returns the entries at the given path for display in a TUI or CLI.
    pub async fn browse(&self, remote_path: Option<&str>) -> Result<Vec<GcpFile>> {
        let list = self.list_files(remote_path).await?;
        Ok(list.files)
    }

    // -----------------------------------------------------------------------
    // Download
    // -----------------------------------------------------------------------

    /// Download a file from the GCP VM to a local path.
    pub async fn download_file(
        &self,
        remote_path: &str,
        local_path: &Path,
    ) -> Result<()> {
        let client = self.http.get_client().await;
        let url = format!("{}/download", self.base_url);

        let resp = client
            .get(&url)
            .query(&[("path", remote_path)])
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "gcp".into(),
                message: format!("download failed ({status}): {body}"),
            });
        }

        // Ensure parent directory exists
        if let Some(parent) = local_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let bytes = resp.bytes().await.map_err(BosuaError::Http)?;
        tokio::fs::write(local_path, &bytes).await?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Media playback
    // -----------------------------------------------------------------------

    /// Request media playback on the GCP VM.
    ///
    /// Sends a play request to the remote instance which handles the actual
    /// media player invocation.
    pub async fn play(&self, remote_path: &str, player: Option<&str>) -> Result<String> {
        let client = self.http.get_client().await;
        let url = format!("{}/play", self.base_url);

        let mut body = serde_json::json!({ "path": remote_path });
        if let Some(p) = player {
            body["player"] = serde_json::json!(p);
        }

        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "gcp".into(),
                message: format!("play failed ({status}): {text}"),
            });
        }

        let result: serde_json::Value = resp.json().await.map_err(BosuaError::Http)?;
        Ok(result
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("playback started")
            .to_string())
    }

    // -----------------------------------------------------------------------
    // Push (upload to VM)
    // -----------------------------------------------------------------------

    /// Push (upload) a local file to the GCP VM.
    pub async fn push_file(
        &self,
        local_path: &Path,
        remote_path: &str,
    ) -> Result<GcpPushResult> {
        let client = self.http.get_client().await;
        let url = format!("{}/upload", self.base_url);

        let file_bytes = tokio::fs::read(local_path).await?;

        let resp = client
            .post(&url)
            .query(&[("path", remote_path)])
            .header("Content-Type", "application/octet-stream")
            .body(file_bytes)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "gcp".into(),
                message: format!("push failed ({status}): {body}"),
            });
        }

        resp.json::<GcpPushResult>().await.map_err(BosuaError::Http)
    }

    // -----------------------------------------------------------------------
    // Remote file listing (matches Go's FetchRemoteFiles)
    // -----------------------------------------------------------------------

    /// Fetch remote file listing from the GCP server (matches Go's `FetchRemoteFiles`).
    pub async fn fetch_remote_files(&self, path: &str) -> Result<GcpFileList> {
        let client = self.http.get_client().await;
        let api_key = std::env::var(ENV_BOSUA_API_KEY).unwrap_or_else(|_| DEFAULT_API_KEY.to_string());
        let url = format!("{}/list-files", self.base_url);

        let mut req = client.get(&url).query(&[("path", path)]);
        req = req.header("Content-Type", "application/json");
        if !api_key.is_empty() {
            req = req.header("X-API-Key", &api_key);
        }

        let resp = req.send().await.map_err(BosuaError::Http)?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "gcp".into(),
                message: format!("list-files failed ({status}): {body}"),
            });
        }
        resp.json::<GcpFileList>().await.map_err(BosuaError::Http)
    }

    // -----------------------------------------------------------------------
    // Episode resolution (matches Go's ConstructEpisodeURL)
    // -----------------------------------------------------------------------

    /// Resolve an episode pattern (e.g. `s01e07`, `e7`, `07`) to a signed URL.
    pub async fn resolve_episode(&self, episode_input: &str) -> Result<String> {
        let client = self.http.get_client().await;
        let api_key = std::env::var(ENV_BOSUA_API_KEY).unwrap_or_else(|_| DEFAULT_API_KEY.to_string());
        let url = format!("{}/resolve-episode", self.base_url);

        let payload = serde_json::json!({ "episode": episode_input });
        let mut req = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload);
        if !api_key.is_empty() {
            req = req.header("X-API-Key", &api_key);
        }

        let resp = req.send().await.map_err(BosuaError::Http)?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            return Err(BosuaError::Cloud {
                service: "gcp".into(),
                message: format!("backend returned non-200 status: {status}"),
            });
        }

        let body: ResolveEpisodeResponse = resp.json().await.map_err(BosuaError::Http)?;
        if !body.success {
            let err_msg = body.error.unwrap_or_else(|| "Unknown error".into());
            return Err(BosuaError::Cloud {
                service: "gcp".into(),
                message: format!("backend error: {err_msg}"),
            });
        }

        body.signed_url
            .filter(|u| !u.trim().is_empty())
            .ok_or_else(|| BosuaError::Cloud {
                service: "gcp".into(),
                message: "no signedUrl in response".into(),
            })
    }

    // -----------------------------------------------------------------------
    // Directory / file search (matches Go's search/discovery.go)
    // -----------------------------------------------------------------------

    /// Find directories matching any of the search terms (one level deep from root).
    pub async fn find_matching_directories(&self, search_terms: &[String]) -> Result<Vec<GcpFile>> {
        let mut matching = Vec::new();
        let root = self.fetch_remote_files("").await?;
        if !root.success {
            return Err(BosuaError::Cloud { service: "gcp".into(), message: format!("server error: {}", root.error) });
        }

        // Search root entries
        Self::search_dirs_in_listing(&root.files, search_terms, &mut matching);

        // Search one level deep
        for f in &root.files {
            if f.is_directory() {
                if let Ok(sub) = self.fetch_remote_files(&f.path).await {
                    if sub.success {
                        Self::search_dirs_in_listing(&sub.files, search_terms, &mut matching);
                    }
                }
            }
        }
        Ok(matching)
    }

    fn search_dirs_in_listing(files: &[GcpFile], terms: &[String], out: &mut Vec<GcpFile>) {
        for f in files {
            if !f.is_directory() { continue; }
            let name_lower = f.name.to_lowercase();
            let path_lower = f.path.to_lowercase();
            for term in terms {
                let t = term.to_lowercase();
                if name_lower.contains(&t) || path_lower.contains(&t) {
                    out.push(f.clone());
                    break;
                }
            }
        }
    }

    /// Find the best matching directory for a single search term.
    pub async fn find_best_matching_directory(&self, search_term: &str) -> Result<Option<GcpFile>> {
        let terms = vec![search_term.to_string()];
        let candidates = self.find_matching_directories(&terms).await?;
        if candidates.is_empty() { return Ok(None); }
        let best = candidates.into_iter().max_by_key(|f| Self::dir_match_score(&f.name, search_term));
        Ok(best)
    }

    fn dir_match_score(name: &str, term: &str) -> i32 {
        let n = name.to_lowercase();
        let t = term.to_lowercase();
        if n == t { 100 }
        else if n.starts_with(&t) { 80 }
        else if n.contains(&t) { 60 }
        else { 0 }
    }

    /// Find the best matching video file in a directory.
    pub async fn find_best_matching_file(&self, dir_path: &str, search_term: &str) -> Result<Option<GcpFile>> {
        let resp = self.fetch_remote_files(dir_path).await?;
        if !resp.success {
            return Err(BosuaError::Cloud { service: "gcp".into(), message: format!("server error: {}", resp.error) });
        }
        let lower_term = search_term.to_lowercase();
        let candidates: Vec<&GcpFile> = resp.files.iter()
            .filter(|f| !f.is_directory())
            .filter(|f| {
                f.mime_type.as_deref().map_or(false, |m| m.starts_with("video/"))
                    || is_video_file_ext(&f.name)
            })
            .filter(|f| {
                f.name.to_lowercase().contains(&lower_term)
                    || f.path.to_lowercase().contains(&lower_term)
            })
            .collect();

        if candidates.is_empty() { return Ok(None); }
        let best = candidates.into_iter().max_by_key(|f| {
            let n = f.name.to_lowercase();
            if n == lower_term { 100i32 }
            else if n.starts_with(&lower_term) { 80 }
            else if n.contains(&lower_term) { 60 }
            else { 0 }
        });
        Ok(best.cloned())
    }
}

// ---------------------------------------------------------------------------
// CloudClient trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl CloudClient for GcpClient {
    fn name(&self) -> &str {
        "Google Cloud Platform"
    }

    /// GCP authentication is handled at the instance level (SSH keys, service
    /// accounts). This is a no-op that verifies connectivity.
    async fn authenticate(&self) -> Result<()> {
        let client = self.http.get_client().await;
        let url = format!("{}/health", self.base_url);

        let resp = client.get(&url).send().await.map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            return Err(BosuaError::Auth(
                "GCP instance health check failed".into(),
            ));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_base_url_prefers_domain() {
        let mut config = DynamicConfig::default();
        config.gcp_domain = "gcp.example.com".into();
        config.gcp_ip = "10.0.0.1".into();
        assert_eq!(GcpClient::derive_base_url(&config), "https://gcp.example.com");
    }

    #[test]
    fn test_derive_base_url_falls_back_to_ip() {
        let mut config = DynamicConfig::default();
        config.gcp_domain = String::new();
        config.gcp_ip = "10.0.0.1".into();
        assert_eq!(GcpClient::derive_base_url(&config), "https://10.0.0.1");
    }

    #[test]
    fn test_gcp_client_name() {
        let config = DynamicConfig::default();
        let http = HttpClient::from_defaults().unwrap();
        let client = GcpClient::new(http, &config);
        assert_eq!(client.name(), "Google Cloud Platform");
    }

    #[test]
    fn test_update_from_config() {
        let mut config = DynamicConfig::default();
        config.gcp_ip = "10.0.0.1".into();
        let http = HttpClient::from_defaults().unwrap();
        let mut client = GcpClient::new(http, &config);
        assert_eq!(client.base_url(), "https://10.0.0.1");

        let mut updated = DynamicConfig::default();
        updated.gcp_ip = "192.168.1.100".into();
        client.update_from_config(&updated);
        assert_eq!(client.base_url(), "https://192.168.1.100");
    }

    #[test]
    fn test_gcp_file_serialization() {
        let file = GcpFile {
            name: "test.txt".into(),
            path: "/home/user/test.txt".into(),
            size: Some(1024),
            is_directory: false,
            is_dir: false,
            mime_type: None,
            signed_url: None,
            mod_time: Some("2024-01-01T00:00:00Z".into()),
        };
        let json = serde_json::to_string(&file).unwrap();
        let deserialized: GcpFile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test.txt");
        assert_eq!(deserialized.size, Some(1024));
        assert!(!deserialized.is_directory());
    }

    #[test]
    fn test_gcp_file_list_serialization() {
        let list = GcpFileList {
            success: true,
            message: String::new(),
            files: vec![GcpFile {
                name: "dir".into(),
                path: "/home/user/dir".into(),
                size: None,
                is_directory: true,
                is_dir: false,
                mime_type: None,
                signed_url: None,
                mod_time: None,
            }],
            path: "/home/user".into(),
            total: 1,
            error: String::new(),
        };
        let json = serde_json::to_string(&list).unwrap();
        let deserialized: GcpFileList = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.files.len(), 1);
        assert!(deserialized.files[0].is_directory());
    }

    #[test]
    fn test_gcp_push_result_serialization() {
        let result = GcpPushResult {
            success: true,
            message: "uploaded".into(),
            remote_path: "/tmp/file.txt".into(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: GcpPushResult = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
        assert_eq!(deserialized.remote_path, "/tmp/file.txt");
    }
}
