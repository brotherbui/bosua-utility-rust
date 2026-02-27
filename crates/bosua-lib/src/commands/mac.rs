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
                .about("Download monitoring")
                .subcommand(
                    Command::new("watch")
                        .about("Watch the Downloads folder for new files"),
                )
                .subcommand(
                    Command::new("list")
                        .about("List recent downloads"),
                ),
        )
        .subcommand(Command::new("email").about("Email utilities"))
        .subcommand(
            Command::new("notes")
                .about("Apple Notes integration")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(Command::new("list").about("List all notes"))
                .subcommand(
                    Command::new("search")
                        .about("Search notes by keyword")
                        .arg(
                            Arg::new("query")
                                .required(true)
                                .help("Search query"),
                        ),
                )
                .subcommand(
                    Command::new("export")
                        .about("Export notes to a directory")
                        .arg(
                            Arg::new("output")
                                .long("output")
                                .short('o')
                                .default_value(".")
                                .help("Output directory"),
                        ),
                ),
        )
        .subcommand(Command::new("system").about("System information"))
        .subcommand(
            Command::new("xcode")
                .about("Xcode project utilities")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(Command::new("list").about("List Xcode projects in current directory"))
                .subcommand(
                    Command::new("build")
                        .about("Build an Xcode project")
                        .arg(
                            Arg::new("project")
                                .required(true)
                                .help("Path to .xcodeproj or .xcworkspace"),
                        )
                        .arg(
                            Arg::new("scheme")
                                .long("scheme")
                                .short('s')
                                .help("Build scheme"),
                        ),
                )
                .subcommand(
                    Command::new("clean")
                        .about("Clean an Xcode project")
                        .arg(
                            Arg::new("project")
                                .required(true)
                                .help("Path to .xcodeproj or .xcworkspace"),
                        ),
                )
                .subcommand(
                    Command::new("run")
                        .about("Run an AppleScript file or expression")
                        .arg(
                            Arg::new("script")
                                .required(true)
                                .help("AppleScript file path or inline expression"),
                        )
                        .arg(
                            Arg::new("inline")
                                .long("inline")
                                .short('e')
                                .action(clap::ArgAction::SetTrue)
                                .help("Treat script argument as inline expression"),
                        ),
                ),
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
        Some(("email", _)) => {
            println!("macOS email utilities are not yet available");
            Ok(())
        }
        Some(("notes", sub)) => handle_notes(sub).await,
        Some(("system", _)) => handle_system().await,
        Some(("xcode", sub)) => handle_xcode(sub).await,
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// Notes handlers — Apple Notes via osascript
// ---------------------------------------------------------------------------

#[cfg(feature = "macos")]
async fn handle_notes(matches: &ArgMatches) -> crate::errors::Result<()> {
    use crate::utils::run_external_tool;

    match matches.subcommand() {
        Some(("list", _)) => {
            let script = r#"tell application "Notes"
    set noteList to every note
    repeat with n in noteList
        log (name of n)
    end repeat
end tell"#;
            let output = run_external_tool("osascript", &["-e", script]).await?;
            if output.trim().is_empty() {
                println!("No notes found");
            } else {
                println!("{}", output.trim());
            }
            Ok(())
        }
        Some(("search", sub)) => {
            let query = sub.get_one::<String>("query").unwrap();
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
        Some(("export", sub)) => {
            let output_dir = sub.get_one::<String>("output").unwrap();
            let script = format!(
                r#"tell application "Notes"
    set noteList to every note
    repeat with n in noteList
        set noteName to name of n
        set noteBody to body of n
        set filePath to POSIX path of "{}" & "/" & noteName & ".html"
        do shell script "echo " & quoted form of noteBody & " > " & quoted form of filePath
    end repeat
    count of noteList
end tell"#,
                output_dir.replace('"', "\\\"")
            );
            let output = run_external_tool("osascript", &["-e", &script]).await?;
            println!("Exported notes to '{}' ({})", output_dir, output.trim());
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
    use crate::utils::run_external_tool;

    match matches.subcommand() {
        Some(("list", _)) => {
            let output = run_external_tool("find", &[".", "-maxdepth", "2", "-name", "*.xcodeproj", "-o", "-name", "*.xcworkspace"]).await?;
            if output.trim().is_empty() {
                println!("No Xcode projects found in current directory");
            } else {
                println!("{}", output.trim());
            }
            Ok(())
        }
        Some(("build", sub)) => {
            let project = sub.get_one::<String>("project").unwrap();
            let mut args = vec!["-project", project.as_str(), "build"];
            let scheme_val;
            if let Some(scheme) = sub.get_one::<String>("scheme") {
                scheme_val = scheme.clone();
                args.insert(1, "-scheme");
                args.insert(2, &scheme_val);
            }
            let output = run_external_tool("xcodebuild", &args).await?;
            println!("{}", output);
            println!("Build succeeded for '{}'", project);
            Ok(())
        }
        Some(("clean", sub)) => {
            let project = sub.get_one::<String>("project").unwrap();
            let output = run_external_tool("xcodebuild", &["-project", project, "clean"]).await?;
            println!("{}", output);
            println!("Clean succeeded for '{}'", project);
            Ok(())
        }
        Some(("run", sub)) => {
            let script = sub.get_one::<String>("script").unwrap();
            let is_inline = sub.get_flag("inline");
            let output = if is_inline {
                run_external_tool("osascript", &["-e", script]).await?
            } else {
                run_external_tool("osascript", &[script]).await?
            };
            if !output.trim().is_empty() {
                println!("{}", output.trim());
            }
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
    use crate::utils::run_external_tool;

    match matches.subcommand() {
        Some(("watch", _)) => {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let downloads_dir = format!("{}/Downloads", home);
            println!("Monitoring downloads in: {}", downloads_dir);
            // Use fswatch or a simple ls-based poll to list new files
            let output = run_external_tool(
                "osascript",
                &["-e", &format!(
                    r#"do shell script "ls -lt '{}' | head -20""#,
                    downloads_dir.replace('\'', "'\\''")
                )],
            )
            .await?;
            println!("{}", output.trim());
            Ok(())
        }
        Some(("list", _)) => {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let downloads_dir = format!("{}/Downloads", home);
            let output = run_external_tool(
                "ls",
                &["-lt", &downloads_dir],
            )
            .await?;
            println!("{}", output.trim());
            Ok(())
        }
        _ => {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let downloads_dir = format!("{}/Downloads", home);
            let output = run_external_tool("ls", &["-lt", &downloads_dir]).await?;
            println!("{}", output.trim());
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// System info handler
// ---------------------------------------------------------------------------

#[cfg(feature = "macos")]
async fn handle_system() -> crate::errors::Result<()> {
    use crate::utils::run_external_tool;

    let output = run_external_tool("sw_vers", &[]).await?;
    println!("{}", output.trim());
    Ok(())
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
        let matches = cmd.try_get_matches_from(["macos", "download"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("download"));
    }

    #[test]
    fn test_mac_command_parses_email() {
        let cmd = mac_command();
        let matches = cmd.try_get_matches_from(["macos", "email"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("email"));
    }

    #[test]
    fn test_mac_command_parses_notes() {
        let cmd = mac_command();
        let matches = cmd.try_get_matches_from(["macos", "notes", "list"]).unwrap();
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
    fn test_mac_command_parses_notes_export() {
        let cmd = mac_command();
        let matches = cmd
            .try_get_matches_from(["macos", "notes", "export", "--output", "/tmp"])
            .unwrap();
        let (_, notes_matches) = matches.subcommand().unwrap();
        let (export_name, export_matches) = notes_matches.subcommand().unwrap();
        assert_eq!(export_name, "export");
        assert_eq!(
            export_matches.get_one::<String>("output").unwrap(),
            "/tmp"
        );
    }

    #[test]
    fn test_mac_command_parses_system() {
        let cmd = mac_command();
        let matches = cmd.try_get_matches_from(["macos", "system"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("system"));
    }

    #[test]
    fn test_mac_command_parses_xcode() {
        let cmd = mac_command();
        let matches = cmd
            .try_get_matches_from(["macos", "xcode", "list"])
            .unwrap();
        let (sub_name, _) = matches.subcommand().unwrap();
        assert_eq!(sub_name, "xcode");
    }

    #[test]
    fn test_mac_command_parses_xcode_build() {
        let cmd = mac_command();
        let matches = cmd
            .try_get_matches_from(["macos", "xcode", "build", "MyApp.xcodeproj", "--scheme", "MyApp"])
            .unwrap();
        let (_, xcode_matches) = matches.subcommand().unwrap();
        let (build_name, build_matches) = xcode_matches.subcommand().unwrap();
        assert_eq!(build_name, "build");
        assert_eq!(
            build_matches.get_one::<String>("project").unwrap(),
            "MyApp.xcodeproj"
        );
        assert_eq!(
            build_matches.get_one::<String>("scheme").unwrap(),
            "MyApp"
        );
    }

    #[test]
    fn test_mac_command_parses_xcode_run_applescript() {
        let cmd = mac_command();
        let matches = cmd
            .try_get_matches_from(["macos", "xcode", "run", "display dialog \"hello\"", "--inline"])
            .unwrap();
        let (_, xcode_matches) = matches.subcommand().unwrap();
        let (run_name, run_matches) = xcode_matches.subcommand().unwrap();
        assert_eq!(run_name, "run");
        assert!(run_matches.get_flag("inline"));
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
        assert!(sub_names.contains(&"email"));
        assert!(sub_names.contains(&"notes"));
        assert!(sub_names.contains(&"system"));
        assert!(sub_names.contains(&"xcode"));
        assert_eq!(sub_names.len(), 5);
    }

    #[test]
    fn test_notes_requires_subcommand() {
        let cmd = mac_command();
        let result = cmd.try_get_matches_from(["macos", "notes"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_xcode_requires_subcommand() {
        let cmd = mac_command();
        let result = cmd.try_get_matches_from(["macos", "xcode"]);
        assert!(result.is_err());
    }
}
