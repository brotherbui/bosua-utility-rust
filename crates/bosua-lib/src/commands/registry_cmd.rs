//! Registry CLI command — list registered commands (text and JSON).
//!
//! Named `registry_cmd` to avoid conflict with the `cli::registry` module.
//!
//! Also contains `ServiceRegistry`, the central holder for lazily-initialized
//! service instances shared across command handlers.

use std::path::PathBuf;
use std::sync::Arc;

use clap::{ArgMatches, Command};
use tokio::sync::OnceCell;

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::aws::AwsClient;
use crate::cloud::cloudflare::CloudflareClient;
use crate::cloud::fshare::FShareClient;
use crate::cloud::gdrive::GDriveClient;
use crate::cloud::tailscale::TailscaleClient;
use crate::config::manager::DynamicConfigManager;
use crate::daemon::cron::CronManager;
use crate::daemon::DaemonManager;
use crate::download::aria2::Aria2Client;
use crate::download::DownloadManager;
use crate::errors::Result;
use crate::http_client::HttpClient;
use crate::search::SearchEngine;

// ---------------------------------------------------------------------------
// ServiceRegistry
// ---------------------------------------------------------------------------

/// Central holder for lazily-initialized service instances.
///
/// Created once at startup and passed to the command dispatch layer.
/// Cloud clients are initialized on first use via `OnceCell` to avoid
/// unnecessary authentication prompts at startup (Requirement 20.4).
pub struct ServiceRegistry {
    pub config_manager: Arc<DynamicConfigManager>,
    pub http_client: HttpClient,
    gdrive: OnceCell<Arc<GDriveClient>>,
    fshare: OnceCell<Arc<FShareClient>>,
    aws: OnceCell<Arc<AwsClient>>,
    cloudflare: OnceCell<Arc<CloudflareClient>>,
    tailscale: OnceCell<Arc<TailscaleClient>>,
    download_manager: OnceCell<Arc<DownloadManager>>,
    search_engine: OnceCell<Arc<SearchEngine>>,
    daemon_manager: OnceCell<Arc<DaemonManager>>,
    cron_manager: OnceCell<Arc<tokio::sync::Mutex<CronManager>>>,
    aria2: OnceCell<Arc<Aria2Client>>,
}

/// Resolve the `.bosua` directory under the user's home.
fn bosua_home() -> PathBuf {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    home.join(".bosua")
}

impl ServiceRegistry {
    /// Create a new `ServiceRegistry`. No clients are initialized yet.
    pub fn new(config_manager: Arc<DynamicConfigManager>, http_client: HttpClient) -> Self {
        Self {
            config_manager,
            http_client,
            gdrive: OnceCell::new(),
            fshare: OnceCell::new(),
            aws: OnceCell::new(),
            cloudflare: OnceCell::new(),
            tailscale: OnceCell::new(),
            download_manager: OnceCell::new(),
            search_engine: OnceCell::new(),
            daemon_manager: OnceCell::new(),
            cron_manager: OnceCell::new(),
            aria2: OnceCell::new(),
        }
    }

    /// Register config-change listeners for eagerly-available services.
    ///
    /// Call once after construction. Wires `DynamicConfigManager::register_on_change`
    /// so that the shared `HttpClient` rebuilds itself when timeout / pool
    /// settings change (Requirement 20.3).
    ///
    /// Lazily-initialized clients (GDrive, AWS, …) register their own
    /// listeners at init time — see e.g. [`Self::gdrive`].
    pub async fn register_config_listeners(&self) {
        let http = self.http_client.clone();
        self.config_manager
            .register_on_change(move |cfg| {
                let http = http.clone();
                let cfg = cfg.clone();
                tokio::spawn(async move {
                    if let Err(e) = http.update_from_config(&cfg).await {
                        tracing::warn!("Failed to update HttpClient from config: {e}");
                    }
                });
            })
            .await;
    }

    /// Lazily initialize and return the Google Drive client.
    ///
    /// On first initialization, registers a config-change listener so that
    /// updates to `gdrive_default_account` in `DynamicConfig` are
    /// automatically propagated to the client (Requirement 20.3).
    pub async fn gdrive(&self) -> Result<&Arc<GDriveClient>> {
        self.gdrive
            .get_or_try_init(|| async {
                let config = self.config_manager.get_config().await;
                let base = bosua_home();
                let client = Arc::new(GDriveClient::new(
                    self.http_client.clone(),
                    base.join("gdrive-token.json"),
                    base.join("gdrive.lock"),
                    base.join("gdrive-retry.lock"),
                    &config,
                    String::new(), // client_id — empty until configured
                    String::new(), // client_secret — empty until configured
                ));

                // Wire config change notification → GDriveClient default account.
                let gdrive_weak = Arc::downgrade(&client);
                self.config_manager
                    .register_on_change(move |cfg| {
                        let new_account = cfg.gdrive_default_account.clone();
                        if let Some(gdrive) = gdrive_weak.upgrade() {
                            tokio::spawn(async move {
                                gdrive.update_default_account(&new_account).await;
                            });
                        }
                    })
                    .await;

                Ok(client)
            })
            .await
    }

