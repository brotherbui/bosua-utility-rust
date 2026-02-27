//! PDF utility operations: merge, split, enhance, and compress.
//!
//! Delegates heavy lifting to external tools (e.g. `qpdf`, `ghostscript`)
//! via `tokio::process::Command`, following the same pattern as the Go
//! implementation.

use std::path::{Path, PathBuf};

use tracing;

use super::sftp::{execute_command, CommandOutput};
use crate::errors::{BosuaError, Result};

/// Options for PDF operations.
#[derive(Debug, Clone)]
pub struct PdfOptions {
    /// Output file path.
    pub output: PathBuf,
    /// Quality level for compression/enhancement (1-100).
    pub quality: u32,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            output: PathBuf::from("output.pdf"),
            quality: 75,
        }
    }
}

/// PDF utility operations.
#[derive(Debug, Clone)]
pub struct PdfUtils;

impl PdfUtils {
    /// Merge multiple PDF files into a single output file.
    ///
    /// Uses `qpdf` under the hood: `qpdf --empty --pages <inputs> -- <output>`.
    pub async fn merge(input_files: &[&Path], output: &Path) -> Result<CommandOutput> {
        if input_files.is_empty() {
            return Err(BosuaError::Command(
                "merge requires at least one input file".into(),
            ));
        }
        for f in input_files {
            if !f.exists() {
                return Err(BosuaError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("input file not found: {}", f.display()),
                )));
            }
        }

        let mut args: Vec<String> = vec!["--empty".into(), "--pages".into()];
        for f in input_files {
            args.push(f.display().to_string());
        }
        args.push("--".into());
        args.push(output.display().to_string());

        tracing::debug!(inputs = input_files.len(), output = %output.display(), "pdf merge");
        execute_command("qpdf", &args).await
    }

    /// Split a PDF file by extracting specific pages.
    ///
    /// `page_range` follows qpdf syntax, e.g. "1-5", "1,3,5", "r1" (last page).
    pub async fn split(
        input: &Path,
        output: &Path,
        page_range: &str,
    ) -> Result<CommandOutput> {
        if !input.exists() {
            return Err(BosuaError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("input file not found: {}", input.display()),
            )));
        }

        let args: Vec<String> = vec![
            input.display().to_string(),
            "--pages".into(),
            ".".into(),
            page_range.into(),
            "--".into(),
            output.display().to_string(),
        ];

        tracing::debug!(input = %input.display(), pages = page_range, "pdf split");
        execute_command("qpdf", &args).await
    }

    /// Enhance a PDF (e.g. linearize for fast web view, repair).
    pub async fn enhance(input: &Path, output: &Path) -> Result<CommandOutput> {
        if !input.exists() {
            return Err(BosuaError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("input file not found: {}", input.display()),
            )));
        }

        let args: Vec<String> = vec![
            input.display().to_string(),
            "--linearize".into(),
            output.display().to_string(),
        ];

        tracing::debug!(input = %input.display(), "pdf enhance");
        execute_command("qpdf", &args).await
    }

    /// Compress a PDF using Ghostscript to reduce file size.
    ///
    /// Maps quality 1-100 to Ghostscript PDF settings:
    /// - 1-25: `/screen` (lowest quality, smallest size)
    /// - 26-50: `/ebook`
    /// - 51-75: `/printer`
    /// - 76-100: `/prepress` (highest quality, largest size)
    pub async fn compress(
        input: &Path,
        output: &Path,
        quality: u32,
    ) -> Result<CommandOutput> {
        if !input.exists() {
            return Err(BosuaError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("input file not found: {}", input.display()),
            )));
        }

        let setting = match quality {
            0..=25 => "/screen",
            26..=50 => "/ebook",
            51..=75 => "/printer",
            _ => "/prepress",
        };

        let args: Vec<String> = vec![
            "-sDEVICE=pdfwrite".into(),
            "-dCompatibilityLevel=1.4".into(),
            format!("-dPDFSETTINGS={}", setting),
            "-dNOPAUSE".into(),
            "-dQUIET".into(),
            "-dBATCH".into(),
            format!("-sOutputFile={}", output.display()),
            input.display().to_string(),
        ];

        tracing::debug!(input = %input.display(), quality, setting, "pdf compress");
        execute_command("gs", &args).await
    }

    /// Map a quality value (1-100) to a Ghostscript PDF setting name.
    pub fn quality_to_setting(quality: u32) -> &'static str {
        match quality {
            0..=25 => "/screen",
            26..=50 => "/ebook",
            51..=75 => "/printer",
            _ => "/prepress",
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_options_default() {
        let opts = PdfOptions::default();
        assert_eq!(opts.output, PathBuf::from("output.pdf"));
        assert_eq!(opts.quality, 75);
    }

    #[test]
    fn quality_to_setting_screen() {
        assert_eq!(PdfUtils::quality_to_setting(0), "/screen");
        assert_eq!(PdfUtils::quality_to_setting(10), "/screen");
        assert_eq!(PdfUtils::quality_to_setting(25), "/screen");
    }

    #[test]
    fn quality_to_setting_ebook() {
        assert_eq!(PdfUtils::quality_to_setting(26), "/ebook");
        assert_eq!(PdfUtils::quality_to_setting(50), "/ebook");
    }

    #[test]
    fn quality_to_setting_printer() {
        assert_eq!(PdfUtils::quality_to_setting(51), "/printer");
        assert_eq!(PdfUtils::quality_to_setting(75), "/printer");
    }

    #[test]
    fn quality_to_setting_prepress() {
        assert_eq!(PdfUtils::quality_to_setting(76), "/prepress");
        assert_eq!(PdfUtils::quality_to_setting(100), "/prepress");
    }

    #[tokio::test]
    async fn merge_empty_inputs_returns_error() {
        let result = PdfUtils::merge(&[], Path::new("out.pdf")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn merge_nonexistent_input_returns_error() {
        let result =
            PdfUtils::merge(&[Path::new("/nonexistent/a.pdf")], Path::new("out.pdf")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn split_nonexistent_input_returns_error() {
        let result =
            PdfUtils::split(Path::new("/nonexistent/a.pdf"), Path::new("out.pdf"), "1-5").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn enhance_nonexistent_input_returns_error() {
        let result =
            PdfUtils::enhance(Path::new("/nonexistent/a.pdf"), Path::new("out.pdf")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn compress_nonexistent_input_returns_error() {
        let result =
            PdfUtils::compress(Path::new("/nonexistent/a.pdf"), Path::new("out.pdf"), 50).await;
        assert!(result.is_err());
    }
}
