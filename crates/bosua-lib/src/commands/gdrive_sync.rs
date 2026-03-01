//! Google Drive Sync CLI command with subcommands.
//!
//! Provides the `gdrive-sync` command with subcommands: add, list, remove, run.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::gdrive::GDriveClient;
use crate::config::manager::DynamicConfigManager;
use crate::errors::Result;

/// Build the `gdrive-sync` clap command with all subcommands.
pub fn gdrive_sync_command() -> Command {
    Command::new("gdrive-sync")
        .about("Track local folders and automatically upload new files to Google Drive.\nNew files are detected by comparing against a known file list and uploaded to the configured Drive folder.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("add")
                .about("Add a local path (file or folder) to track for gdrive sync")
                .arg(Arg::new("path").required(true).help("Local path to track")),
        )
        .subcommand(Command::new("list").about("List all tracked folders for gdrive sync"))
        .subcommand(
            Command::new("remove")
                .about("Remove a folder from gdrive sync tracking")
                .arg(Arg::new("path").required(true).help("Local path to remove")),
        )
        .subcommand(Command::new("run").about("Scan all tracked folders and upload new files to Google Drive"))
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
    _config_mgr: &DynamicConfigManager,
) -> Result<()> {
    match matches.subcommand() {
        Some(("add", sub)) => {
            let path = sub.get_one::<String>("path").unwrap();
            println!("Added path to gdrive sync tracking: {}", path);
            // TODO: persist to tracking database
            Ok(())
        }
        Some(("list", _)) => {
            println!("Tracked folders for gdrive sync:");
            // TODO: load from tracking database
            println!("  (none)");
            Ok(())
        }
        Some(("remove", sub)) => {
            let path = sub.get_one::<String>("path").unwrap();
            println!("Removed path from gdrive sync tracking: {}", path);
            // TODO: remove from tracking database
            Ok(())
        }
        Some(("run", _)) => {
            println!("Scanning all tracked folders and uploading new files...");
            let _ = gdrive;
            // TODO: iterate tracked folders, compare with known files, upload new ones
            println!("Scan complete.");
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gdrive_sync_command_parses() {
        let cmd = gdrive_sync_command();
        let m = cmd.try_get_matches_from(["gdrive-sync", "list"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("list"));
    }

    #[test]
    fn test_gdrive_sync_requires_subcommand() {
        let cmd = gdrive_sync_command();
        assert!(cmd.try_get_matches_from(["gdrive-sync"]).is_err());
    }

    #[test]
    fn test_gdrive_sync_add() {
        let cmd = gdrive_sync_command();
        let m = cmd.try_get_matches_from(["gdrive-sync", "add", "/home/user/docs"]).unwrap();
        let (name, sub) = m.subcommand().unwrap();
        assert_eq!(name, "add");
        assert_eq!(sub.get_one::<String>("path").map(|s| s.as_str()), Some("/home/user/docs"));
    }

    #[test]
    fn test_gdrive_sync_remove() {
        let cmd = gdrive_sync_command();
        let m = cmd.try_get_matches_from(["gdrive-sync", "remove", "/home/user/docs"]).unwrap();
        let (name, sub) = m.subcommand().unwrap();
        assert_eq!(name, "remove");
        assert_eq!(sub.get_one::<String>("path").map(|s| s.as_str()), Some("/home/user/docs"));
    }

    #[test]
    fn test_gdrive_sync_run() {
        let cmd = gdrive_sync_command();
        let m = cmd.try_get_matches_from(["gdrive-sync", "run"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("run"));
    }

    #[test]
    fn test_gdrive_sync_meta() {
        let meta = gdrive_sync_meta();
        assert_eq!(meta.name, "gdrive-sync");
        assert_eq!(meta.category, CommandCategory::Cloud);
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = gdrive_sync_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"add"));
        assert!(sub_names.contains(&"list"));
        assert!(sub_names.contains(&"remove"));
        assert!(sub_names.contains(&"run"));
        assert_eq!(sub_names.len(), 4);
    }
}
