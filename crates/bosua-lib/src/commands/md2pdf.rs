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
        .about("Convert Markdown files to PDF format with beautiful styling")
        .aliases(["mdpdf", "markdown"])
        .arg(
            Arg::new("input")
                .required(true)
                .help("Input Markdown file"),
        )
        .arg(
            Arg::new("css")
                .long("css")
                .help("Custom CSS file for styling"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output PDF file (default: same name as input with .pdf extension)"),
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

    if let Some(css) = matches.get_one::<String>("css") {
        args.push("--css");
        args.push(css.as_str());
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
        assert_eq!(matches.get_one::<String>("input").map(|s| s.as_str()), Some("README.md"));
    }

    #[test]
    fn test_md2pdf_with_output() {
        let cmd = md2pdf_command();
        let matches = cmd.try_get_matches_from(["md2pdf", "README.md", "--output", "out.pdf"]).unwrap();
        assert_eq!(matches.get_one::<String>("output").map(|s| s.as_str()), Some("out.pdf"));
    }

    #[test]
    fn test_md2pdf_with_css() {
        let cmd = md2pdf_command();
        let matches = cmd.try_get_matches_from(["md2pdf", "README.md", "--css", "custom.css"]).unwrap();
        assert_eq!(matches.get_one::<String>("css").map(|s| s.as_str()), Some("custom.css"));
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
    }

    #[test]
    fn test_md2pdf_aliases() {
        let cmd = md2pdf_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"mdpdf"));
        assert!(aliases.contains(&"markdown"));
    }
}
