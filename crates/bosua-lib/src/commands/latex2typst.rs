//! LaTeX to Typst conversion command.
//!
//! Provides the `latex2typst` command for converting LaTeX files to Typst format.

use std::path::Path;

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};
use crate::utils::run_external_tool;

/// Build the `latex2typst` clap command.
pub fn latex2typst_command() -> Command {
    Command::new("latex2typst")
        .about("Convert LaTeX files to Typst format")
        .aliases(["tex2typ", "tex2typst"])
        .arg(
            Arg::new("input")
                .required(true)
                .help("Input LaTeX file"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output Typst file path"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn latex2typst_meta() -> CommandMeta {
    CommandBuilder::from_clap(latex2typst_command())
        .category(CommandCategory::Developer)
        .build()
}

/// Handle the `latex2typst` command.
pub async fn handle_latex2typst(matches: &ArgMatches) -> Result<()> {
    let input = matches.get_one::<String>("input").unwrap();
    let input_path = Path::new(input);

    if !input_path.exists() {
        return Err(BosuaError::Command(format!(
            "Input file not found: {}",
            input
        )));
    }

    let output = match matches.get_one::<String>("output") {
        Some(o) => o.clone(),
        None => input_path
            .with_extension("typ")
            .to_string_lossy()
            .to_string(),
    };

    run_external_tool(
        "pandoc",
        &[input, "-o", &output, "-f", "latex", "-t", "typst"],
    )
    .await?;

    println!("Converted {} to {}", input, output);
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latex2typst_command_parses_input() {
        let cmd = latex2typst_command();
        let matches = cmd.try_get_matches_from(["latex2typst", "paper.tex"]).unwrap();
        assert_eq!(
            matches.get_one::<String>("input").map(|s| s.as_str()),
            Some("paper.tex"),
        );
    }

    #[test]
    fn test_latex2typst_with_output() {
        let cmd = latex2typst_command();
        let matches = cmd
            .try_get_matches_from(["latex2typst", "paper.tex", "--output", "paper.typ"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("output").map(|s| s.as_str()),
            Some("paper.typ"),
        );
    }

    #[test]
    fn test_latex2typst_requires_input() {
        let cmd = latex2typst_command();
        let result = cmd.try_get_matches_from(["latex2typst"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_latex2typst_meta() {
        let meta = latex2typst_meta();
        assert_eq!(meta.name, "latex2typst");
        assert_eq!(meta.category, CommandCategory::Developer);
        assert!(!meta.description.is_empty());
    }
}
