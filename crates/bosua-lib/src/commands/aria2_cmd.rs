//! Aria2 CLI command — interact with an Aria2 download daemon via JSON-RPC.
//!
//! Subcommands: add, status, pause, unpause, remove, stats.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::download::aria2::Aria2Client;
use crate::errors::{BosuaError, Result};

/// Build the `aria2` clap command with subcommands.
pub fn aria2_command() -> Command {
    Command::new("aria2")
        .about("Aria2 Remote stuffs")
        .aliases(["a2"])
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("add")
                .about("Add a download by URI")
                .arg(
                    Arg::new("uris")
                        .required(true)
                        .num_args(1..)
                        .help("One or more URIs (mirrors for the same file)"),
                )
                .arg(
                    Arg::new("dir")
                        .long("dir")
                        .short('d')
                        .help("Download directory"),
                )
                .arg(
                    Arg::new("out")
                        .long("out")
                        .short('o')
                        .help("Output filename"),
                ),
        )
        .subcommand(
            Command::new("status")
                .about("Query download status")
                .aliases(["check", "c", "chk"])
                .arg(
                    Arg::new("gid")
                        .required(true)
                        .help("Aria2 download GID"),
                ),
        )
        .subcommand(
            Command::new("pause")
                .about("Pause a download")
                .arg(
                    Arg::new("gid")
                        .required(true)
                        .help("Aria2 download GID"),
                ),
        )
        .subcommand(
            Command::new("unpause")
                .about("Resume a paused download")
                .arg(
                    Arg::new("gid")
                        .required(true)
                        .help("Aria2 download GID"),
                ),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove a download")
                .arg(
                    Arg::new("gid")
                        .required(true)
                        .help("Aria2 download GID"),
                ),
        )
        .subcommand(Command::new("stats").about("Show global download statistics"))
}

/// Build the `CommandMeta` for registry registration.
pub fn aria2_meta() -> CommandMeta {
    CommandBuilder::from_clap(aria2_command())
        .category(CommandCategory::Network)
        .build()
}

/// Handle the `aria2` command dispatch.
pub async fn handle_aria2(matches: &ArgMatches, client: &Aria2Client) -> Result<()> {
    match matches.subcommand() {
        Some(("add", sub)) => {
            let uris: Vec<String> = sub
                .get_many::<String>("uris")
                .unwrap()
                .map(|s| s.to_string())
                .collect();

            let mut options = serde_json::Map::new();
            if let Some(dir) = sub.get_one::<String>("dir") {
                options.insert("dir".to_string(), serde_json::Value::String(dir.clone()));
            }
            if let Some(out) = sub.get_one::<String>("out") {
                options.insert("out".to_string(), serde_json::Value::String(out.clone()));
            }

            let opts = if options.is_empty() {
                None
            } else {
                Some(serde_json::Value::Object(options))
            };

            let result = client.add_uri(uris, opts).await?;
            println!("Download added. GID: {}", result);
            Ok(())
        }
        Some(("status", sub)) => {
            let gid = sub.get_one::<String>("gid").unwrap();
            let result = client.tell_status(gid, None).await?;
            println!("{}", serde_json::to_string_pretty(&result).map_err(|e| {
                BosuaError::Application(format!("Failed to format status: {}", e))
            })?);
            Ok(())
        }
        Some(("pause", sub)) => {
            let gid = sub.get_one::<String>("gid").unwrap();
            client.pause(gid).await?;
            println!("Download {} paused.", gid);
            Ok(())
        }
        Some(("unpause", sub)) => {
            let gid = sub.get_one::<String>("gid").unwrap();
            client.unpause(gid).await?;
            println!("Download {} resumed.", gid);
            Ok(())
        }
        Some(("remove", sub)) => {
            let gid = sub.get_one::<String>("gid").unwrap();
            client.remove(gid).await?;
            println!("Download {} removed.", gid);
            Ok(())
        }
        Some(("stats", _)) => {
            let result = client.get_global_stat().await?;
            println!("{}", serde_json::to_string_pretty(&result).map_err(|e| {
                BosuaError::Application(format!("Failed to format stats: {}", e))
            })?);
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aria2_command_add() {
        let cmd = aria2_command();
        let matches = cmd
            .try_get_matches_from(["aria2", "add", "https://example.com/file.zip"])
            .unwrap();
        let (name, _) = matches.subcommand().unwrap();
        assert_eq!(name, "add");
    }

    #[test]
    fn test_aria2_command_add_multiple_uris() {
        let cmd = aria2_command();
        let matches = cmd
            .try_get_matches_from([
                "aria2",
                "add",
                "https://mirror1.com/file.zip",
                "https://mirror2.com/file.zip",
            ])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let uris: Vec<&String> = sub.get_many::<String>("uris").unwrap().collect();
        assert_eq!(uris.len(), 2);
    }

    #[test]
    fn test_aria2_command_add_with_options() {
        let cmd = aria2_command();
        let matches = cmd
            .try_get_matches_from([
                "aria2",
                "add",
                "https://example.com/file.zip",
                "--dir",
                "/tmp",
                "--out",
                "output.zip",
            ])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(
            sub.get_one::<String>("dir").map(|s| s.as_str()),
            Some("/tmp")
        );
        assert_eq!(
            sub.get_one::<String>("out").map(|s| s.as_str()),
            Some("output.zip")
        );
    }

    #[test]
    fn test_aria2_command_status() {
        let cmd = aria2_command();
        let matches = cmd
            .try_get_matches_from(["aria2", "status", "abc123"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "status");
        assert_eq!(
            sub.get_one::<String>("gid").map(|s| s.as_str()),
            Some("abc123")
        );
    }

    #[test]
    fn test_aria2_command_status_check_alias() {
        let cmd = aria2_command();
        let matches = cmd
            .try_get_matches_from(["aria2", "check", "abc123"])
            .unwrap();
        let (name, _) = matches.subcommand().unwrap();
        assert_eq!(name, "status");
    }

    #[test]
    fn test_aria2_command_pause() {
        let cmd = aria2_command();
        let matches = cmd
            .try_get_matches_from(["aria2", "pause", "abc123"])
            .unwrap();
        let (name, _) = matches.subcommand().unwrap();
        assert_eq!(name, "pause");
    }

    #[test]
    fn test_aria2_command_unpause() {
        let cmd = aria2_command();
        let matches = cmd
            .try_get_matches_from(["aria2", "unpause", "abc123"])
            .unwrap();
        let (name, _) = matches.subcommand().unwrap();
        assert_eq!(name, "unpause");
    }

    #[test]
    fn test_aria2_command_remove() {
        let cmd = aria2_command();
        let matches = cmd
            .try_get_matches_from(["aria2", "remove", "abc123"])
            .unwrap();
        let (name, _) = matches.subcommand().unwrap();
        assert_eq!(name, "remove");
    }

    #[test]
    fn test_aria2_command_stats() {
        let cmd = aria2_command();
        let matches = cmd
            .try_get_matches_from(["aria2", "stats"])
            .unwrap();
        let (name, _) = matches.subcommand().unwrap();
        assert_eq!(name, "stats");
    }

    #[test]
    fn test_aria2_requires_subcommand() {
        let cmd = aria2_command();
        let result = cmd.try_get_matches_from(["aria2"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_aria2_meta() {
        let meta = aria2_meta();
        assert_eq!(meta.name, "aria2");
        assert_eq!(meta.category, CommandCategory::Network);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_aria2_a2_alias() {
        let cmd = aria2_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"a2"));
    }
}
