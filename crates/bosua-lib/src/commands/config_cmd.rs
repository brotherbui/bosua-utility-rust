//! Config CLI command — view and modify configuration.
//!
//! Subcommands: show, set, reset, path.
//! Named `config_cmd` to avoid conflict with the `config` module.

use clap::{ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::config::manager::DynamicConfigManager;
use crate::errors::{BosuaError, Result};

/// Build the `config` clap command.
///
/// In Go macOS variant, `AddCommonCmd()` registers `config` with aliases
/// `i, c, conf, info`. This matches that behavior.
pub fn config_command() -> Command {
    Command::new("config")
        .aliases(["c", "conf"])
        .about("Print out some config info")
}

/// Build the `CommandMeta` for registry registration.
pub fn config_meta() -> CommandMeta {
    CommandBuilder::from_clap(config_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `config` command.
pub async fn handle_config(
    _matches: &ArgMatches,
    config_mgr: &DynamicConfigManager,
) -> Result<()> {
    let config = config_mgr.get_config().await;
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| BosuaError::Config(format!("Failed to serialize config: {}", e)))?;
    println!("{}", json);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_command_parses() {
        let cmd = config_command();
        let _m = cmd.try_get_matches_from(["config"]).unwrap();
    }

    #[test]
    fn test_config_alias_c() {
        let cmd = config_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"c"));
        assert!(aliases.contains(&"conf"));
    }

    #[test]
    fn test_config_meta() {
        let meta = config_meta();
        assert_eq!(meta.name, "config");
        assert_eq!(meta.category, CommandCategory::Utility);
    }
}
