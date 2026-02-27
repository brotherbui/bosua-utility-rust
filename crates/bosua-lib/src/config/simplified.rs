use std::path::PathBuf;
use std::sync::OnceLock;

/// Immutable application configuration initialized once at startup from environment variables.
///
/// Access via `SimplifiedConfig::get()` which returns a `&'static SimplifiedConfig`.
/// The singleton is lazily initialized on first access using `OnceLock`.
pub struct SimplifiedConfig {
    pub temp_dir: PathBuf,
    pub file_dir: PathBuf,
    pub home_dir: PathBuf,
    pub download_dir: PathBuf,
    pub homebrew_prefix: Option<String>,
    pub pwd: PathBuf,
    pub path: String,
    pub server_ip: String,
    pub server_domain: String,
    pub gcp_ip: String,
    pub gcp_domain: String,
    pub input_links_file: PathBuf,
    pub download_lock_file: PathBuf,
    pub gdrive_lock_file: PathBuf,
    pub gdrive_retry_lock_file: PathBuf,
    pub token_file: PathBuf,
    pub sheets_cache_file: PathBuf,
}

static CONFIG: OnceLock<SimplifiedConfig> = OnceLock::new();

impl SimplifiedConfig {
    /// Returns a reference to the global `SimplifiedConfig` singleton.
    /// Initializes from environment variables on first call.
    pub fn get() -> &'static SimplifiedConfig {
        CONFIG.get_or_init(|| SimplifiedConfig::from_env())
    }

    fn from_env() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let home_dir = PathBuf::from(&home);
        let temp_dir = std::env::temp_dir();

        Self {
            download_dir: home_dir.join("Downloads"),
            input_links_file: home_dir.join("Downloads/links.txt"),
            file_dir: std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from(".")),
            download_lock_file: temp_dir.join("download.lock"),
            gdrive_lock_file: temp_dir.join("gdrive.lock"),
            gdrive_retry_lock_file: temp_dir.join("gdrive_retry.lock"),
            token_file: temp_dir.join("token.txt"),
            sheets_cache_file: temp_dir.join("bosua_sheets_cache.db"),
            homebrew_prefix: std::env::var("HOMEBREW_PREFIX").ok(),
            pwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            path: std::env::var("PATH").unwrap_or_default(),
            server_ip: std::env::var("BACKEND_IP").unwrap_or_default(),
            server_domain: std::env::var("BACKEND_DOMAIN").unwrap_or_default(),
            gcp_ip: std::env::var("GCP_IP").unwrap_or_default(),
            gcp_domain: std::env::var("GCP_DOMAIN").unwrap_or_default(),
            temp_dir,
            home_dir,
        }
    }
}
