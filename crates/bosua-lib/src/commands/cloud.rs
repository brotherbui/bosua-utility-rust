//! Unified cloud CLI command with subcommands.
//!
//! Provides the `cloud` command with subcommands: config, daemon, gdrive,
//! service, setup, stats, sync.
//!
//! This is a unified entry point for cloud service configuration and status
//! monitoring.

use clap::{ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};

/// Build the `cloud` clap command with all subcommands.
pub fn cloud_command() -> Command {
    Command::new("cloud")
        .about("Unified cloud service management")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("config").about("Cloud service configuration"))
        .subcommand(Command::new("daemon").about("Cloud daemon management"))
        .subcommand(Command::new("gdrive").about("Google Drive cloud operations"))
        .subcommand(Command::new("service").about("Cloud service status"))
        .subcommand(Command::new("setup").about("Cloud service setup wizard"))
        .subcommand(Command::new("stats").about("Cloud usage statistics"))
        .subcommand(Command::new("sync").about("Cloud synchronization"))
}

/// Build the `CommandMeta` for registry registration.
pub fn cloud_meta() -> CommandMeta {
    CommandBuilder::from_clap(cloud_command())
        .category(CommandCategory::Cloud)
        .build()
}

/// Handle the `cloud` command dispatch.
pub fn handle_cloud(matches: &ArgMatches) {
    match matches.subcommand() {
        Some(("config", _)) => println!("cloud config: not yet implemented"),
        Some(("daemon", _)) => println!("cloud daemon: not yet implemented"),
        Some(("gdrive", _)) => println!("cloud gdrive: not yet implemented"),
        Some(("service", _)) => println!("cloud service: not yet implemented"),
        Some(("setup", _)) => println!("cloud setup: not yet implemented"),
        Some(("stats", _)) => println!("cloud stats: not yet implemented"),
        Some(("sync", _)) => println!("cloud sync: not yet implemented"),
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_command_parses_config() {
        let cmd = cloud_command();
        let matches = cmd.try_get_matches_from(["cloud", "config"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("config"));
    }

    #[test]
    fn test_cloud_command_parses_daemon() {
        let cmd = cloud_command();
        let matches = cmd.try_get_matches_from(["cloud", "daemon"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("daemon"));
    }

    #[test]
    fn test_cloud_command_parses_gdrive() {
        let cmd = cloud_command();
        let matches = cmd.try_get_matches_from(["cloud", "gdrive"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("gdrive"));
    }

    #[test]
    fn test_cloud_command_parses_service() {
        let cmd = cloud_command();
        let matches = cmd.try_get_matches_from(["cloud", "service"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("service"));
    }

    #[test]
    fn test_cloud_command_parses_setup() {
        let cmd = cloud_command();
        let matches = cmd.try_get_matches_from(["cloud", "setup"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("setup"));
    }

    #[test]
    fn test_cloud_command_parses_stats() {
        let cmd = cloud_command();
        let matches = cmd.try_get_matches_from(["cloud", "stats"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("stats"));
    }

    #[test]
    fn test_cloud_command_parses_sync() {
        let cmd = cloud_command();
        let matches = cmd.try_get_matches_from(["cloud", "sync"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("sync"));
    }

    #[test]
    fn test_cloud_requires_subcommand() {
        let cmd = cloud_command();
        let result = cmd.try_get_matches_from(["cloud"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cloud_meta() {
        let meta = cloud_meta();
        assert_eq!(meta.name, "cloud");
        assert_eq!(meta.category, CommandCategory::Cloud);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = cloud_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"config"));
        assert!(sub_names.contains(&"daemon"));
        assert!(sub_names.contains(&"gdrive"));
        assert!(sub_names.contains(&"service"));
        assert!(sub_names.contains(&"setup"));
        assert!(sub_names.contains(&"stats"));
        assert!(sub_names.contains(&"sync"));
        assert_eq!(sub_names.len(), 7);
    }
}
