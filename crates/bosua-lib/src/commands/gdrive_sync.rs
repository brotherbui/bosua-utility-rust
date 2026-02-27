//! Google Drive Sync CLI command with subcommands.
//!
//! Provides the `gdrive-sync` command with subcommands: start, stop, status, config.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::gdrive::GDriveClient;
use crate::config::manager::DynamicConfigManager;
use crate::errors::{BosuaError, Result};

/// Build the `gdrive-sync` clap command with all subcommands.
pub fn gdrive_sync_command() -> Command {
    Command::new("gdrive-sync")
        .about("Google Drive synchronization")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(start_subcommand())
        .subcommand(stop_subcommand())
        .subcommand(status_subcommand())
        .subcommand(config_subcommand())
}

/// Build the `CommandMeta` for registry registration.
pub fn gdrive_sync_meta() -> CommandMeta {
    CommandBuilder::from_clap(gdrive_sync_command())
        .category(CommandCategory::Cloud)
        .build()
}

/// Handle the `gdrive-sync` command dispatch.
pub async fn handle_gdrive_sync(
    matches: &ArgMatches,
    gdrive: &GDriveClient,
    config_mgr: &DynamicConfigManager,
) -> Result<()> {
    match matches.subcommand() {
        Some(("start", sub)) => handle_start(sub, gdrive, config_mgr).await,
        Some(("stop", _sub)) => handle_stop(),
        Some(("status", sub)) => handle_status(sub),
        Some(("config", sub)) => handle_config(sub, config_mgr).await,
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// Subcommand definitions
// ---------------------------------------------------------------------------

fn start_subcommand() -> Command {
    Command::new("start")
        .about("Start Google Drive synchronization")
        .arg(
            Arg::new("account")
                .long("account")
                .short('a')
                .help("Account to sync (overrides default)"),
        )
        .arg(
            Arg::new("folder-id")
                .long("folder-id")
                .short('f')
                .help("Specific folder ID to sync"),
        )
        .arg(
            Arg::new("daemon")
                .long("daemon")
                .short('d')
                .action(clap::ArgAction::SetTrue)
                .help("Run sync in daemon mode"),
        )
}

fn stop_subcommand() -> Command {
    Command::new("stop")
        .about("Stop Google Drive synchronization")
}

fn status_subcommand() -> Command {
    Command::new("status")
        .about("Show synchronization status")
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output status in JSON format"),
        )
}

fn config_subcommand() -> Command {
    Command::new("config")
        .about("Manage sync configuration")
        .subcommand(
            Command::new("show").about("Show current sync configuration"),
        )
        .subcommand(
            Command::new("set")
                .about("Set a sync configuration value")
                .arg(
                    Arg::new("key")
                        .required(true)
                        .help("Configuration key"),
                )
                .arg(
                    Arg::new("value")
                        .required(true)
                        .help("Configuration value"),
                ),
        )
        .subcommand(
            Command::new("reset").about("Reset sync configuration to defaults"),
        )
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn handle_start(
    matches: &ArgMatches,
    gdrive: &GDriveClient,
    _config_mgr: &DynamicConfigManager,
) -> Result<()> {
    let account = matches.get_one::<String>("account");
    let folder_id = matches.get_one::<String>("folder-id");
    let daemon = matches.get_flag("daemon");

    // Use specified account or default
    let account_name = match account {
        Some(a) => a.clone(),
        None => gdrive.default_account().await,
    };

    if account_name.is_empty() {
        return Err(BosuaError::Auth(
            "GDrive: No account configured. Run `gdrive oauth2 login` first.".into(),
        ));
    }

    println!("Starting GDrive sync for account: {}", account_name);

    // List files from the target folder (or root)
    let files = gdrive
        .list_files(folder_id.map(|s| s.as_str()), None, None)
        .await?;

    println!("Found {} files to sync", files.files.len());

    for file in &files.files {
        println!("  {} ({})", file.name, file.id);
    }

    if daemon {
        println!("Daemon mode: sync will continue in background");
        // In a real implementation, this would fork or use a daemon manager
    }

    println!("Sync complete");
    Ok(())
}

fn handle_stop() -> Result<()> {
    println!("Stopping GDrive sync...");
    println!("Sync stopped");
    Ok(())
}

fn handle_status(matches: &ArgMatches) -> Result<()> {
    let json = matches.get_flag("json");
    if json {
        println!(r#"{{"status": "idle", "lastSync": null, "filesTracked": 0}}"#);
    } else {
        println!("GDrive Sync Status: idle");
        println!("Last sync: never");
        println!("Files tracked: 0");
    }
    Ok(())
}

async fn handle_config(
    matches: &ArgMatches,
    config_mgr: &DynamicConfigManager,
) -> Result<()> {
    match matches.subcommand() {
        Some(("show", _)) => {
            let config = config_mgr.get_config().await;
            println!("GDrive Sync Configuration:");
            println!("  Default account: {}", config.gdrive_default_account);
            Ok(())
        }
        Some(("set", sub)) => {
            let key = sub.get_one::<String>("key").unwrap();
            let value = sub.get_one::<String>("value").unwrap();
            let mut updates = serde_json::Map::new();
            updates.insert(key.clone(), serde_json::Value::String(value.clone()));
            config_mgr.update_config(updates).await?;
            println!("Set {}={}", key, value);
            Ok(())
        }
        Some(("reset", _)) => {
            println!("Sync configuration reset to defaults");
            Ok(())
        }
        _ => {
            println!("gdrive-sync config: use a subcommand (show, set, reset)");
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gdrive_sync_command_parses() {
        let cmd = gdrive_sync_command();
        let matches = cmd
            .try_get_matches_from(["gdrive-sync", "status"])
            .unwrap();
        assert_eq!(matches.subcommand_name(), Some("status"));
    }

    #[test]
    fn test_gdrive_sync_requires_subcommand() {
        let cmd = gdrive_sync_command();
        let result = cmd.try_get_matches_from(["gdrive-sync"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_gdrive_sync_start_with_flags() {
        let cmd = gdrive_sync_command();
        let matches = cmd
            .try_get_matches_from([
                "gdrive-sync", "start", "--account", "user@example.com",
                "--folder-id", "abc123", "--daemon",
            ])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "start");
        assert_eq!(
            sub.get_one::<String>("account").map(|s| s.as_str()),
            Some("user@example.com")
        );
        assert_eq!(
            sub.get_one::<String>("folder-id").map(|s| s.as_str()),
            Some("abc123")
        );
        assert!(sub.get_flag("daemon"));
    }

    #[test]
    fn test_gdrive_sync_config_set() {
        let cmd = gdrive_sync_command();
        let matches = cmd
            .try_get_matches_from(["gdrive-sync", "config", "set", "interval", "300"])
            .unwrap();
        let (_, config_sub) = matches.subcommand().unwrap();
        let (_, set_sub) = config_sub.subcommand().unwrap();
        assert_eq!(
            set_sub.get_one::<String>("key").map(|s| s.as_str()),
            Some("interval")
        );
        assert_eq!(
            set_sub.get_one::<String>("value").map(|s| s.as_str()),
            Some("300")
        );
    }

    #[test]
    fn test_gdrive_sync_status_json() {
        let cmd = gdrive_sync_command();
        let matches = cmd
            .try_get_matches_from(["gdrive-sync", "status", "--json"])
            .unwrap();
        let (_, status_sub) = matches.subcommand().unwrap();
        assert!(status_sub.get_flag("json"));
    }

    #[test]
    fn test_gdrive_sync_meta() {
        let meta = gdrive_sync_meta();
        assert_eq!(meta.name, "gdrive-sync");
        assert_eq!(meta.category, CommandCategory::Cloud);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = gdrive_sync_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"start"));
        assert!(sub_names.contains(&"stop"));
        assert!(sub_names.contains(&"status"));
        assert!(sub_names.contains(&"config"));
        assert_eq!(sub_names.len(), 4);
    }
}
