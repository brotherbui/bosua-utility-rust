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
        .about("Onflix video downloads with ffmpeg")
        .arg(
            Arg::new("url")
                .required(true)
                .help("Video URL to download"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output file path"),
        )
        .arg(
            Arg::new("quality")
                .long("quality")
                .short('q')
                .default_value("best")
                .help("Video quality (best, 1080p, 720p, 480p)"),
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
    let url = matches.get_one::<String>("url").unwrap();
    let output = matches.get_one::<String>("output");
    let quality = matches.get_one::<String>("quality").unwrap();

    println!("Downloading from Onflix: {}", url);
    println!("Quality: {}", quality);

    // Build ffmpeg arguments for stream download
    let output_file = output
        .map(|o| o.to_string())
        .unwrap_or_else(|| "output.mp4".to_string());

    let mut args = vec![
        "-i", url.as_str(),
        "-c", "copy",
    ];

    // Map quality to ffmpeg options
    match quality.as_str() {
        "1080p" => {
            args.extend_from_slice(&["-vf", "scale=-1:1080"]);
        }
        "720p" => {
            args.extend_from_slice(&["-vf", "scale=-1:720"]);
        }
        "480p" => {
            args.extend_from_slice(&["-vf", "scale=-1:480"]);
        }
        "best" | _ => {
            // Use original quality, no scaling
        }
    }

    args.push(&output_file);

    let result = run_external_tool("ffmpeg", &args).await;
    match result {
        Ok(out) => {
            if !out.is_empty() {
                println!("{}", out);
            }
            println!("Downloaded to: {}", output_file);
            Ok(())
        }
        Err(e) => Err(BosuaError::Command(format!(
            "Failed to download video: {}",
            e
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onflix_command_parses() {
        let cmd = onflix_command();
        let matches = cmd
            .try_get_matches_from(["onflix", "https://example.com/video"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("url").map(|s| s.as_str()),
            Some("https://example.com/video"),
        );
        assert_eq!(
            matches.get_one::<String>("quality").map(|s| s.as_str()),
            Some("best"),
        );
    }

    #[test]
    fn test_onflix_command_with_output() {
        let cmd = onflix_command();
        let matches = cmd
            .try_get_matches_from(["onflix", "https://example.com/video", "-o", "/tmp/video.mp4"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("output").map(|s| s.as_str()),
            Some("/tmp/video.mp4"),
        );
    }

    #[test]
    fn test_onflix_command_with_quality() {
        let cmd = onflix_command();
        let matches = cmd
            .try_get_matches_from(["onflix", "https://example.com/video", "--quality", "720p"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("quality").map(|s| s.as_str()),
            Some("720p"),
        );
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
        assert!(!meta.description.is_empty());
    }
}
