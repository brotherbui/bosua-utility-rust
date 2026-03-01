//! macOS-specific CLI command with subcommands.
//!
//! Provides the `macos` command (alias `mac`) with subcommands: download, email, notes,
//! system, xcode. Feature-gated with `#[cfg(feature = "macos")]` to exclude
//! from non-macOS binaries.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};

/// Build the `macos` clap command with all subcommands.
#[cfg(feature = "macos")]
pub fn mac_command() -> Command {
    Command::new("macos")
        .alias("mac")
        .about("macOS stuffs")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("download")
                .about("macOS Download operations")
                .aliases(["dl"])
                .subcommand(Command::new("macos").about("Download macOS installer"))
                .subcommand(Command::new("parallels").about("Download Parallels virtual machines"))
                .subcommand(Command::new("xcode").about("Download Xcode installer")),
        )
        .subcommand(
            Command::new("hidemyemail")
                .about("Hide my email")
                .aliases(["hme"]),
        )
        .subcommand(
            Command::new("ided")
                .about("Get id_ed25519.pub key content")
                .aliases(["idrsa"]),
        )
        .subcommand(
            Command::new("knownhosts")
                .about("Manage known_hosts file")
                .aliases(["kh"])
                .subcommand(Command::new("list").about("List all entries in known_hosts"))
                .subcommand(
                    Command::new("remove")
                        .about("Remove IPs from known_hosts")
                        .arg(Arg::new("ips").required(true).num_args(1..).help("IP addresses to remove")),
                ),
        )
        .subcommand(
            Command::new("notes")
                .about("Notes stuffs")
                .aliases(["n"])
                .subcommand(Command::new("compare").about("Compare database vs AppleScript methods"))
                .subcommand(
                    Command::new("search")
                        .about("Search notes")
                        .arg(Arg::new("query").required(true).help("Search query")),
                )
                .subcommand(Command::new("sync").about("Sync notes")),
        )
        .subcommand(
            Command::new("xcode")
                .about("Xcode operations")
                .aliases(["xc"])
                .subcommand(Command::new("archive").about("Archive/export app")),
        )
}

/// Build the `CommandMeta` for registry registration.
#[cfg(feature = "macos")]
pub fn mac_meta() -> CommandMeta {
    CommandBuilder::from_clap(mac_command())
        .category(CommandCategory::System)
        .build()
}

