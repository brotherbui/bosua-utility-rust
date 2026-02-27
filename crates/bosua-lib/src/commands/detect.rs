//! Detect CLI command — file type detection.
//!
//! Args: file.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};

/// Build the `detect` clap command.
pub fn detect_command() -> Command {
    Command::new("detect")
        .about("File type detection")
        .arg(
            Arg::new("file")
                .required(true)
                .help("File to detect type for"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn detect_meta() -> CommandMeta {
    CommandBuilder::from_clap(detect_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `detect` command.
/// Detect MIME type from magic bytes (file header).
fn detect_from_magic(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() >= 4 {
        // PDF: %PDF
        if bytes.starts_with(&[0x25, 0x50, 0x44, 0x46]) {
            return Some("application/pdf");
        }
        // PNG: 89 50 4E 47
        if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            return Some("image/png");
        }
        // GIF: GIF8
        if bytes.starts_with(&[0x47, 0x49, 0x46, 0x38]) {
            return Some("image/gif");
        }
        // ZIP/DOCX/XLSX: PK\x03\x04
        if bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
            return Some("application/zip");
        }
        // WASM: \0asm
        if bytes.starts_with(&[0x00, 0x61, 0x73, 0x6D]) {
            return Some("application/wasm");
        }
    }
    if bytes.len() >= 3 {
        // JPEG: FF D8 FF
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Some("image/jpeg");
        }
    }
    if bytes.len() >= 6 {
        // TIFF (little-endian): II*\0
        if bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00]) {
            return Some("image/tiff");
        }
        // TIFF (big-endian): MM\0*
        if bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A]) {
            return Some("image/tiff");
        }
    }
    if bytes.len() >= 8 {
        // WebP: RIFF....WEBP
        if bytes.starts_with(b"RIFF") && bytes[8..].starts_with(b"WEBP") {
            return Some("image/webp");
        }
    }
    if bytes.len() >= 12 {
        // MP4/MOV: ftyp at offset 4
        if &bytes[4..8] == b"ftyp" {
            return Some("video/mp4");
        }
    }
    None
}

/// Detect MIME type from file extension as a fallback.
fn detect_from_extension(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref() {
        Some("txt") => "text/plain",
        Some("html") | Some("htm") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("json") => "application/json",
        Some("xml") => "application/xml",
        Some("csv") => "text/csv",
        Some("md") => "text/markdown",
        Some("pdf") => "application/pdf",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("avi") => "video/x-msvideo",
        Some("mkv") => "video/x-matroska",
        Some("zip") => "application/zip",
        Some("gz") | Some("gzip") => "application/gzip",
        Some("tar") => "application/x-tar",
        Some("rar") => "application/x-rar-compressed",
        Some("7z") => "application/x-7z-compressed",
        Some("doc") => "application/msword",
        Some("docx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        Some("xls") => "application/vnd.ms-excel",
        Some("xlsx") => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        Some("ppt") => "application/vnd.ms-powerpoint",
        Some("pptx") => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        Some("rs") => "text/x-rust",
        Some("py") => "text/x-python",
        Some("go") => "text/x-go",
        Some("toml") => "application/toml",
        Some("yaml") | Some("yml") => "application/yaml",
        Some("sh") => "application/x-sh",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

/// Handle the `detect` command.
pub fn handle_detect(matches: &ArgMatches) -> Result<()> {
    let file = matches.get_one::<String>("file").unwrap();
    let path = std::path::Path::new(file);

    if !path.exists() {
        return Err(BosuaError::Command(format!("File not found: {}", file)));
    }

    // Read first 16 bytes for magic byte detection
    let bytes = std::fs::read(path).map_err(|e| {
        BosuaError::Command(format!("Failed to read file '{}': {}", file, e))
    })?;

    let mime = detect_from_magic(&bytes).unwrap_or_else(|| detect_from_extension(path));

    println!("{}: {}", file, mime);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_command_parses() {
        let cmd = detect_command();
        let matches = cmd.try_get_matches_from(["detect", "myfile.bin"]).unwrap();
        assert_eq!(
            matches.get_one::<String>("file").map(|s| s.as_str()),
            Some("myfile.bin"),
        );
    }

    #[test]
    fn test_detect_requires_file() {
        let cmd = detect_command();
        let result = cmd.try_get_matches_from(["detect"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_meta() {
        let meta = detect_meta();
        assert_eq!(meta.name, "detect");
        assert_eq!(meta.category, CommandCategory::Utility);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_detect_from_magic_pdf() {
        let bytes = vec![0x25, 0x50, 0x44, 0x46, 0x2D, 0x31, 0x2E, 0x34];
        assert_eq!(detect_from_magic(&bytes), Some("application/pdf"));
    }

    #[test]
    fn test_detect_from_magic_png() {
        let bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_from_magic(&bytes), Some("image/png"));
    }

    #[test]
    fn test_detect_from_magic_jpeg() {
        let bytes = vec![0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(detect_from_magic(&bytes), Some("image/jpeg"));
    }

    #[test]
    fn test_detect_from_magic_gif() {
        let bytes = vec![0x47, 0x49, 0x46, 0x38, 0x39, 0x61];
        assert_eq!(detect_from_magic(&bytes), Some("image/gif"));
    }

    #[test]
    fn test_detect_from_magic_zip() {
        let bytes = vec![0x50, 0x4B, 0x03, 0x04];
        assert_eq!(detect_from_magic(&bytes), Some("application/zip"));
    }

    #[test]
    fn test_detect_from_magic_unknown() {
        let bytes = vec![0x00, 0x00, 0x00, 0x00];
        assert_eq!(detect_from_magic(&bytes), None);
    }

    #[test]
    fn test_detect_from_extension_known() {
        assert_eq!(detect_from_extension(std::path::Path::new("file.json")), "application/json");
        assert_eq!(detect_from_extension(std::path::Path::new("file.rs")), "text/x-rust");
        assert_eq!(detect_from_extension(std::path::Path::new("file.HTML")), "text/html");
    }

    #[test]
    fn test_detect_from_extension_unknown() {
        assert_eq!(detect_from_extension(std::path::Path::new("file.xyz")), "application/octet-stream");
        assert_eq!(detect_from_extension(std::path::Path::new("noext")), "application/octet-stream");
    }
}
