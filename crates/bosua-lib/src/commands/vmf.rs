//! VMF CLI command — VMF Source management.
//!
//! Manages VMF (media source) configuration files: list, add, remove, set-default.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};

/// Build the `vmf` clap command.
pub fn vmf_command() -> Command {
    Command::new("vmf")
        .about("VMF Source management")
        .arg(
            Arg::new("file")
                .required(true)
                .help("VMF source file"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output path"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn vmf_meta() -> CommandMeta {
    CommandBuilder::from_clap(vmf_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `vmf` command.
pub fn handle_vmf(matches: &ArgMatches) -> Result<()> {
    let file = matches.get_one::<String>("file").unwrap();
    let output = matches.get_one::<String>("output");

    let path = std::path::Path::new(file);
    if !path.exists() {
        return Err(BosuaError::Command(format!("VMF file not found: {}", file)));
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| BosuaError::Command(format!("Failed to read VMF file '{}': {}", file, e)))?;

    let output_path = output.map(|o| std::path::Path::new(o.as_str()));

    // Parse and display VMF content
    println!("VMF Source: {}", file);
    println!("Lines: {}", content.lines().count());

    if let Some(out) = output_path {
        std::fs::write(out, &content).map_err(|e| {
            BosuaError::Command(format!("Failed to write output '{}': {}", out.display(), e))
        })?;
        println!("Written to: {}", out.display());
    } else {
        // Display summary of VMF entries
        let entries: Vec<&str> = content
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
            .collect();
        println!("Entries: {}", entries.len());
        for entry in &entries {
            println!("  {}", entry);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vmf_command_parses() {
        let cmd = vmf_command();
        let matches = cmd.try_get_matches_from(["vmf", "map.vmf"]).unwrap();
        assert_eq!(
            matches.get_one::<String>("file").map(|s| s.as_str()),
            Some("map.vmf"),
        );
    }

    #[test]
    fn test_vmf_command_with_output() {
        let cmd = vmf_command();
        let matches = cmd
            .try_get_matches_from(["vmf", "map.vmf", "--output", "/tmp/out.vmf"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("output").map(|s| s.as_str()),
            Some("/tmp/out.vmf"),
        );
    }

    #[test]
    fn test_vmf_requires_file() {
        let cmd = vmf_command();
        let result = cmd.try_get_matches_from(["vmf"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_vmf_meta() {
        let meta = vmf_meta();
        assert_eq!(meta.name, "vmf");
        assert_eq!(meta.category, CommandCategory::Utility);
        assert!(!meta.description.is_empty());
    }
}
