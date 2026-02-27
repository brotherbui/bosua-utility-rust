//! Daemon CLI command with subcommands: start, stop, restart, status, logs, config.
//!
//! Provides the `daemon` command for managing the Bosua background service.
//! Uses systemd integration on Linux via `DaemonManager`.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::daemon::DaemonManager;
use crate::errors::{BosuaError, Result};

/// Build the `daemon` clap command with all subcommands.
pub fn daemon_command() -> Command {
    Command::new("daemon")
        .about("Daemon management")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("start").about("Start the daemon service"))
        .subcommand(Command::new("stop").about("Stop the daemon service"))
        .subcommand(Command::new("restart").about("Restart the daemon service"))
        .subcommand(Command::new("status").about("Show daemon status"))
        .subcommand(
            Command::new("logs")
                .about("Show daemon logs")
                .arg(
                    Arg::new("lines")
                        .long("lines")
                        .short('n')
                        .default_value("50")
                        .value_parser(clap::value_parser!(usize))
                        .help("Number of log lines to show"),
                ),
        )
        .subcommand(Command::new("config").about("Show daemon configuration"))
}

/// Build the `CommandMeta` for registry registration.
pub fn daemon_meta() -> CommandMeta {
    CommandBuilder::from_clap(daemon_command())
        .category(CommandCategory::System)
        .build()
}

/// Handle the `daemon` command dispatch.
pub fn handle_daemon(matches: &ArgMatches, daemon: &DaemonManager) -> Result<()> {
    match matches.subcommand() {
        Some(("start", _)) => {
            daemon.start()?;
            println!("Daemon started.");
        }
        Some(("stop", _)) => {
            daemon.stop()?;
            println!("Daemon stopped.");
        }
        Some(("restart", _)) => {
            daemon.restart()?;
            println!("Daemon restarted.");
        }
        Some(("status", _)) => {
            let status = daemon.status()?;
            println!("Daemon status: {}", status);
        }
        Some(("logs", sub)) => {
            let lines = sub.get_one::<usize>("lines").copied().unwrap_or(50);
            let output = daemon.logs(lines)?;
            println!("{}", output);
        }
        Some(("config", _)) => {
            let cfg = daemon.get_config();
            println!("Service: {}", cfg.service_name);
            if let Some(ref dir) = cfg.working_dir {
                println!("Working directory: {}", dir);
            }
            println!("Workers: {}", cfg.worker_count);
            println!("Restart on failure: {}", cfg.restart_on_failure);
        }
        _ => unreachable!("subcommand_required is set"),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_command_parses_start() {
        let cmd = daemon_command();
        let matches = cmd.try_get_matches_from(["daemon", "start"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("start"));
    }

    #[test]
    fn test_daemon_command_parses_stop() {
        let cmd = daemon_command();
        let matches = cmd.try_get_matches_from(["daemon", "stop"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("stop"));
    }

    #[test]
    fn test_daemon_command_parses_restart() {
        let cmd = daemon_command();
        let matches = cmd.try_get_matches_from(["daemon", "restart"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("restart"));
    }

    #[test]
    fn test_daemon_command_parses_status() {
        let cmd = daemon_command();
        let matches = cmd.try_get_matches_from(["daemon", "status"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("status"));
    }

    #[test]
    fn test_daemon_command_parses_logs() {
        let cmd = daemon_command();
        let matches = cmd
            .try_get_matches_from(["daemon", "logs", "--lines", "100"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(sub.get_one::<usize>("lines").copied(), Some(100));
    }

    #[test]
    fn test_daemon_logs_default_lines() {
        let cmd = daemon_command();
        let matches = cmd.try_get_matches_from(["daemon", "logs"]).unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(sub.get_one::<usize>("lines").copied(), Some(50));
    }

    #[test]
    fn test_daemon_command_parses_config() {
        let cmd = daemon_command();
        let matches = cmd.try_get_matches_from(["daemon", "config"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("config"));
    }

    #[test]
    fn test_daemon_requires_subcommand() {
        let cmd = daemon_command();
        let result = cmd.try_get_matches_from(["daemon"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_daemon_meta() {
        let meta = daemon_meta();
        assert_eq!(meta.name, "daemon");
        assert_eq!(meta.category, CommandCategory::System);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = daemon_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"start"));
        assert!(sub_names.contains(&"stop"));
        assert!(sub_names.contains(&"restart"));
        assert!(sub_names.contains(&"status"));
        assert!(sub_names.contains(&"logs"));
        assert!(sub_names.contains(&"config"));
        assert_eq!(sub_names.len(), 6);
    }
}