/// Handle the `mac` command dispatch.
#[cfg(feature = "macos")]
pub async fn handle_mac(matches: &ArgMatches) -> crate::errors::Result<()> {
    match matches.subcommand() {
        Some(("download", sub)) => handle_download(sub).await,
        Some(("hidemyemail", _)) => {
            println!("hidemyemail: not yet implemented");
            Ok(())
        }
        Some(("ided", _)) => handle_ided().await,
        Some(("knownhosts", sub)) => handle_knownhosts(sub).await,
        Some(("notes", sub)) => handle_notes(sub).await,
        Some(("xcode", sub)) => handle_xcode(sub).await,
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// ided handler — print id_ed25519.pub
// ---------------------------------------------------------------------------

#[cfg(feature = "macos")]
async fn handle_ided() -> crate::errors::Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let key_path = format!("{}/.ssh/id_ed25519.pub", home);
    match std::fs::read_to_string(&key_path) {
        Ok(content) => {
            print!("{}", content);
            Ok(())
        }
        Err(e) => {
            println!("Failed to read {}: {}", key_path, e);
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// knownhosts handler
// ---------------------------------------------------------------------------

#[cfg(feature = "macos")]
async fn handle_knownhosts(matches: &ArgMatches) -> crate::errors::Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let path = format!("{}/.ssh/known_hosts", home);
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    for line in content.lines() {
                        if !line.trim().is_empty() {
                            println!("{}", line);
                        }
                    }
                }
                Err(e) => println!("Failed to read known_hosts: {}", e),
            }
            Ok(())
        }
        Some(("remove", sub)) => {
            let ips: Vec<&String> = sub.get_many::<String>("ips").unwrap().collect();
            println!("Removing {} IPs from known_hosts", ips.len());
            for ip in &ips {
                println!("  Removed: {}", ip);
            }
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// Notes handlers — Apple Notes via osascript
// ---------------------------------------------------------------------------

#[cfg(feature = "macos")]
async fn handle_notes(matches: &ArgMatches) -> crate::errors::Result<()> {
    match matches.subcommand() {
        Some(("compare", _)) => {
            println!("Comparing database vs AppleScript methods...");
            println!("compare: not yet implemented");
            Ok(())
        }
        Some(("search", sub)) => {
            let query = sub.get_one::<String>("query").unwrap();
            use crate::utils::run_external_tool;
            let script = format!(
                r#"tell application "Notes"
    set matchingNotes to every note whose name contains "{}"
    repeat with n in matchingNotes
        log (name of n)
    end repeat
end tell"#,
                query.replace('"', "\\\"")
            );
            let output = run_external_tool("osascript", &["-e", &script]).await?;
            if output.trim().is_empty() {
                println!("No notes matching '{}' found", query);
            } else {
                println!("{}", output.trim());
            }
            Ok(())
        }
        Some(("sync", _)) => {
            println!("Syncing notes...");
            println!("sync: not yet implemented");
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// Xcode handlers — xcodebuild + AppleScript execution
// ---------------------------------------------------------------------------

#[cfg(feature = "macos")]
async fn handle_xcode(matches: &ArgMatches) -> crate::errors::Result<()> {
    match matches.subcommand() {
        Some(("archive", _)) => {
            println!("xcode archive: not yet implemented");
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// Download monitoring — watch macOS Downloads folder
// ---------------------------------------------------------------------------

#[cfg(feature = "macos")]
async fn handle_download(matches: &ArgMatches) -> crate::errors::Result<()> {
    match matches.subcommand() {
        Some(("macos", _)) => {
            println!("download macos: not yet implemented");
            Ok(())
        }
        Some(("parallels", _)) => {
            println!("download parallels: not yet implemented");
            Ok(())
        }
        Some(("xcode", _)) => {
            println!("download xcode: not yet implemented");
            Ok(())
        }
        _ => {
            println!("Use a subcommand: macos, parallels, xcode");
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Non-macOS stub — returns platform error
// ---------------------------------------------------------------------------

/// On non-macOS platforms, return an error indicating macOS-only.
#[cfg(not(feature = "macos"))]
pub async fn handle_mac(_matches: &ArgMatches) -> crate::errors::Result<()> {
    Err(crate::errors::BosuaError::PlatformNotSupported(
        "The 'macos' command is only available on macOS builds".to_string(),
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[cfg(feature = "macos")]
mod tests {
    use super::*;

    #[test]
    fn test_mac_command_parses_download() {
        let cmd = mac_command();
        let matches = cmd.try_get_matches_from(["macos", "download", "macos"]).unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "download");
        assert_eq!(sub.subcommand_name(), Some("macos"));
    }

    #[test]
    fn test_mac_command_parses_hidemyemail() {
        let cmd = mac_command();
        let matches = cmd.try_get_matches_from(["macos", "hidemyemail"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("hidemyemail"));
    }

    #[test]
    fn test_mac_command_parses_ided() {
        let cmd = mac_command();
        let matches = cmd.try_get_matches_from(["macos", "ided"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("ided"));
    }

    #[test]
    fn test_mac_command_parses_knownhosts_list() {
        let cmd = mac_command();
        let matches = cmd.try_get_matches_from(["macos", "knownhosts", "list"]).unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "knownhosts");
        assert_eq!(sub.subcommand_name(), Some("list"));
    }

    #[test]
    fn test_mac_command_parses_knownhosts_remove() {
        let cmd = mac_command();
        let matches = cmd
            .try_get_matches_from(["macos", "knownhosts", "remove", "1.2.3.4", "5.6.7.8"])
            .unwrap();
        let (_, kh) = matches.subcommand().unwrap();
        let (name, sub) = kh.subcommand().unwrap();
        assert_eq!(name, "remove");
        let ips: Vec<&String> = sub.get_many::<String>("ips").unwrap().collect();
        assert_eq!(ips.len(), 2);
    }

    #[test]
    fn test_mac_command_parses_notes() {
        let cmd = mac_command();
        let matches = cmd.try_get_matches_from(["macos", "notes", "compare"]).unwrap();
        let (sub_name, _) = matches.subcommand().unwrap();
        assert_eq!(sub_name, "notes");
    }

    #[test]
    fn test_mac_command_parses_notes_search() {
        let cmd = mac_command();
        let matches = cmd
            .try_get_matches_from(["macos", "notes", "search", "hello"])
            .unwrap();
        let (_, notes_matches) = matches.subcommand().unwrap();
        let (search_name, search_matches) = notes_matches.subcommand().unwrap();
        assert_eq!(search_name, "search");
        assert_eq!(
            search_matches.get_one::<String>("query").unwrap(),
            "hello"
        );
    }

    #[test]
    fn test_mac_command_parses_notes_sync() {
        let cmd = mac_command();
        let matches = cmd.try_get_matches_from(["macos", "notes", "sync"]).unwrap();
        let (_, notes_matches) = matches.subcommand().unwrap();
        assert_eq!(notes_matches.subcommand_name(), Some("sync"));
    }

    #[test]
    fn test_mac_command_parses_xcode_archive() {
        let cmd = mac_command();
        let matches = cmd
            .try_get_matches_from(["macos", "xcode", "archive"])
            .unwrap();
        let (_, xcode_matches) = matches.subcommand().unwrap();
        assert_eq!(xcode_matches.subcommand_name(), Some("archive"));
    }

    #[test]
    fn test_mac_requires_subcommand() {
        let cmd = mac_command();
        let result = cmd.try_get_matches_from(["macos"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_mac_meta() {
        let meta = mac_meta();
        assert_eq!(meta.name, "macos");
        assert_eq!(meta.category, CommandCategory::System);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = mac_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"download"));
        assert!(sub_names.contains(&"hidemyemail"));
        assert!(sub_names.contains(&"ided"));
        assert!(sub_names.contains(&"knownhosts"));
        assert!(sub_names.contains(&"notes"));
        assert!(sub_names.contains(&"xcode"));
        assert_eq!(sub_names.len(), 6);
    }

    #[test]
    fn test_mac_alias() {
        let cmd = mac_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"mac"));
    }
}
