//! Shared HTTP client with connection pooling and DynamicConfig integration.
//!
//! Wraps `reqwest::Client` and rebuilds it when configuration changes.
//! The client is safe to clone (internally `Arc`-ed) and can be shared
//! across tasks.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use crate::config::dynamic::DynamicConfig;
use crate::errors::{BosuaError, Result};

/// A shared HTTP client that rebuilds itself when `DynamicConfig` changes.
///
/// Internally holds an `Arc<RwLock<reqwest::Client>>` so clones are cheap
/// and reads are non-blocking when no rebuild is in progress.
#[derive(Clone)]
pub struct HttpClient {
    inner: Arc<RwLock<reqwest::Client>>,
}

impl HttpClient {
    /// Build a new `HttpClient` configured from the given `DynamicConfig`.
    pub fn new(config: &DynamicConfig) -> Result<Self> {
        let client = Self::build_client(config)?;
        Ok(Self {
            inner: Arc::new(RwLock::new(client)),
        })
    }

    /// Build an `HttpClient` using `DynamicConfig::default()`.
    pub fn from_defaults() -> Result<Self> {
        Self::new(&DynamicConfig::default())
    }

    /// Get a clone of the current `reqwest::Client`.
    ///
    /// `reqwest::Client` is internally `Arc`-ed, so cloning is cheap.
    pub async fn get_client(&self) -> reqwest::Client {
        self.inner.read().await.clone()
    }

    /// Rebuild the inner client from an updated `DynamicConfig`.
    ///
    /// This is intended to be called from a `DynamicConfigManager::register_on_change`
    /// callback whenever config values change.
    pub async fn update_from_config(&self, config: &DynamicConfig) -> Result<()> {
        let new_client = Self::build_client(config)?;
        *self.inner.write().await = new_client;
        Ok(())
    }

    /// Construct a `reqwest::Client` from the relevant `DynamicConfig` fields.
    fn build_client(config: &DynamicConfig) -> Result<reqwest::Client> {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout as u64))
            .connect_timeout(Duration::from_secs(config.tls_handshake_timeout as u64))
            .pool_max_idle_per_host(config.max_idle_conns_per_host as usize)
            .pool_idle_timeout(Duration::from_secs(config.idle_conn_timeout as u64))
            .build()
            .map_err(BosuaError::Http)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_from_defaults() {
        let client = HttpClient::from_defaults().expect("should build from defaults");
        let _inner = client.get_client().await;
    }

    #[tokio::test]
    async fn test_new_with_config() {
        let config = DynamicConfig::default();
        let client = HttpClient::new(&config).expect("should build from config");
        let _inner = client.get_client().await;
    }

    #[tokio::test]
    async fn test_update_from_config() {
        let client = HttpClient::from_defaults().expect("should build");

        let mut config = DynamicConfig::default();
        config.timeout = 60;
        config.max_idle_conns_per_host = 50;

        client
            .update_from_config(&config)
            .await
            .expect("should update");

        // Verify we can still get a working client after update
        let _inner = client.get_client().await;
    }

    #[tokio::test]
    async fn test_clone_shares_state() {
        let client = HttpClient::from_defaults().expect("should build");
        let cloned = client.clone();

        // Update via the original
        let mut config = DynamicConfig::default();
        config.timeout = 120;
        client
            .update_from_config(&config)
            .await
            .expect("should update");

        // The clone should see the updated client (same Arc)
        let _inner = cloned.get_client().await;
    }

    #[tokio::test]
    async fn test_custom_pool_settings() {
        let mut config = DynamicConfig::default();
        config.max_idle_conns_per_host = 10;
        config.idle_conn_timeout = 30;
        config.tls_handshake_timeout = 5;
        config.timeout = 15;

        let client = HttpClient::new(&config).expect("should build with custom settings");
        let _inner = client.get_client().await;
    }
}
