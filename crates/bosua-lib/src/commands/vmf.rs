//! VMF CLI command — Manage VMF sources (Google Sheets) for movie searches.
//!
//! Subcommands: add, delete, disable, edit, enable, info, init, list, migrate.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::Result;

/// Build the `vmf` clap command.
pub fn vmf_command() -> Command {
    Command::new("vmf")
        .about("Manage VMF sources (Google Sheets) for movie searches. Supports list, add, delete, edit, and migrate operations.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("add").about("Add a new VMF source"))
        .subcommand(
            Command::new("delete")
                .about("Delete a VMF source by ID")
                .arg(Arg::new("id").required(true).help("VMF source ID")),
        )
        .subcommand(
            Command::new("disable")
                .about("Disable a VMF source")
                .arg(Arg::new("id").required(true).help("VMF source ID")),
        )
        .subcommand(
            Command::new("edit")
                .about("Edit an existing VMF source")
                .arg(Arg::new("id").required(true).help("VMF source ID")),
        )
        .subcommand(
            Command::new("enable")
                .about("Enable a VMF source")
                .arg(Arg::new("id").required(true).help("VMF source ID")),
        )
        .subcommand(
            Command::new("info")
                .about("Show detailed information about a VMF source")
                .arg(Arg::new("id").required(true).help("VMF source ID")),
        )
        .subcommand(Command::new("init").about("Initialize database with default VMF sources"))
        .subcommand(Command::new("list").about("List all VMF sources"))
        .subcommand(Command::new("migrate").about("Migrate sources from text file to database"))
}

/// Build the `CommandMeta` for registry registration.
pub fn vmf_meta() -> CommandMeta {
    CommandBuilder::from_clap(vmf_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `vmf` command.
pub fn handle_vmf(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("add", _)) => {
            println!("vmf add: not yet implemented");
            Ok(())
        }
        Some(("delete", sub)) => {
            let id = sub.get_one::<String>("id").unwrap();
            println!("vmf delete {}: not yet implemented", id);
            Ok(())
        }
        Some(("disable", sub)) => {
            let id = sub.get_one::<String>("id").unwrap();
            println!("vmf disable {}: not yet implemented", id);
            Ok(())
        }
        Some(("edit", sub)) => {
            let id = sub.get_one::<String>("id").unwrap();
            println!("vmf edit {}: not yet implemented", id);
            Ok(())
        }
        Some(("enable", sub)) => {
            let id = sub.get_one::<String>("id").unwrap();
            println!("vmf enable {}: not yet implemented", id);
            Ok(())
        }
        Some(("info", sub)) => {
            let id = sub.get_one::<String>("id").unwrap();
            println!("vmf info {}: not yet implemented", id);
            Ok(())
        }
        Some(("init", _)) => {
            println!("vmf init: not yet implemented");
            Ok(())
        }
        Some(("list", _)) => {
            println!("vmf list: not yet implemented");
            Ok(())
        }
        Some(("migrate", _)) => {
            println!("vmf migrate: not yet implemented");
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vmf_command_parses() {
        let cmd = vmf_command();
        let m = cmd.try_get_matches_from(["vmf", "list"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("list"));
    }

    #[test]
    fn test_vmf_requires_subcommand() {
        let cmd = vmf_command();
        assert!(cmd.try_get_matches_from(["vmf"]).is_err());
    }

    #[test]
    fn test_vmf_delete() {
        let cmd = vmf_command();
        let m = cmd.try_get_matches_from(["vmf", "delete", "42"]).unwrap();
        let (name, sub) = m.subcommand().unwrap();
        assert_eq!(name, "delete");
        assert_eq!(sub.get_one::<String>("id").map(|s| s.as_str()), Some("42"));
    }

    #[test]
    fn test_vmf_meta() {
        let meta = vmf_meta();
        assert_eq!(meta.name, "vmf");
        assert_eq!(meta.category, CommandCategory::Utility);
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = vmf_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        for name in &["add", "delete", "disable", "edit", "enable", "info", "init", "list", "migrate"] {
            assert!(sub_names.contains(name), "missing subcommand: {}", name);
        }
        assert_eq!(sub_names.len(), 9);
    }
}
