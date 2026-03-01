//! Unified cloud CLI command with subcommands.
//!
//! Provides the `cloud` command with subcommands: config, daemon, gdrive,
//! service, setup, stats, sync.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};

/// Build the `cloud` clap command with all subcommands.
pub fn cloud_command() -> Command {
    Command::new("cloud")
        .about("Cloud stuffs")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(Arg::new("backend").long("backend").action(clap::ArgAction::SetTrue).help("Use Backend IP. Will be overwritten by IP if specified"))
        .arg(Arg::new("gcp").long("gcp").action(clap::ArgAction::SetTrue).help("Use GCP IP. Will be overwritten by IP if specified"))
        .arg(Arg::new("ip").long("ip").help("Server IP address"))
        .subcommand(
            Command::new("config")
                .about("Configure services on cloud server")
                .subcommand(Command::new("caddy").about("Configure Caddy reverse proxy")),
        )
        .subcommand(
            Command::new("daemon")
                .about("Manage daemon on remote server")
                .subcommand(Command::new("logs").about("Show daemon logs from remote server"))
                .subcommand(Command::new("restart").about("Restart bosua daemon on remote server"))
                .subcommand(Command::new("start").about("Start bosua daemon on remote server"))
                .subcommand(Command::new("status").about("Check daemon status on remote server"))
                .subcommand(Command::new("stop").about("Stop bosua daemon on remote server")),
        )
        .subcommand(
            Command::new("gdrive")
                .about("Manage Google Drive on remote server")
                .subcommand(Command::new("import").about("Import Google Drive account to remote server")),
        )
        .subcommand(
            Command::new("service")
                .about("Manage services on server")
                .subcommand(Command::new("restart").about("Restart one or more services"))
                .subcommand(Command::new("start").about("Start one or more services"))
                .subcommand(Command::new("status").about("Check status of services"))
                .subcommand(Command::new("stop").about("Stop one or more services")),
        )
        .subcommand(
            Command::new("setup")
                .about("Setup server infrastructure and services")
                .subcommand(Command::new("aria2").about("Install and configure aria2 download manager"))
                .subcommand(Command::new("backend").about("Run backend setup commands"))
                .subcommand(Command::new("caddy").about("Install and configure Caddy web server"))
                .subcommand(Command::new("check-env").about("Check deployment environment and server connectivity"))
                .subcommand(Command::new("cloudflared").about("Install and configure cloudflared daemon"))
                .subcommand(Command::new("dante").about("Install and configure SOCKS5 proxy server"))
                .subcommand(Command::new("deploy").about("Build Linux binary and deploy to server"))
                .subcommand(Command::new("full").about("Run all setup commands"))
                .subcommand(Command::new("gcp").about("Run gcp setup commands"))
                .subcommand(Command::new("kodi-repo").about("Setup Kodi addon repository on remote server"))
                .subcommand(Command::new("latex").about("Install TeX Live for full LaTeX support"))
                .subcommand(Command::new("microsocks").about("Install and configure microsocks SOCKS5 proxy server"))
                .subcommand(Command::new("tailscale").about("Install and configure Tailscale VPN")),
        )
        .subcommand(
            Command::new("stats")
                .about("Display server system information (CPU, memory, disk)")
                .aliases(["info"])
                .arg(Arg::new("gcp").long("gcp").action(clap::ArgAction::SetTrue).help("Check GCP")),
        )
        .subcommand(
            Command::new("sync")
                .about("Sync files to cloud server")
                .subcommand(Command::new("kodi-repo").about("Sync a single Kodi addon to remote server")),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn cloud_meta() -> CommandMeta {
    CommandBuilder::from_clap(cloud_command())
        .category(CommandCategory::Cloud)
        .build()
}

/// Handle the `cloud` command dispatch.
pub fn handle_cloud(matches: &ArgMatches) {
    let _backend = matches.get_flag("backend");
    let _gcp = matches.get_flag("gcp");
    let _ip = matches.get_one::<String>("ip");

    match matches.subcommand() {
        Some(("config", _sub)) => println!("cloud config: not yet implemented"),
        Some(("daemon", _sub)) => println!("cloud daemon: not yet implemented"),
        Some(("gdrive", _sub)) => println!("cloud gdrive: not yet implemented"),
        Some(("service", _sub)) => println!("cloud service: not yet implemented"),
        Some(("setup", _sub)) => println!("cloud setup: not yet implemented"),
        Some(("stats", _sub)) => println!("cloud stats: not yet implemented"),
        Some(("sync", _sub)) => println!("cloud sync: not yet implemented"),
        _ => unreachable!("subcommand_required is set"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_command_parses() {
        let cmd = cloud_command();
        let m = cmd.try_get_matches_from(["cloud", "stats"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("stats"));
    }

    #[test]
    fn test_cloud_persistent_flags() {
        let cmd = cloud_command();
        let m = cmd.try_get_matches_from(["cloud", "--backend", "--ip", "1.2.3.4", "stats"]).unwrap();
        assert!(m.get_flag("backend"));
        assert_eq!(m.get_one::<String>("ip").map(|s| s.as_str()), Some("1.2.3.4"));
    }

    #[test]
    fn test_cloud_requires_subcommand() {
        let cmd = cloud_command();
        assert!(cmd.try_get_matches_from(["cloud"]).is_err());
    }

    #[test]
    fn test_cloud_meta() {
        let meta = cloud_meta();
        assert_eq!(meta.name, "cloud");
        assert_eq!(meta.category, CommandCategory::Cloud);
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = cloud_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        for name in &["config", "daemon", "gdrive", "service", "setup", "stats", "sync"] {
            assert!(sub_names.contains(name), "missing: {}", name);
        }
        assert_eq!(sub_names.len(), 7);
    }
}
