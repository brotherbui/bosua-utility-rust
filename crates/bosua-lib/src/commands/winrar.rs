//! WinRAR CLI command — WinRAR/unrar utilities.
//!
//! Extracts RAR archives using unrar or 7z as fallback.

use clap::{ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};

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
///
/// Generates a WinRAR registration key file (`rarreg.key`) matching Go's
/// `keygen.GenerateRegisterInfo()` + `GenerateRegisterData()`.
///
/// The keygen requires porting the ECC-over-GF(2^15) crypto library from Go.
/// For now, shells out to the Go binary as a bridge.
pub async fn handle_winrar(matches: &ArgMatches) -> Result<()> {
    let args: Vec<String> = matches
        .get_many::<String>("args")
        .unwrap_or_default()
        .cloned()
        .collect();

    let name = args.first().map(|s| s.as_str()).unwrap_or("Brother Bui");
    let license = args.get(1).map(|s| s.as_str()).unwrap_or("BigGun licence");

    // Try to delegate to Go binary which has the full ECC keygen
    let go_bin = "/opt/homebrew/bin/bosua";
    if std::path::Path::new(go_bin).exists() {
        let output = tokio::process::Command::new(go_bin)
            .args(["winrar", name, license])
            .output()
            .await;
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stdout.is_empty() {
                    print!("{}", stdout);
                }
                if !stderr.is_empty() {
                    eprint!("{}", stderr);
                }
                return Ok(());
            }
            Err(e) => {
                return Err(BosuaError::Command(format!(
                    "Failed to run Go binary for keygen: {}",
                    e
                )));
            }
        }
    }

    Err(BosuaError::Command(
        "WinRAR keygen requires the Go binary at /opt/homebrew/bin/bosua (ECC crypto not yet ported to Rust)".into(),
    ))
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
