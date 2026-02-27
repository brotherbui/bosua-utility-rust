//! HTTP server configuration types.
//!
//! `ServerConfig` holds the listener address and TLS settings.
//! `EndpointConfig` describes a single API endpoint for centralized routing.

use std::path::PathBuf;

/// Configuration for the HTTP/HTTPS server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// TCP port to listen on.
    pub port: u16,
    /// Bind address (e.g. "0.0.0.0").
    pub host: String,
    /// Path to the TLS certificate file (PEM).
    pub cert_file: Option<PathBuf>,
    /// Path to the TLS private key file (PEM).
    pub key_file: Option<PathBuf>,
    /// Whether to enable TLS.
    pub tls: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "0.0.0.0".into(),
            cert_file: None,
            key_file: None,
            tls: false,
        }
    }
}

impl ServerConfig {
    /// Validate the configuration, returning an error if TLS is enabled but
    /// cert or key files are missing.
    pub fn validate(&self) -> crate::errors::Result<()> {
        if self.tls {
            if self.cert_file.is_none() {
                return Err(crate::errors::BosuaError::Config(
                    "TLS enabled but cert_file is not set".into(),
                ));
            }
            if self.key_file.is_none() {
                return Err(crate::errors::BosuaError::Config(
                    "TLS enabled but key_file is not set".into(),
                ));
            }
        }
        Ok(())
    }

    /// Returns the socket address string "host:port".
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Describes a single HTTP endpoint for centralized configuration.
#[derive(Debug, Clone)]
pub struct EndpointConfig {
    /// URL path (e.g. "/health").
    pub path: &'static str,
    /// HTTP method.
    pub method: axum::http::Method,
    /// Human-readable description.
    pub description: &'static str,
    /// Whether this endpoint requires API key authentication.
    pub requires_auth: bool,
}

/// Returns the full list of endpoint configurations for the server.
pub fn endpoint_configurations() -> Vec<EndpointConfig> {
    use axum::http::Method;
    vec![
        // Health
        EndpointConfig { path: "/health", method: Method::GET, description: "Health check", requires_auth: false },
        // Links
        EndpointConfig { path: "/append-links", method: Method::POST, description: "Append download links", requires_auth: true },
        // Search
        EndpointConfig { path: "/search", method: Method::GET, description: "Search across sources", requires_auth: true },
        // Files
        EndpointConfig { path: "/files", method: Method::GET, description: "List files", requires_auth: true },
        EndpointConfig { path: "/files", method: Method::POST, description: "Upload file", requires_auth: true },
        EndpointConfig { path: "/files", method: Method::DELETE, description: "Delete file", requires_auth: true },
        // Config
        EndpointConfig { path: "/config", method: Method::GET, description: "Get configuration", requires_auth: true },
        EndpointConfig { path: "/config", method: Method::PUT, description: "Update configuration", requires_auth: true },
        EndpointConfig { path: "/config/reset", method: Method::POST, description: "Reset configuration to defaults", requires_auth: true },
        // Daemon
        EndpointConfig { path: "/daemon/status", method: Method::GET, description: "Daemon status", requires_auth: true },
        // Notifications
        EndpointConfig { path: "/notifications", method: Method::GET, description: "List notifications", requires_auth: true },
        EndpointConfig { path: "/notifications/sse", method: Method::GET, description: "SSE notification stream", requires_auth: false },
        // Logs
        EndpointConfig { path: "/logs", method: Method::GET, description: "View logs", requires_auth: true },
        // Stats
        EndpointConfig { path: "/stats", method: Method::GET, description: "Server statistics", requires_auth: true },
        // Memprofile
        EndpointConfig { path: "/memprofile", method: Method::GET, description: "Memory profile", requires_auth: true },
        // PDF
        EndpointConfig { path: "/pdf", method: Method::POST, description: "PDF processing", requires_auth: true },
        // LaTeX
        EndpointConfig { path: "/latex2pdf", method: Method::POST, description: "LaTeX to PDF conversion", requires_auth: true },
        // Onflix
        EndpointConfig { path: "/onflix", method: Method::POST, description: "Onflix video download", requires_auth: true },
        // FShare
        EndpointConfig { path: "/fshare/links", method: Method::POST, description: "Submit FShare links", requires_auth: true },
        EndpointConfig { path: "/fshare/scan", method: Method::GET, description: "Scan FShare folder", requires_auth: true },
        // GDrive
        EndpointConfig { path: "/gdrive/files", method: Method::GET, description: "List GDrive files", requires_auth: true },
        EndpointConfig { path: "/gdrive/upload", method: Method::POST, description: "Upload to GDrive", requires_auth: true },
        EndpointConfig { path: "/gdrive/proxy", method: Method::GET, description: "GDrive proxy stream", requires_auth: true },
        // GCloud
        EndpointConfig { path: "/gcloud/instances", method: Method::GET, description: "List GCloud instances", requires_auth: true },
        // GCP
        EndpointConfig { path: "/gcp/browse", method: Method::GET, description: "Browse GCP VM files", requires_auth: true },
        EndpointConfig { path: "/gcp/download", method: Method::GET, description: "Download from GCP VM", requires_auth: true },
        EndpointConfig { path: "/gcp/list", method: Method::GET, description: "List GCP VM files", requires_auth: true },
        EndpointConfig { path: "/gcp/play", method: Method::POST, description: "Play media from GCP VM", requires_auth: true },
        EndpointConfig { path: "/gcp/push", method: Method::POST, description: "Push file to GCP VM", requires_auth: true },
        // AWS
        EndpointConfig { path: "/aws/ec2", method: Method::GET, description: "List EC2 instances", requires_auth: true },
        EndpointConfig { path: "/aws/ec2/start", method: Method::POST, description: "Start EC2 instance", requires_auth: true },
        EndpointConfig { path: "/aws/ec2/stop", method: Method::POST, description: "Stop EC2 instance", requires_auth: true },
        EndpointConfig { path: "/aws/sg", method: Method::GET, description: "List AWS security groups", requires_auth: true },
        EndpointConfig { path: "/aws/regions", method: Method::GET, description: "List AWS regions", requires_auth: true },
        EndpointConfig { path: "/aws/zones", method: Method::GET, description: "List AWS availability zones", requires_auth: true },
        // Tailscale
        EndpointConfig { path: "/tailscale/devices", method: Method::GET, description: "List Tailscale devices", requires_auth: true },
        EndpointConfig { path: "/tailscale/acl", method: Method::GET, description: "Get Tailscale ACL policy", requires_auth: true },
        EndpointConfig { path: "/tailscale/routes", method: Method::GET, description: "List Tailscale routes", requires_auth: true },
        EndpointConfig { path: "/tailscale/keys", method: Method::GET, description: "List Tailscale auth keys", requires_auth: true },
        // WordPress
        EndpointConfig { path: "/wordpress", method: Method::GET, description: "WordPress operations", requires_auth: true },
        // Cache
        EndpointConfig { path: "/cache", method: Method::GET, description: "View cache", requires_auth: true },
        EndpointConfig { path: "/cache", method: Method::DELETE, description: "Clear cache", requires_auth: true },
        // Accounts
        EndpointConfig { path: "/accounts", method: Method::GET, description: "List accounts", requires_auth: true },
        // Instance
        EndpointConfig { path: "/instance-create", method: Method::POST, description: "Create cloud instance", requires_auth: true },
        // Kodi
        EndpointConfig { path: "/kodi-repo", method: Method::GET, description: "Kodi repository", requires_auth: true },
        // Media
        EndpointConfig { path: "/play", method: Method::POST, description: "Media playback", requires_auth: true },
    ]
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum HTTP header size in bytes (1 MB).
pub const MAX_HEADER_SIZE: usize = 1024 * 1024;

/// TCP socket buffer size (1 MB).
pub const SOCKET_BUFFER_SIZE: usize = 1024 * 1024;

/// Read timeout in seconds.
pub const READ_TIMEOUT_SECS: u64 = 60;

/// Idle timeout in seconds (5 minutes).
pub const IDLE_TIMEOUT_SECS: u64 = 300;

/// Graceful shutdown timeout in seconds.
pub const SHUTDOWN_TIMEOUT_SECS: u64 = 5;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let cfg = ServerConfig::default();
        assert!(!cfg.tls);
        assert_eq!(cfg.port, 8080);
        assert_eq!(cfg.host, "0.0.0.0");
        cfg.validate().expect("default config should be valid");
    }

