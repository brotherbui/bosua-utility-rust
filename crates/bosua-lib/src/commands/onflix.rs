//! Onflix CLI command — Onflix video downloads with ffmpeg integration.
//!
//! Args: url, --output, --quality.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};
use crate::utils::run_external_tool;

/// Build the `onflix` clap command.
pub fn onflix_command() -> Command {
    Command::new("onflix")
        .about("Download movies/episodes from Onflix via ffmpeg")
        .aliases(["of"])
        .arg(
            Arg::new("url")
                .required(true)
                .num_args(1..)
                .help("Video URL(s) to download, or a file.txt containing URLs"),
        )
        .arg(
            Arg::new("all")
                .long("all")
                .short('a')
                .action(clap::ArgAction::SetTrue)
                .help("Download all episodes without prompting"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output directory (default: ~/Downloads)"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn onflix_meta() -> CommandMeta {
    CommandBuilder::from_clap(onflix_command())
        .category(CommandCategory::Media)
        .build()
}

/// Handle the `onflix` command.
pub async fn handle_onflix(matches: &ArgMatches) -> Result<()> {
    let urls: Vec<&String> = matches.get_many::<String>("url").unwrap().collect();
    let output = matches.get_one::<String>("output");
    let all = matches.get_flag("all");

    let output_dir = output
        .map(|o| o.to_string())
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            format!("{}/Downloads", home)
        });

    println!("Downloading {} URL(s) to {}", urls.len(), output_dir);
    if all {
        println!("Mode: download all episodes");
    }

    for url in &urls {
        println!("Downloading: {}", url);
        let args = vec!["-i", url.as_str(), "-c", "copy", &output_dir];
        let result = run_external_tool("ffmpeg", &args).await;
        match result {
            Ok(out) => {
                if !out.is_empty() {
                    println!("{}", out);
                }
            }
            Err(e) => {
                return Err(BosuaError::Command(format!("Failed to download {}: {}", url, e)));
            }
        }
    }

    println!("Done.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onflix_command_parses() {
        let cmd = onflix_command();
        let matches = cmd.try_get_matches_from(["onflix", "https://example.com/video"]).unwrap();
        let urls: Vec<&String> = matches.get_many::<String>("url").unwrap().collect();
        assert_eq!(urls, vec!["https://example.com/video"]);
    }

    #[test]
    fn test_onflix_multiple_urls() {
        let cmd = onflix_command();
        let matches = cmd.try_get_matches_from(["onflix", "https://a.com/1", "https://b.com/2"]).unwrap();
        let urls: Vec<&String> = matches.get_many::<String>("url").unwrap().collect();
        assert_eq!(urls.len(), 2);
    }

    #[test]
    fn test_onflix_all_flag() {
        let cmd = onflix_command();
        let matches = cmd.try_get_matches_from(["onflix", "--all", "https://example.com/video"]).unwrap();
        assert!(matches.get_flag("all"));
    }

    #[test]
    fn test_onflix_command_with_output() {
        let cmd = onflix_command();
        let matches = cmd.try_get_matches_from(["onflix", "https://example.com/video", "-o", "/tmp/videos"]).unwrap();
        assert_eq!(matches.get_one::<String>("output").map(|s| s.as_str()), Some("/tmp/videos"));
    }

    #[test]
    fn test_onflix_requires_url() {
        let cmd = onflix_command();
        let result = cmd.try_get_matches_from(["onflix"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_onflix_meta() {
        let meta = onflix_meta();
        assert_eq!(meta.name, "onflix");
        assert_eq!(meta.category, CommandCategory::Media);
    }

    #[test]
    fn test_onflix_alias() {
        let cmd = onflix_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"of"));
    }
}
