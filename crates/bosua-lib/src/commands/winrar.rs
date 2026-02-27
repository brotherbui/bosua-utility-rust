//! WinRAR CLI command — WinRAR/unrar utilities.
//!
//! Extracts RAR archives using unrar or 7z as fallback.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};
use crate::utils::run_external_tool;

/// Build the `winrar` clap command.
pub fn winrar_command() -> Command {
    Command::new("winrar")
        .about("WinRAR utilities")
        .arg(
            Arg::new("file")
                .required(true)
                .help("Archive file"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output directory"),
        )
        .arg(
            Arg::new("password")
                .long("password")
                .short('p')
                .help("Archive password"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn winrar_meta() -> CommandMeta {
    CommandBuilder::from_clap(winrar_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `winrar` command.
pub async fn handle_winrar(matches: &ArgMatches) -> Result<()> {
    let file = matches.get_one::<String>("file").unwrap();
    let output = matches.get_one::<String>("output");
    let password = matches.get_one::<String>("password");

    if !std::path::Path::new(file).exists() {
        return Err(BosuaError::Command(format!("Archive not found: {}", file)));
    }

    let output_dir = output.map(|o| o.as_str()).unwrap_or(".");

    // Try unrar first, then 7z as fallback
    let mut args = vec!["x"];

    let pass_arg;
    if let Some(pw) = password {
        pass_arg = format!("-p{}", pw);
        args.push(&pass_arg);
    }

    args.push(file.as_str());
    args.push(output_dir);

    println!("Extracting: {}", file);

    let result = run_external_tool("unrar", &args).await;
    match result {
        Ok(out) => {
            if !out.is_empty() {
                println!("{}", out);
            }
            println!("Extracted to: {}", output_dir);
            Ok(())
        }
        Err(_) => {
            // Fall back to 7z
            println!("unrar not found, trying 7z...");
            let mut z_args = vec!["x"];

            let z_pass_arg;
            if let Some(pw) = password {
                z_pass_arg = format!("-p{}", pw);
                z_args.push(&z_pass_arg);
            }

            let z_out_arg = format!("-o{}", output_dir);
            z_args.push(&z_out_arg);
            z_args.push(file.as_str());

            let out = run_external_tool("7z", &z_args).await.map_err(|e| {
                BosuaError::Command(format!(
                    "Neither unrar nor 7z available to extract '{}': {}",
                    file, e
                ))
            })?;

            if !out.is_empty() {
                println!("{}", out);
            }
            println!("Extracted to: {}", output_dir);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_winrar_command_parses() {
        let cmd = winrar_command();
        let matches = cmd.try_get_matches_from(["winrar", "archive.rar"]).unwrap();
        assert_eq!(
            matches.get_one::<String>("file").map(|s| s.as_str()),
            Some("archive.rar"),
        );
    }

    #[test]
    fn test_winrar_command_with_output() {
        let cmd = winrar_command();
        let matches = cmd
            .try_get_matches_from(["winrar", "archive.rar", "--output", "/tmp/out"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("output").map(|s| s.as_str()),
            Some("/tmp/out"),
        );
    }

    #[test]
    fn test_winrar_command_with_password() {
        let cmd = winrar_command();
        let matches = cmd
            .try_get_matches_from(["winrar", "archive.rar", "--password", "secret"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("password").map(|s| s.as_str()),
            Some("secret"),
        );
    }

    #[test]
    fn test_winrar_requires_file() {
        let cmd = winrar_command();
        let result = cmd.try_get_matches_from(["winrar"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_winrar_meta() {
        let meta = winrar_meta();
        assert_eq!(meta.name, "winrar");
        assert_eq!(meta.category, CommandCategory::Utility);
        assert!(!meta.description.is_empty());
    }
}
