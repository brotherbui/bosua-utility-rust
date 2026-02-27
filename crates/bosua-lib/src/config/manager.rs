use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::RwLock;

use super::dynamic::DynamicConfig;
use crate::errors::{BosuaError, Result};

/// Thread-safe manager for `DynamicConfig` with file persistence and change callbacks.
///
/// Loads configuration from `~/.bosua/config.json` (or a custom directory),
/// persists every mutation to disk, and notifies registered listeners on change.
pub struct DynamicConfigManager {
    config: Arc<RwLock<DynamicConfig>>,
    config_path: PathBuf,
    on_change: Arc<RwLock<Vec<Box<dyn Fn(&DynamicConfig) + Send + Sync>>>>,
}

impl DynamicConfigManager {
    /// Initialize the config manager.
    ///
    /// * If `config_dir` is `Some`, uses that directory for `config.json`.
    /// * Otherwise defaults to `~/.bosua/`.
    /// * Creates the file with defaults when missing.
    /// * Falls back to defaults on any read/parse error (logs a warning).
    pub async fn initialize(config_dir: Option<PathBuf>) -> Result<Self> {
        let dir = match config_dir {
            Some(d) => d,
            None => {
                let home = std::env::var("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("/tmp"));
                home.join(".bosua")
            }
        };

        let config_path = dir.join("config.json");
        let config = Self::load_or_create_config(&config_path).await;

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
            on_change: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Return the absolute path to the configuration file.
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    /// Return a clone of the current configuration.
    pub async fn get_config(&self) -> DynamicConfig {
        self.config.read().await.clone()
    }

    /// Apply a partial update from a JSON map of key-value pairs.
    ///
    /// Only the keys present in `updates` are changed; all other fields keep
    /// their current values. The result is persisted to disk and all registered
    /// callbacks are notified.
    pub async fn update_config(&self, updates: serde_json::Map<String, Value>) -> Result<()> {
        let mut config = self.config.write().await;

        // Serialize current config to a JSON map, merge updates, deserialize back.
        let mut current_value = serde_json::to_value(&*config)
            .map_err(|e| BosuaError::Config(format!("Failed to serialize config: {e}")))?;

        if let Some(obj) = current_value.as_object_mut() {
            for (key, value) in updates {
                obj.insert(key, value);
            }
        }

        *config = serde_json::from_value(current_value)
            .map_err(|e| BosuaError::Config(format!("Failed to apply config updates: {e}")))?;

        self.persist(&config).await?;
        self.notify_change(&config).await;

        Ok(())
    }

    /// Reset all fields to their default values, persist, and notify.
    pub async fn reset_to_defaults(&self) -> Result<()> {
        let mut config = self.config.write().await;
        *config = DynamicConfig::default();

        self.persist(&config).await?;
        self.notify_change(&config).await;

        Ok(())
    }

    /// Register a callback that fires on every config change.
    pub async fn register_on_change(
        &self,
        callback: impl Fn(&DynamicConfig) + Send + Sync + 'static,
    ) {
        self.on_change.write().await.push(Box::new(callback));
    }

    // ── private helpers ──────────────────────────────────────────────

    /// Try to load config from disk; create with defaults if missing; fall back
    /// to defaults on any error.
    async fn load_or_create_config(path: &PathBuf) -> DynamicConfig {
        if path.exists() {
            match tokio::fs::read_to_string(path).await {
                Ok(contents) => match serde_json::from_str::<DynamicConfig>(&contents) {
                    Ok(cfg) => return cfg,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to parse config file {}: {}. Using defaults.",
                            path.display(),
                            e
                        );
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        "Failed to read config file {}: {}. Using defaults.",
                        path.display(),
                        e
                    );
                }
            }
        }

        // File doesn't exist or was unreadable/corrupt — create with defaults.
        let defaults = DynamicConfig::default();
        if let Err(e) = Self::write_config(path, &defaults).await {
            tracing::warn!(
                "Failed to create default config file {}: {}",
                path.display(),
                e
            );
        }
        defaults
    }

    /// Persist the given config to disk.
    async fn persist(&self, config: &DynamicConfig) -> Result<()> {
        Self::write_config(&self.config_path, config).await
    }

