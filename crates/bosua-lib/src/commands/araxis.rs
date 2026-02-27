//! Araxis CLI command — Araxis Merge integration.
//!
//! Launches Araxis Merge (or falls back to diff) for file comparison.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};
use crate::utils::run_external_tool;

/// Build the `araxis` clap command.
pub fn araxis_command() -> Command {
    Command::new("araxis")
        .about("Araxis Merge integration")
        .arg(
            Arg::new("left")
                .required(true)
                .help("Left file for comparison"),
        )
        .arg(
            Arg::new("right")
                .required(true)
                .help("Right file for comparison"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output path for merge result"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn araxis_meta() -> CommandMeta {
    CommandBuilder::from_clap(araxis_command())
        .category(CommandCategory::Developer)
        .build()
}

/// Handle the `araxis` command.
pub async fn handle_araxis(matches: &ArgMatches) -> Result<()> {
    let left = matches.get_one::<String>("left").unwrap();
    let right = matches.get_one::<String>("right").unwrap();
    let output = matches.get_one::<String>("output");

    // Verify files exist
    if !std::path::Path::new(left).exists() {
        return Err(BosuaError::Command(format!("Left file not found: {}", left)));
    }
    if !std::path::Path::new(right).exists() {
        return Err(BosuaError::Command(format!("Right file not found: {}", right)));
    }

    // Try Araxis Merge first, fall back to diff
    let mut args = vec![left.as_str(), right.as_str()];
    let output_str;
    if let Some(out) = output {
        output_str = format!("-merge:{}", out);
        args.push(&output_str);
    }

    // Try araxismerge (macOS) or compare (Windows)
    let result = run_external_tool("araxismerge", &args).await;
    match result {
        Ok(out) => {
            if !out.is_empty() {
                println!("{}", out);
            }
            println!("Araxis Merge comparison complete");
            Ok(())
        }
        Err(_) => {
            // Fall back to diff
            println!("Araxis Merge not found, falling back to diff");
            let diff_args = vec![left.as_str(), right.as_str()];
            match run_external_tool("diff", &diff_args).await {
                Ok(out) => {
                    println!("{}", out);
                    Ok(())
                }
                Err(e) => Err(BosuaError::Command(format!(
                    "Neither Araxis Merge nor diff available: {}",
                    e
                ))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_araxis_command_parses() {
        let cmd = araxis_command();
        let matches = cmd
            .try_get_matches_from(["araxis", "file_a.txt", "file_b.txt"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("left").map(|s| s.as_str()),
            Some("file_a.txt"),
        );
        assert_eq!(
            matches.get_one::<String>("right").map(|s| s.as_str()),
            Some("file_b.txt"),
        );
    }

    #[test]
    fn test_araxis_command_with_output() {
        let cmd = araxis_command();
        let matches = cmd
            .try_get_matches_from(["araxis", "a.txt", "b.txt", "--output", "merged.txt"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("output").map(|s| s.as_str()),
            Some("merged.txt"),
        );
    }

    #[test]
    fn test_araxis_requires_both_files() {
        let cmd = araxis_command();
        let result = cmd.try_get_matches_from(["araxis", "only_one.txt"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_araxis_requires_left() {
        let cmd = araxis_command();
        let result = cmd.try_get_matches_from(["araxis"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_araxis_meta() {
        let meta = araxis_meta();
        assert_eq!(meta.name, "araxis");
        assert_eq!(meta.category, CommandCategory::Developer);
        assert!(!meta.description.is_empty());
    }
}
