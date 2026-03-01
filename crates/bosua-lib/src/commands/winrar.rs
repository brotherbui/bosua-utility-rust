//! WinRAR CLI command — WinRAR/unrar utilities.
//!
//! Extracts RAR archives using unrar or 7z as fallback.

use clap::{ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::Result;

/// Build the `winrar` clap command.
pub fn winrar_command() -> Command {
    Command::new("winrar")
        .about("Winrar keygen generator")
        .aliases(["wr"])
}

/// Build the `CommandMeta` for registry registration.
pub fn winrar_meta() -> CommandMeta {
    CommandBuilder::from_clap(winrar_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `winrar` command.
pub async fn handle_winrar(matches: &ArgMatches) -> Result<()> {
    let _ = matches;
    println!("Generating WinRAR registration key...");
    println!("winrar keygen: not yet implemented");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_winrar_command_parses() {
        let cmd = winrar_command();
        let _matches = cmd.try_get_matches_from(["winrar"]).unwrap();
    }

    #[test]
    fn test_winrar_alias() {
        let cmd = winrar_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"wr"));
    }

    #[test]
    fn test_winrar_meta() {
        let meta = winrar_meta();
        assert_eq!(meta.name, "winrar");
        assert_eq!(meta.category, CommandCategory::Utility);
    }
}