    #[test]
    fn tls_without_cert_fails_validation() {
        let cfg = ServerConfig {
            tls: true,
            cert_file: None,
            key_file: Some("/tmp/key.pem".into()),
            ..Default::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn tls_without_key_fails_validation() {
        let cfg = ServerConfig {
            tls: true,
            cert_file: Some("/tmp/cert.pem".into()),
            key_file: None,
            ..Default::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn tls_with_both_files_passes_validation() {
        let cfg = ServerConfig {
            tls: true,
            cert_file: Some("/tmp/cert.pem".into()),
            key_file: Some("/tmp/key.pem".into()),
            ..Default::default()
        };
        cfg.validate().expect("should pass with both files");
    }

    #[test]
    fn addr_format() {
        let cfg = ServerConfig {
            host: "127.0.0.1".into(),
            port: 3000,
            ..Default::default()
        };
        assert_eq!(cfg.addr(), "127.0.0.1:3000");
    }

    #[test]
    fn endpoint_configurations_has_at_least_30() {
        let endpoints = endpoint_configurations();
        assert!(
            endpoints.len() >= 30,
            "Expected at least 30 endpoints, got {}",
            endpoints.len()
        );
    }

    #[test]
    fn health_endpoint_does_not_require_auth() {
        let endpoints = endpoint_configurations();
        let health = endpoints.iter().find(|e| e.path == "/health").unwrap();
        assert!(!health.requires_auth);
    }

    #[test]
    fn protected_endpoints_require_auth() {
        let endpoints = endpoint_configurations();
        let protected = endpoints
            .iter()
            .find(|e| e.path == "/append-links")
            .unwrap();
        assert!(protected.requires_auth);
    }
}
