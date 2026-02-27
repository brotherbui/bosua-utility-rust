//! Cron CLI command with subcommands: list, add, remove, run.
//!
//! Provides the `cron` command for managing scheduled tasks.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::daemon::cron::{CronJob, CronManager};
use crate::errors::Result;

/// Build the `cron` clap command with all subcommands.
pub fn cron_command() -> Command {
    Command::new("cron")
        .about("Cron job scheduling")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("list").about("List all cron jobs"))
        .subcommand(
            Command::new("add")
                .about("Add a new cron job")
                .arg(Arg::new("name").required(true).help("Job name"))
                .arg(
                    Arg::new("schedule")
                        .required(true)
                        .help("Cron schedule expression (e.g. \"0 */6 * * *\")"),
                )
                .arg(
                    Arg::new("command")
                        .required(true)
                        .help("Command to execute"),
                ),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove a cron job")
                .arg(Arg::new("name").required(true).help("Job name to remove")),
        )
        .subcommand(Command::new("run").about("Run all pending cron jobs"))
}

/// Build the `CommandMeta` for registry registration.
pub fn cron_meta() -> CommandMeta {
    CommandBuilder::from_clap(cron_command())
        .category(CommandCategory::System)
        .build()
}

/// Handle the `cron` command dispatch.
pub fn handle_cron(matches: &ArgMatches, cron: &mut CronManager) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let jobs = cron.list_jobs();
            if jobs.is_empty() {
                println!("No cron jobs registered.");
            } else {
                println!("{:<20} {:<20} {:<30} {}", "NAME", "SCHEDULE", "COMMAND", "ENABLED");
                println!("{}", "-".repeat(78));
                for job in jobs {
                    println!(
                        "{:<20} {:<20} {:<30} {}",
                        job.name, job.schedule, job.command, job.enabled
                    );
                }
            }
        }
        Some(("add", sub)) => {
            let name = sub.get_one::<String>("name").unwrap();
            let schedule = sub.get_one::<String>("schedule").unwrap();
            let command = sub.get_one::<String>("command").unwrap();
            let job = CronJob::new(name.clone(), schedule.clone(), command.clone());
            cron.add_job(job)?;
            println!("Cron job '{}' added.", name);
        }
        Some(("remove", sub)) => {
            let name = sub.get_one::<String>("name").unwrap();
            cron.remove_job(name)?;
            println!("Cron job '{}' removed.", name);
        }
        Some(("run", _)) => {
            let executed = cron.run_pending();
            if executed.is_empty() {
                println!("No pending jobs to execute.");
            } else {
                println!("Executed {} job(s):", executed.len());
                for name in &executed {
                    println!("  - {}", name);
                }
            }
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
    fn test_cron_command_parses_list() {
        let cmd = cron_command();
        let matches = cmd.try_get_matches_from(["cron", "list"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("list"));
    }

    #[test]
    fn test_cron_command_parses_add() {
        let cmd = cron_command();
        let matches = cmd
            .try_get_matches_from(["cron", "add", "backup", "0 2 * * *", "bosua gdrive-sync"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(sub.get_one::<String>("name").map(|s| s.as_str()), Some("backup"));
        assert_eq!(sub.get_one::<String>("schedule").map(|s| s.as_str()), Some("0 2 * * *"));
        assert_eq!(sub.get_one::<String>("command").map(|s| s.as_str()), Some("bosua gdrive-sync"));
    }

    #[test]
    fn test_cron_command_parses_remove() {
        let cmd = cron_command();
        let matches = cmd
            .try_get_matches_from(["cron", "remove", "backup"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(sub.get_one::<String>("name").map(|s| s.as_str()), Some("backup"));
    }

    #[test]
    fn test_cron_command_parses_run() {
        let cmd = cron_command();
        let matches = cmd.try_get_matches_from(["cron", "run"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("run"));
    }

    #[test]
    fn test_cron_requires_subcommand() {
        let cmd = cron_command();
        let result = cmd.try_get_matches_from(["cron"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cron_add_requires_all_args() {
        let cmd = cron_command();
        // Missing command arg
        let result = cmd.try_get_matches_from(["cron", "add", "backup", "0 2 * * *"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cron_remove_requires_name() {
        let cmd = cron_command();
        let result = cmd.try_get_matches_from(["cron", "remove"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cron_meta() {
        let meta = cron_meta();
        assert_eq!(meta.name, "cron");
        assert_eq!(meta.category, CommandCategory::System);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = cron_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"list"));
        assert!(sub_names.contains(&"add"));
        assert!(sub_names.contains(&"remove"));
        assert!(sub_names.contains(&"run"));
        assert_eq!(sub_names.len(), 4);
    }
}
