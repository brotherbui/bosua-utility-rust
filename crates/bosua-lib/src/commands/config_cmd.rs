//! Config CLI command — view and modify configuration.
//!
//! Subcommands: show, set, reset, path.
//! Named `config_cmd` to avoid conflict with the `config` module.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::config::manager::DynamicConfigManager;
use crate::errors::{BosuaError, Result};
use crate::output;

/// Build the `config` clap command.
///
/// In Go macOS variant, `AddCommonCmd()` registers `config` with aliases
/// `i, c, conf, info`. This matches that behavior.
pub fn config_command() -> Command {
    Command::new("config")
        .aliases(["i", "c", "conf", "info"])
        .about("Print out configuration information")
        .subcommand(
            Command::new("show")
                .about("Show current configuration")
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            Command::new("set")
                .about("Set a configuration value")
                .arg(Arg::new("key").required(true).help("Configuration key"))
                .arg(Arg::new("value").required(true).help("Configuration value")),
        )
        .subcommand(Command::new("reset").about("Reset configuration to defaults"))
        .subcommand(Command::new("path").about("Show configuration file path"))
}

/// Build the `CommandMeta` for registry registration.
pub fn config_meta() -> CommandMeta {
    CommandBuilder::from_clap(config_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `config` command.
pub async fn handle_config(
    matches: &ArgMatches,
    config_mgr: &DynamicConfigManager,
) -> Result<()> {
    match matches.subcommand() {
        Some(("show", sub)) => {
            let config = config_mgr.get_config().await;
            if sub.get_flag("json") {
                let json = serde_json::to_string_pretty(&config)
                    .map_err(|e| BosuaError::Config(format!("Failed to serialize config: {}", e)))?;
                println!("{}", json);
            } else {
                let value = serde_json::to_value(&config)
                    .map_err(|e| BosuaError::Config(format!("Failed to serialize config: {}", e)))?;
                if let Some(obj) = value.as_object() {
                    for (key, val) in obj {
                        println!("{}: {}", key, val);
                    }
                }
            }
            Ok(())
        }
        Some(("set", sub)) => {
            let key = sub.get_one::<String>("key").unwrap().clone();
            let value_str = sub.get_one::<String>("value").unwrap().clone();

            // Parse value: try number first, then fall back to string
            let value: serde_json::Value = if let Ok(n) = value_str.parse::<u64>() {
                serde_json::Value::Number(n.into())
            } else if let Ok(n) = value_str.parse::<f64>() {
                serde_json::Number::from_f64(n)
                    .map(serde_json::Value::Number)
                    .unwrap_or_else(|| serde_json::Value::String(value_str.clone()))
            } else {
                serde_json::Value::String(value_str.clone())
            };

            let mut updates = serde_json::Map::new();
            updates.insert(key.clone(), value);

            config_mgr.update_config(updates).await.map_err(|e| {
                BosuaError::Config(format!("Invalid config key '{}': {}", key, e))
            })?;

            output::success(&format!("Config '{}' set to '{}'", key, value_str));
            Ok(())
        }
        Some(("reset", _)) => {
            config_mgr.reset_to_defaults().await?;
            output::success("Configuration reset to defaults");
            Ok(())
        }
        Some(("path", _)) => {
            let path = config_mgr.config_path();
            println!("{}", path.display());
            Ok(())
        }
        _ => {
            output::info("config: use a subcommand (show, set, reset, path)");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_command_parses_show() {
        let cmd = config_command();
        let matches = cmd.try_get_matches_from(["config", "show"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("show"));
    }

    #[test]
    fn test_config_command_parses_show_json() {
        let cmd = config_command();
        let matches = cmd
            .try_get_matches_from(["config", "show", "--json"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "show");
        assert!(sub.get_flag("json"));
    }

    #[test]
    fn test_config_command_parses_set() {
        let cmd = config_command();
        let matches = cmd
            .try_get_matches_from(["config", "set", "timeout", "60"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "set");
        assert_eq!(sub.get_one::<String>("key").map(|s| s.as_str()), Some("timeout"));
        assert_eq!(sub.get_one::<String>("value").map(|s| s.as_str()), Some("60"));
    }

    #[test]
    fn test_config_command_parses_reset() {
        let cmd = config_command();
        let matches = cmd.try_get_matches_from(["config", "reset"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("reset"));
    }

    #[test]
    fn test_config_command_parses_path() {
        let cmd = config_command();
        let matches = cmd.try_get_matches_from(["config", "path"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("path"));
    }

    #[test]
    fn test_config_meta() {
        let meta = config_meta();
        assert_eq!(meta.name, "config");
        assert_eq!(meta.category, CommandCategory::Utility);
        assert!(!meta.description.is_empty());
    }
}