    /// Write config JSON to the given path, creating parent directories as needed.
    async fn write_config(path: &PathBuf, config: &DynamicConfig) -> Result<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| BosuaError::Config(format!("Failed to create config dir: {e}")))?;
        }

        let json = serde_json::to_string_pretty(config)
            .map_err(|e| BosuaError::Config(format!("Failed to serialize config: {e}")))?;

        tokio::fs::write(path, json)
            .await
            .map_err(|e| BosuaError::Config(format!("Failed to write config file: {e}")))?;

        Ok(())
    }

    /// Invoke all registered on-change callbacks with the new config.
    async fn notify_change(&self, config: &DynamicConfig) {
        let callbacks = self.on_change.read().await;
        for cb in callbacks.iter() {
            cb(config);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    async fn setup() -> (tempfile::TempDir, DynamicConfigManager) {
        let tmp = tempfile::TempDir::new().unwrap();
        let mgr = DynamicConfigManager::initialize(Some(tmp.path().to_path_buf()))
            .await
            .unwrap();
        (tmp, mgr)
    }

    #[tokio::test]
    async fn test_initialize_creates_default_config_file() {
        let (tmp, _mgr): (tempfile::TempDir, DynamicConfigManager) = setup().await;
        let path = tmp.path().join("config.json");
        assert!(path.exists());

        let contents = std::fs::read_to_string(&path).unwrap();
        let loaded: DynamicConfig = serde_json::from_str(&contents).unwrap();
        assert_eq!(loaded, DynamicConfig::default());
    }

    #[tokio::test]
    async fn test_initialize_loads_existing_config() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("config.json");

        let mut custom = DynamicConfig::default();
        custom.max_retries = 42;
        let json = serde_json::to_string_pretty(&custom).unwrap();
        std::fs::write(&path, &json).unwrap();

        let mgr = DynamicConfigManager::initialize(Some(tmp.path().to_path_buf()))
            .await
            .unwrap();
        let cfg = mgr.get_config().await;
        assert_eq!(cfg.max_retries, 42);
    }

    #[tokio::test]
    async fn test_initialize_falls_back_on_corrupt_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("config.json");
        std::fs::write(&path, "not valid json!!!").unwrap();

        let mgr = DynamicConfigManager::initialize(Some(tmp.path().to_path_buf()))
            .await
            .unwrap();
        let cfg = mgr.get_config().await;
        assert_eq!(cfg, DynamicConfig::default());
    }

    #[tokio::test]
    async fn test_get_config_returns_clone() {
        let (_tmp, mgr): (tempfile::TempDir, DynamicConfigManager) = setup().await;
        let a = mgr.get_config().await;
        let b = mgr.get_config().await;
        assert_eq!(a, b);
    }

    #[tokio::test]
    async fn test_update_config_partial() {
        let (_tmp, mgr): (tempfile::TempDir, DynamicConfigManager) = setup().await;

        let mut updates = serde_json::Map::new();
        updates.insert("maxRetries".into(), Value::Number(10.into()));
        updates.insert(
            "awsRegion".into(),
            Value::String("us-west-2".into()),
        );
        mgr.update_config(updates).await.unwrap();

        let cfg = mgr.get_config().await;
        assert_eq!(cfg.max_retries, 10);
        assert_eq!(cfg.aws_region, "us-west-2");
        // Unchanged fields stay at defaults
        assert_eq!(cfg.timeout, 30);
        assert_eq!(cfg.kodi_username, "kodi");
    }

    #[tokio::test]
    async fn test_update_config_persists_to_disk() {
        let (tmp, mgr): (tempfile::TempDir, DynamicConfigManager) = setup().await;

        let mut updates = serde_json::Map::new();
        updates.insert("timeout".into(), Value::Number(99.into()));
        mgr.update_config(updates).await.unwrap();

        let path = tmp.path().join("config.json");
        let contents = std::fs::read_to_string(&path).unwrap();
        let loaded: DynamicConfig = serde_json::from_str(&contents).unwrap();
        assert_eq!(loaded.timeout, 99);
    }

    #[tokio::test]
    async fn test_reset_to_defaults() {
        let (_tmp, mgr): (tempfile::TempDir, DynamicConfigManager) = setup().await;

        // Modify first
        let mut updates = serde_json::Map::new();
        updates.insert("maxRetries".into(), Value::Number(99.into()));
        mgr.update_config(updates).await.unwrap();
        assert_eq!(mgr.get_config().await.max_retries, 99);

        // Reset
        mgr.reset_to_defaults().await.unwrap();
        assert_eq!(mgr.get_config().await, DynamicConfig::default());
    }

    #[tokio::test]
    async fn test_reset_to_defaults_is_idempotent() {
        let (_tmp, mgr): (tempfile::TempDir, DynamicConfigManager) = setup().await;
        mgr.reset_to_defaults().await.unwrap();
        let a = mgr.get_config().await;
        mgr.reset_to_defaults().await.unwrap();
        let b = mgr.get_config().await;
        assert_eq!(a, b);
    }

    #[tokio::test]
    async fn test_on_change_callback_fires() {
        let (_tmp, mgr): (tempfile::TempDir, DynamicConfigManager) = setup().await;

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        mgr.register_on_change(move |_cfg| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        })
        .await;

        let mut updates = serde_json::Map::new();
        updates.insert("timeout".into(), Value::Number(1.into()));
        mgr.update_config(updates).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        mgr.reset_to_defaults().await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_multiple_on_change_callbacks() {
        let (_tmp, mgr): (tempfile::TempDir, DynamicConfigManager) = setup().await;

        let c1 = Arc::new(AtomicU32::new(0));
        let c2 = Arc::new(AtomicU32::new(0));

        let c1_clone = c1.clone();
        mgr.register_on_change(move |_| {
            c1_clone.fetch_add(1, Ordering::SeqCst);
        })
        .await;

        let c2_clone = c2.clone();
        mgr.register_on_change(move |_| {
            c2_clone.fetch_add(1, Ordering::SeqCst);
        })
        .await;

        let mut updates = serde_json::Map::new();
        updates.insert("timeout".into(), Value::Number(5.into()));
        mgr.update_config(updates).await.unwrap();

        assert_eq!(c1.load(Ordering::SeqCst), 1);
        assert_eq!(c2.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_update_with_invalid_value_type_fails() {
        let (_tmp, mgr): (tempfile::TempDir, DynamicConfigManager) = setup().await;

        let mut updates = serde_json::Map::new();
        // "maxRetries" expects a number, not a string
        updates.insert("maxRetries".into(), Value::String("not_a_number".into()));
        let result: std::result::Result<(), BosuaError> = mgr.update_config(updates).await;
        assert!(result.is_err());
    }
}
