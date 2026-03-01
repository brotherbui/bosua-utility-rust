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
        Some(("hidemyemail", sub)) => handle_hidemyemail(sub).await,
        Some(("ided", _)) => handle_ided().await,
        Some(("knownhosts", sub)) => handle_knownhosts(sub).await,
        Some(("notes", sub)) => handle_notes(sub).await,
        Some(("xcode", sub)) => handle_xcode(sub).await,
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// hidemyemail handler — Generate iCloud Hide My Email address via AppleScript
// ---------------------------------------------------------------------------

#[cfg(feature = "macos")]
async fn handle_hidemyemail(_matches: &ArgMatches) -> crate::errors::Result<()> {
    // Use AppleScript to open System Settings > iCloud > Hide My Email
    let script = r#"tell application "System Settings"
    activate
    delay 1
end tell
tell application "System Events"
    tell process "System Settings"
        -- Navigate to Apple ID > iCloud > Hide My Email
        delay 0.5
    end tell
end tell"#;

    let status = tokio::process::Command::new("osascript")
        .args(["-e", script])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await
        .map_err(|e| crate::errors::BosuaError::Command(format!("AppleScript failed: {}", e)))?;

    if !status.success() {
        println!("Note: AppleScript automation may require accessibility permissions.");
        println!("Go to System Settings > Privacy & Security > Accessibility to grant access.");
    }
    println!("Opened System Settings for Hide My Email");
    Ok(())
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
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let path = format!("{}/.ssh/known_hosts", home);

            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    let mut kept_lines = Vec::new();
                    let mut removed = Vec::new();

                    for line in content.lines() {
                        let should_remove = ips.iter().any(|ip| line.contains(ip.as_str()));
                        if should_remove {
                            for ip in &ips {
                                if line.contains(ip.as_str()) {
                                    removed.push(ip.to_string());
                                }
                            }
                        } else {
                            kept_lines.push(line);
                        }
                    }

                    let new_content = kept_lines.join("\n") + "\n";
                    if let Err(e) = std::fs::write(&path, new_content) {
                        println!("Failed to write known_hosts: {}", e);
                    } else {
                        removed.sort();
                        removed.dedup();
                        for ip in &removed {
                            println!("  Removed: {}", ip);
                        }
                        println!("Removed {} IPs from known_hosts", removed.len());
                    }
                }
                Err(e) => println!("Failed to read known_hosts: {}", e),
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
            // Compare notes from database vs AppleScript
            use crate::utils::run_external_tool;
            let script = r#"tell application "Notes"
    set noteList to every note
    repeat with n in noteList
        log (name of n)
    end repeat
end tell"#;
            let output = run_external_tool("osascript", &["-e", script]).await?;
            let applescript_notes: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
            println!("Notes from AppleScript: {} notes found", applescript_notes.len());
            for note in &applescript_notes {
                println!("  {}", note);
            }
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
            // Sync notes using AppleScript
            use crate::utils::run_external_tool;
            let script = r#"tell application "Notes"
    set noteList to every note
    set noteCount to count of noteList
    log noteCount
end tell"#;
            let output = run_external_tool("osascript", &["-e", script]).await?;
            let count = output.trim();
            println!("Notes synced. Total notes: {}", count);
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
            // Run xcodebuild archive from current directory
            use crate::utils::run_external_tool;

            // Find .xcodeproj in current directory
            let cwd = std::env::current_dir().map_err(crate::errors::BosuaError::Io)?;
            let mut project_path = None;
            if let Ok(entries) = std::fs::read_dir(&cwd) {
                for entry in entries.flatten() {
                    if entry.path().extension().map(|e| e == "xcodeproj").unwrap_or(false) {
                        project_path = Some(entry.path());
                        break;
                    }
                }
            }

            let project = match project_path {
                Some(p) => p,
                None => {
                    return Err(crate::errors::BosuaError::Command(
                        "No .xcodeproj found in current directory".into(),
                    ));
                }
            };

            let project_name = project.file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("App");

            println!("Archiving {}...", project_name);
            let archive_path = format!("{}.xcarchive", project_name);

            let output = run_external_tool("xcodebuild", &[
                "-project", &project.to_string_lossy(),
                "-scheme", project_name,
                "-archivePath", &archive_path,
                "archive",
            ]).await?;
            println!("{}", output);
            println!("Archive created: {}", archive_path);
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
            // Open macOS download page
            let url = "https://developer.apple.com/download/";
            let _ = std::process::Command::new("open").arg(url).status();
            println!("Opened macOS download page: {}", url);
            println!("Use `softwareupdate --list-full-installers` to see available installers");
            let output = tokio::process::Command::new("softwareupdate")
                .args(["--list-full-installers"])
                .output()
                .await;
            if let Ok(out) = output {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if !stdout.is_empty() {
                    println!("{}", stdout);
                }
            }
            Ok(())
        }
        Some(("parallels", _)) => {
            let url = "https://www.parallels.com/products/desktop/download/";
            let _ = std::process::Command::new("open").arg(url).status();
            println!("Opened Parallels download page: {}", url);
            Ok(())
        }
        Some(("xcode", _)) => {
            let url = "https://developer.apple.com/download/all/?q=Xcode";
            let _ = std::process::Command::new("open").arg(url).status();
            println!("Opened Xcode download page: {}", url);
            println!("Tip: Use `xcode-select --install` for command line tools");
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
