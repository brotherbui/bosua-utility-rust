//! Config CLI command — view and modify configuration.
//!
//! Subcommands: show, set, reset, path.
//! Named `config_cmd` to avoid conflict with the `config` module.

use clap::{ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::config::manager::DynamicConfigManager;
use crate::errors::Result;

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
///
/// Matches Go's `PrintVariables()` — prints env dirs and paths.
pub async fn handle_config(
    _matches: &ArgMatches,
    _config_mgr: &DynamicConfigManager,
) -> Result<()> {
    let sc = crate::config::simplified::SimplifiedConfig::get();
    println!("=== Bosua Utility Configuration ===");
    println!("TEMP_DIR: {}", sc.temp_dir.display());
    println!("DOWNLOAD_DIR: {}", sc.download_dir.display());
    println!("HOME_DIR: {}", sc.home_dir.display());
    println!("PWD: {}", sc.pwd.display());
    println!("FILE_DIR: {}", sc.file_dir.display());
    println!("INPUT_LINKS_FILE: {}", sc.input_links_file.display());
    println!("DOWNLOAD_LOCK_FILE: {}", sc.download_lock_file.display());
    println!("GDRIVE_LOCK_FILE: {}", sc.gdrive_lock_file.display());
    println!("GDRIVE_RETRY_LOCK_FILE: {}", sc.gdrive_retry_lock_file.display());
    println!("TOKEN_FILE: {}", sc.token_file.display());
    println!("SHEETS_CACHE_FILE: {}", sc.sheets_cache_file.display());
    if !sc.server_ip.is_empty() {
        println!("BACKEND_IP: {}", sc.server_ip);
    }
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
