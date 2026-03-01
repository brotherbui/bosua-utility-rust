//! LaTeX to PDF conversion command.
//!
//! Provides the `latex2pdf` command for converting LaTeX files to PDF.
//! Supports multiple engines: pdflatex, xelatex, lualatex.

use std::path::Path;

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};
use crate::utils::run_external_tool;

/// Build the `latex2pdf` clap command.
pub fn latex2pdf_command() -> Command {
    Command::new("latex2pdf")
        .about("Convert LaTeX files to PDF format")
        .aliases(["tex2pdf", "latexpdf"])
        .arg(
            Arg::new("input")
                .required(true)
                .help("Input LaTeX file"),
        )
        .arg(
            Arg::new("css")
                .long("css")
                .help("Custom CSS file for styling (local mode only)"),
        )
        .arg(
            Arg::new("engine")
                .long("engine")
                .value_parser(["auto", "pdflatex", "xelatex", "lualatex"])
                .default_value("auto")
                .help("LaTeX engine: auto, pdflatex, xelatex, lualatex"),
        )
        .arg(
            Arg::new("local")
                .long("local")
                .action(clap::ArgAction::SetTrue)
                .help("Force local browser-based conversion"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output PDF file (default: same name as input with .pdf extension)"),
        )
        .arg(
            Arg::new("remote")
                .long("remote")
                .action(clap::ArgAction::SetTrue)
                .help("Force remote server conversion with full TeX Live"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn latex2pdf_meta() -> CommandMeta {
    CommandBuilder::from_clap(latex2pdf_command())
        .category(CommandCategory::Developer)
        .build()
}

/// Handle the `latex2pdf` command.
pub async fn handle_latex2pdf(matches: &ArgMatches) -> Result<()> {
    let input = matches.get_one::<String>("input").unwrap();
    let input_path = Path::new(input);

    if !input_path.exists() {
        return Err(BosuaError::Command(format!(
            "Input file not found: {}",
            input
        )));
    }

    let engine = matches
        .get_one::<String>("engine")
        .map(|s| s.as_str())
        .unwrap_or("pdflatex");

    let output_dir = match matches.get_one::<String>("output") {
        Some(o) => {
            let p = Path::new(o);
            p.parent()
                .unwrap_or(Path::new("."))
                .to_string_lossy()
                .to_string()
        }
        None => input_path
            .parent()
            .unwrap_or(Path::new("."))
            .to_string_lossy()
            .to_string(),
    };

    let output_dir_str = if output_dir.is_empty() { "." } else { &output_dir };

    run_external_tool(engine, &["-output-directory", output_dir_str, input]).await?;

    let output_name = input_path.with_extension("pdf");
    let output_display = if output_dir_str == "." {
        output_name
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    } else {
        Path::new(output_dir_str)
            .join(output_name.file_name().unwrap_or_default())
            .to_string_lossy()
            .to_string()
    };

    println!("Compiled {} to {} using {}", input, output_display, engine);
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latex2pdf_command_parses_input() {
        let cmd = latex2pdf_command();
        let matches = cmd.try_get_matches_from(["latex2pdf", "paper.tex"]).unwrap();
        assert_eq!(matches.get_one::<String>("input").map(|s| s.as_str()), Some("paper.tex"));
    }

    #[test]
    fn test_latex2pdf_with_output() {
        let cmd = latex2pdf_command();
        let matches = cmd.try_get_matches_from(["latex2pdf", "paper.tex", "--output", "paper.pdf"]).unwrap();
        assert_eq!(matches.get_one::<String>("output").map(|s| s.as_str()), Some("paper.pdf"));
    }

    #[test]
    fn test_latex2pdf_default_engine() {
        let cmd = latex2pdf_command();
        let matches = cmd.try_get_matches_from(["latex2pdf", "paper.tex"]).unwrap();
        assert_eq!(matches.get_one::<String>("engine").map(|s| s.as_str()), Some("auto"));
    }

    #[test]
    fn test_latex2pdf_with_xelatex() {
        let cmd = latex2pdf_command();
        let matches = cmd.try_get_matches_from(["latex2pdf", "paper.tex", "--engine", "xelatex"]).unwrap();
        assert_eq!(matches.get_one::<String>("engine").map(|s| s.as_str()), Some("xelatex"));
    }

    #[test]
    fn test_latex2pdf_css_flag() {
        let cmd = latex2pdf_command();
        let matches = cmd.try_get_matches_from(["latex2pdf", "paper.tex", "--css", "custom.css"]).unwrap();
        assert_eq!(matches.get_one::<String>("css").map(|s| s.as_str()), Some("custom.css"));
    }

    #[test]
    fn test_latex2pdf_local_remote_flags() {
        let cmd = latex2pdf_command();
        let matches = cmd.try_get_matches_from(["latex2pdf", "paper.tex", "--local"]).unwrap();
        assert!(matches.get_flag("local"));

        let cmd = latex2pdf_command();
        let matches = cmd.try_get_matches_from(["latex2pdf", "paper.tex", "--remote"]).unwrap();
        assert!(matches.get_flag("remote"));
    }

    #[test]
    fn test_latex2pdf_invalid_engine_rejected() {
        let cmd = latex2pdf_command();
        let result = cmd.try_get_matches_from(["latex2pdf", "paper.tex", "--engine", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_latex2pdf_requires_input() {
        let cmd = latex2pdf_command();
        let result = cmd.try_get_matches_from(["latex2pdf"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_latex2pdf_meta() {
        let meta = latex2pdf_meta();
        assert_eq!(meta.name, "latex2pdf");
        assert_eq!(meta.category, CommandCategory::Developer);
    }

    #[test]
    fn test_latex2pdf_aliases() {
        let cmd = latex2pdf_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"tex2pdf"));
        assert!(aliases.contains(&"latexpdf"));
    }
}
