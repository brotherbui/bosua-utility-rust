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

// ---------------------------------------------------------------------------
// Data models
// ---------------------------------------------------------------------------

/// Metadata for a file on a GCP VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpFile {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(rename = "isDir", default)]
    pub is_dir: bool,
    #[serde(rename = "modTime", default)]
    pub mod_time: Option<String>,
}

/// Response from the GCP VM file listing API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpFileList {
    pub files: Vec<GcpFile>,
    #[serde(default)]
    pub path: String,
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
    fn derive_base_url(config: &DynamicConfig) -> String {
        let host = if !config.gcp_domain.is_empty() {
            &config.gcp_domain
        } else if !config.gcp_ip.is_empty() {
            &config.gcp_ip
        } else {
            "localhost"
        };
        format!("https://{host}")
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
    fn test_derive_base_url_defaults_to_localhost() {
        let config = DynamicConfig::default();
        assert_eq!(GcpClient::derive_base_url(&config), "https://localhost");
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
        let config = DynamicConfig::default();
        let http = HttpClient::from_defaults().unwrap();
        let mut client = GcpClient::new(http, &config);
        assert_eq!(client.base_url(), "https://localhost");

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
            is_dir: false,
            mod_time: Some("2024-01-01T00:00:00Z".into()),
        };
        let json = serde_json::to_string(&file).unwrap();
        let deserialized: GcpFile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test.txt");
        assert_eq!(deserialized.size, Some(1024));
        assert!(!deserialized.is_dir);
    }

    #[test]
    fn test_gcp_file_list_serialization() {
        let list = GcpFileList {
            files: vec![GcpFile {
                name: "dir".into(),
                path: "/home/user/dir".into(),
                size: None,
                is_dir: true,
                mod_time: None,
            }],
            path: "/home/user".into(),
        };
        let json = serde_json::to_string(&list).unwrap();
        let deserialized: GcpFileList = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.files.len(), 1);
        assert!(deserialized.files[0].is_dir);
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
