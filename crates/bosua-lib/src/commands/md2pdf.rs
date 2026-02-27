//! Markdown to PDF conversion command.
//!
//! Provides the `md2pdf` command for converting Markdown files to PDF.

use std::path::Path;

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};
use crate::utils::run_external_tool;

/// Build the `md2pdf` clap command.
pub fn md2pdf_command() -> Command {
    Command::new("md2pdf")
        .about("Markdown to PDF conversion")
        .arg(
            Arg::new("input")
                .required(true)
                .help("Input Markdown file"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output PDF file path"),
        )
        .arg(
            Arg::new("template")
                .long("template")
                .short('t')
                .help("PDF template to use"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn md2pdf_meta() -> CommandMeta {
    CommandBuilder::from_clap(md2pdf_command())
        .category(CommandCategory::Developer)
        .build()
}

/// Handle the `md2pdf` command.
pub async fn handle_md2pdf(matches: &ArgMatches) -> Result<()> {
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
            .with_extension("pdf")
            .to_string_lossy()
            .to_string(),
    };

    let mut args = vec![input.as_str(), "-o", output.as_str()];

    if let Some(template) = matches.get_one::<String>("template") {
        args.push("--template");
        args.push(template.as_str());
    }

    run_external_tool("pandoc", &args).await?;
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
    fn test_md2pdf_command_parses_input() {
        let cmd = md2pdf_command();
        let matches = cmd.try_get_matches_from(["md2pdf", "README.md"]).unwrap();
        assert_eq!(
            matches.get_one::<String>("input").map(|s| s.as_str()),
            Some("README.md"),
        );
    }

    #[test]
    fn test_md2pdf_with_output() {
        let cmd = md2pdf_command();
        let matches = cmd
            .try_get_matches_from(["md2pdf", "README.md", "--output", "out.pdf"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("output").map(|s| s.as_str()),
            Some("out.pdf"),
        );
    }

    #[test]
    fn test_md2pdf_with_template() {
        let cmd = md2pdf_command();
        let matches = cmd
            .try_get_matches_from(["md2pdf", "README.md", "--template", "academic"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("template").map(|s| s.as_str()),
            Some("academic"),
        );
    }

    #[test]
    fn test_md2pdf_requires_input() {
        let cmd = md2pdf_command();
        let result = cmd.try_get_matches_from(["md2pdf"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_md2pdf_meta() {
        let meta = md2pdf_meta();
        assert_eq!(meta.name, "md2pdf");
        assert_eq!(meta.category, CommandCategory::Developer);
        assert!(!meta.description.is_empty());
    }
}