    /// Lazily initialize and return the FShare client.
    ///
    /// Uses the shared FShare app key and loads any saved token from
    /// `~/.config/fshare/fshare_token.txt` so the Rust and Go CLIs
    /// share authentication state.
    ///
    /// Credentials are decoded from obfuscated strings matching Go's
    /// `GetAccount()`, so the client can auto-re-login when the token expires.
    pub async fn fshare(&self) -> Result<&Arc<FShareClient>> {
        self.fshare
            .get_or_try_init(|| async {
                let (email, password) = crate::cloud::fshare::get_fshare_account();
                let client = FShareClient::new(
                    self.http_client.clone(),
                    email,
                    password,
                    crate::cloud::fshare::FSHARE_APP_KEY.to_string(),
                );
                // Load saved token from file (shared with Go binary)
                let _ = client.load_token_from_file().await;
                Ok(Arc::new(client))
            })
            .await
    }

    /// Lazily initialize and return the AWS client.
    pub async fn aws(&self) -> Result<&Arc<AwsClient>> {
        self.aws
            .get_or_try_init(|| async {
                let config = self.config_manager.get_config().await;
                let client = AwsClient::new(&config);
                Ok(Arc::new(client))
            })
            .await
    }

    /// Lazily initialize and return the Cloudflare client.
    pub async fn cloudflare(&self) -> Result<&Arc<CloudflareClient>> {
        self.cloudflare
            .get_or_try_init(|| async {
                let client = CloudflareClient::new(
                    self.http_client.clone(),
                    String::new(), // api_token — empty until configured
                    None,          // zone_id
                );
                Ok(Arc::new(client))
            })
            .await
    }

    /// Lazily initialize and return the Tailscale client.
    pub async fn tailscale(&self) -> Result<&Arc<TailscaleClient>> {
        self.tailscale
            .get_or_try_init(|| async {
                let client = TailscaleClient::new(
                    self.http_client.clone(),
                    String::new(), // api_key
                    String::new(), // tailnet
                );
                Ok(Arc::new(client))
            })
            .await
    }

    /// Lazily initialize and return the download manager.
    pub async fn download_manager(&self) -> Result<&Arc<DownloadManager>> {
        self.download_manager
            .get_or_try_init(|| async {
                let base = bosua_home();
                let dm = DownloadManager::new(
                    self.http_client.clone(),
                    Arc::clone(&self.config_manager),
                    base.join("download.lock"),
                    base.join("downloads"),
                    base.join("links.txt"),
                );
                Ok(Arc::new(dm))
            })
            .await
    }

    /// Lazily initialize and return the search engine.
    pub async fn search_engine(&self) -> Result<&Arc<SearchEngine>> {
        self.search_engine
            .get_or_try_init(|| async {
                let db_path = bosua_home().join("search.db");
                let engine = SearchEngine::new(db_path)?;
                Ok(Arc::new(engine))
            })
            .await
    }

    /// Lazily initialize and return the daemon manager.
    pub async fn daemon_manager(&self) -> &Arc<DaemonManager> {
        self.daemon_manager
            .get_or_init(|| async { Arc::new(DaemonManager::with_defaults()) })
            .await
    }

    /// Lazily initialize and return the cron manager.
    pub async fn cron_manager(&self) -> &Arc<tokio::sync::Mutex<CronManager>> {
        self.cron_manager
            .get_or_init(|| async {
                Arc::new(tokio::sync::Mutex::new(CronManager::new()))
            })
            .await
    }

    /// Lazily initialize and return the Aria2 RPC client.
    pub async fn aria2(&self) -> Result<&Arc<Aria2Client>> {
        self.aria2
            .get_or_try_init(|| async {
                let client = Aria2Client::new(
                    self.http_client.clone(),
                    None, // default endpoint
                    None, // no token
                );
                Ok(Arc::new(client))
            })
            .await
    }
}

/// Build the `registry` clap command.
pub fn registry_command() -> Command {
    Command::new("registry")
        .about("Manage and view the command registry system")
        .aliases(["reg", "commands"])
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("list").about("List all registered commands by category"))
        .subcommand(Command::new("stats").about("Show command registry statistics"))
        .subcommand(Command::new("validate").about("Validate command registry for issues"))
}

