//! Araxis CLI command — Araxis Merge integration.
//!
//! Launches Araxis Merge (or falls back to diff) for file comparison.

use clap::{ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::Result;

/// Build the `araxis` clap command.
pub fn araxis_command() -> Command {
    Command::new("araxis")
        .about("Araxis Merge stuffs")
        .aliases(["a"])
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("register").about("Register Araxis with auto-generated credentials"))
}

/// Build the `CommandMeta` for registry registration.
pub fn araxis_meta() -> CommandMeta {
    CommandBuilder::from_clap(araxis_command())
        .category(CommandCategory::Developer)
        .build()
}

/// Handle the `araxis` command.
///
/// Delegates to Go binary which has testmail integration and AppleScript
/// automation for Araxis registration.
pub async fn handle_araxis(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("register", _)) => {
            // Delegate to Go binary which has testmail + clipboard + AppleScript integration
            let go_bin = "/opt/homebrew/bin/bosua";
            if !std::path::Path::new(go_bin).exists() {
                return Err(crate::errors::BosuaError::Command(
                    "Araxis register requires the Go binary at /opt/homebrew/bin/bosua (testmail integration not yet ported)".into(),
                ));
            }

            let status = tokio::process::Command::new(go_bin)
                .args(["araxis", "register"])
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()
                .await
                .map_err(|e| crate::errors::BosuaError::Command(format!("Failed to run Go binary: {}", e)))?;

            if !status.success() {
                return Err(crate::errors::BosuaError::Command("Araxis registration failed".into()));
            }
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_araxis_command_parses_register() {
        let cmd = araxis_command();
        let matches = cmd.try_get_matches_from(["araxis", "register"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("register"));
    }

    #[test]
    fn test_araxis_requires_subcommand() {
        let cmd = araxis_command();
        let result = cmd.try_get_matches_from(["araxis"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_araxis_alias() {
        let cmd = araxis_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"a"));
    }

    #[test]
    fn test_araxis_meta() {
        let meta = araxis_meta();
        assert_eq!(meta.name, "araxis");
        assert_eq!(meta.category, CommandCategory::Developer);
    }
}