/// Build the `CommandMeta` for registry registration.
pub fn registry_meta() -> CommandMeta {
    CommandBuilder::from_clap(registry_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `registry` command.
pub fn handle_registry(matches: &ArgMatches) {
    match matches.subcommand() {
        Some(("list", _)) => {
            // Use the Rust CommandRegistry to list commands
            use crate::cli::CommandRegistry;
            let root = Command::new("bosua");
            let mut registry = CommandRegistry::new(root);
            #[cfg(feature = "macos")]
            crate::commands::register_macos_commands(&mut registry);
            #[cfg(not(feature = "macos"))]
            crate::commands::register_linux_commands(&mut registry);

            registry.list_commands();
        }
        Some(("stats", _)) => {
            use crate::cli::CommandRegistry;
            let root = Command::new("bosua");
            let mut registry = CommandRegistry::new(root);
            #[cfg(feature = "macos")]
            crate::commands::register_macos_commands(&mut registry);
            #[cfg(not(feature = "macos"))]
            crate::commands::register_linux_commands(&mut registry);

            let stats = registry.stats();
            println!("Command Registry Statistics:");
            println!("{}", "=".repeat(40));
            println!("  {:<20} {}", "Total commands", stats.total);
            for (category, count) in &stats.per_category {
                println!("  {:<20} {}", format!("{}", category), count);
            }
        }
        Some(("validate", _)) => {
            use crate::cli::CommandRegistry;
            let root = Command::new("bosua");
            let mut registry = CommandRegistry::new(root);
            #[cfg(feature = "macos")]
            crate::commands::register_macos_commands(&mut registry);
            #[cfg(not(feature = "macos"))]
            crate::commands::register_linux_commands(&mut registry);

            let issues = registry.validate();
            if issues.is_empty() {
                println!("Registry validation passed. {} commands, no issues found.", registry.len());
            } else {
                println!("Registry validation found {} issues:", issues.len());
                for issue in &issues {
                    println!("  - {}", issue);
                }
            }
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_command_parses() {
        let cmd = registry_command();
        let m = cmd.try_get_matches_from(["registry", "list"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("list"));
    }

    #[test]
    fn test_registry_command_stats() {
        let cmd = registry_command();
        let m = cmd.try_get_matches_from(["registry", "stats"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("stats"));
    }

    #[test]
    fn test_registry_command_validate() {
        let cmd = registry_command();
        let m = cmd.try_get_matches_from(["registry", "validate"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("validate"));
    }

    #[test]
    fn test_registry_alias_reg() {
        let cmd = registry_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"reg"));
        assert!(aliases.contains(&"commands"));
    }

    #[test]
    fn test_registry_requires_subcommand() {
        let cmd = registry_command();
        assert!(cmd.try_get_matches_from(["registry"]).is_err());
    }

    #[test]
    fn test_registry_meta() {
        let meta = registry_meta();
        assert_eq!(meta.name, "registry");
        assert_eq!(meta.category, CommandCategory::Utility);
    }

    #[tokio::test]
    async fn test_gdrive_config_change_updates_default_account() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_manager = Arc::new(
            DynamicConfigManager::initialize(Some(tmp.path().to_path_buf()))
                .await
                .unwrap(),
        );
        let http_client = HttpClient::from_defaults().unwrap();
        let services = ServiceRegistry::new(config_manager.clone(), http_client);

        // Initialize GDrive client (this registers the on_change listener)
        let gdrive = services.gdrive().await.unwrap();
        assert_eq!(gdrive.default_account().await, "");

        // Update config with a new gdrive_default_account
        let mut updates = serde_json::Map::new();
        updates.insert(
            "gdriveDefaultAccount".into(),
            serde_json::Value::String("user@example.com".into()),
        );
        config_manager.update_config(updates).await.unwrap();

        // Give the spawned task a moment to complete
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert_eq!(gdrive.default_account().await, "user@example.com");
    }

    #[tokio::test]
    async fn test_http_client_config_change_listener_registered() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_manager = Arc::new(
            DynamicConfigManager::initialize(Some(tmp.path().to_path_buf()))
                .await
                .unwrap(),
        );
        let http_client = HttpClient::from_defaults().unwrap();
        let services = ServiceRegistry::new(config_manager.clone(), http_client);
        services.register_config_listeners().await;

        // Update config — the HttpClient listener should not panic
        let mut updates = serde_json::Map::new();
        updates.insert("timeout".into(), serde_json::Value::Number(60.into()));
        config_manager.update_config(updates).await.unwrap();

        // Give the spawned task a moment to complete
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}
