//! FShare folder scanning CLI command.
//!
//! Provides the `fshare-scan` command for listing files in FShare folders.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::fshare::{FShareClient, FShareFileEntry};
use crate::errors::{BosuaError, Result};
use crate::output;

/// Build the `fshare-scan` clap command.
pub fn fshare_scan_command() -> Command {
    Command::new("fshare-scan")
        .about("FShare folder scanning")
        .arg(
            Arg::new("url")
                .required(true)
                .help("FShare folder URL to scan"),
        )
        .arg(
            Arg::new("page")
                .long("page")
                .short('p')
                .value_parser(clap::value_parser!(u32))
                .help("Page number (1-based)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output results as JSON"),
        )
        .arg(
            Arg::new("recursive")
                .long("recursive")
                .short('r')
                .action(clap::ArgAction::SetTrue)
                .help("Recursively scan subfolders"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn fshare_scan_meta() -> CommandMeta {
    CommandBuilder::from_clap(fshare_scan_command())
        .category(CommandCategory::Cloud)
        .build()
}

/// Extract the link code from an FShare URL.
///
/// Accepts URLs like:
/// - `https://www.fshare.vn/folder/ABC123`
/// - `https://www.fshare.vn/file/ABC123`
///
/// Returns the link code (e.g. `ABC123`) or an error if the URL is invalid.
pub fn extract_link_code(url: &str) -> Result<String> {
    let trimmed = url.trim().trim_end_matches('/');

    // Match against known FShare URL patterns
    let prefixes = [
        "https://www.fshare.vn/folder/",
        "https://www.fshare.vn/file/",
        "http://www.fshare.vn/folder/",
        "http://www.fshare.vn/file/",
        "https://fshare.vn/folder/",
        "https://fshare.vn/file/",
        "http://fshare.vn/folder/",
        "http://fshare.vn/file/",
    ];

    for prefix in &prefixes {
        if let Some(code) = trimmed.strip_prefix(prefix) {
            let code = code.split('?').next().unwrap_or(code);
            let code = code.split('#').next().unwrap_or(code);
            if code.is_empty() {
                return Err(BosuaError::Command(
                    "FShare URL has no link code after the path".into(),
                ));
            }
            return Ok(code.to_string());
        }
    }

    Err(BosuaError::Command(format!(
        "Invalid FShare URL: '{}'. Expected format: https://www.fshare.vn/folder/<code> or https://www.fshare.vn/file/<code>",
        url
    )))
}

/// Format a file size in human-readable form.
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format a file type indicator.
fn format_type(entry: &FShareFileEntry) -> &str {
    match entry.file_type {
        Some(1) => "DIR ",
        Some(0) => "FILE",
        _ => "    ",
    }
}

/// Display a single file entry in a formatted line.
fn display_entry(entry: &FShareFileEntry) {
    let size_str = entry.size.map(|s| format_size(s)).unwrap_or_default();
    let modified = entry.modified.as_deref().unwrap_or("-");
    let ftype = format_type(entry);
    println!(
        "  {} {:>10}  {}  {}",
        ftype, size_str, modified, entry.name
    );
}

/// Handle the `fshare-scan` command.
pub async fn handle_fshare_scan(
    matches: &ArgMatches,
    fshare: &FShareClient,
) -> Result<()> {
    let url = matches.get_one::<String>("url").unwrap();
    let specific_page = matches.get_one::<u32>("page").copied();
    let json = matches.get_flag("json");

    let link_code = extract_link_code(url)?;

    // Collect all entries (paginate through all pages unless a specific page is requested)
    let mut all_entries: Vec<FShareFileEntry> = Vec::new();

    if let Some(page) = specific_page {
        // Fetch only the requested page
        let resp = fshare.scan_folder(&link_code, Some(page)).await?;
        if let Some(code) = resp.code {
            if code != 200 && code != 0 {
                let msg = resp.msg.unwrap_or_else(|| "unknown error".into());
                return Err(BosuaError::Cloud {
                    service: "fshare".into(),
                    message: format!("Folder scan failed: {}", msg),
                });
            }
        }
        all_entries.extend(resp.files);
    } else {
        // Paginate through all pages
        let mut current_page = 1u32;
        loop {
            let resp = fshare.scan_folder(&link_code, Some(current_page)).await?;
            if let Some(code) = resp.code {
                if code != 200 && code != 0 {
                    let msg = resp.msg.unwrap_or_else(|| "unknown error".into());
                    return Err(BosuaError::Cloud {
                        service: "fshare".into(),
                        message: format!("Folder scan failed: {}", msg),
                    });
                }
            }

            all_entries.extend(resp.files);

            // Check if there are more pages
            let last_page = resp.total_pages.unwrap_or(1);
            if current_page >= last_page {
                break;
            }
            current_page += 1;
        }
    }

    // Output results
    if json {
        let json_output = serde_json::to_string_pretty(&all_entries)
            .map_err(|e| BosuaError::Application(format!("JSON serialization error: {}", e)))?;
        println!("{}", json_output);
    } else if all_entries.is_empty() {
        output::info("Folder is empty or does not exist.");
    } else {
        output::success(&format!(
            "Found {} item(s) in folder {}:",
            all_entries.len(),
            link_code
        ));
        println!();
        println!("  TYPE {:>10}  MODIFIED             NAME", "SIZE");
        println!("  {}", "-".repeat(60));
        for entry in &all_entries {
            display_entry(entry);
        }
    }

    Ok(())
}


// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fshare_scan_command_parses() {
        let cmd = fshare_scan_command();
        let matches = cmd
            .try_get_matches_from(["fshare-scan", "https://www.fshare.vn/folder/ABC123"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("url").map(|s| s.as_str()),
            Some("https://www.fshare.vn/folder/ABC123"),
        );
    }

    #[test]
    fn test_fshare_scan_requires_url() {
        let cmd = fshare_scan_command();
        let result = cmd.try_get_matches_from(["fshare-scan"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_fshare_scan_with_options() {
        let cmd = fshare_scan_command();
        let matches = cmd
            .try_get_matches_from([
                "fshare-scan",
                "https://www.fshare.vn/folder/ABC",
                "--page",
                "2",
                "--json",
                "--recursive",
            ])
            .unwrap();
        assert_eq!(matches.get_one::<u32>("page"), Some(&2));
        assert!(matches.get_flag("json"));
        assert!(matches.get_flag("recursive"));
    }

    #[test]
    fn test_fshare_scan_meta() {
        let meta = fshare_scan_meta();
        assert_eq!(meta.name, "fshare-scan");
        assert_eq!(meta.category, CommandCategory::Cloud);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_fshare_scan_short_flags() {
        let cmd = fshare_scan_command();
        let matches = cmd
            .try_get_matches_from([
                "fshare-scan",
                "https://www.fshare.vn/folder/ABC",
                "-p",
                "5",
                "-r",
            ])
            .unwrap();
        assert_eq!(matches.get_one::<u32>("page"), Some(&5));
        assert!(matches.get_flag("recursive"));
    }

    // --- extract_link_code tests ---

    #[test]
    fn test_extract_link_code_folder_url() {
        let code = extract_link_code("https://www.fshare.vn/folder/ABC123").unwrap();
        assert_eq!(code, "ABC123");
    }

    #[test]
    fn test_extract_link_code_file_url() {
        let code = extract_link_code("https://www.fshare.vn/file/XYZ789").unwrap();
        assert_eq!(code, "XYZ789");
    }

    #[test]
    fn test_extract_link_code_trailing_slash() {
        let code = extract_link_code("https://www.fshare.vn/folder/ABC123/").unwrap();
        assert_eq!(code, "ABC123");
    }

    #[test]
    fn test_extract_link_code_with_query_params() {
        let code = extract_link_code("https://www.fshare.vn/folder/ABC123?page=2").unwrap();
        assert_eq!(code, "ABC123");
    }

    #[test]
    fn test_extract_link_code_without_www() {
        let code = extract_link_code("https://fshare.vn/folder/ABC123").unwrap();
        assert_eq!(code, "ABC123");
    }

    #[test]
    fn test_extract_link_code_invalid_url() {
        let result = extract_link_code("https://example.com/folder/ABC123");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_link_code_empty_code() {
        let result = extract_link_code("https://www.fshare.vn/folder/");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_link_code_completely_invalid() {
        let result = extract_link_code("not a url at all");
        assert!(result.is_err());
    }

    // --- format_size tests ---

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(500), "500 B");
    }

    #[test]
    fn test_format_size_kb() {
        assert_eq!(format_size(2048), "2.00 KB");
    }

    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size(5 * 1024 * 1024), "5.00 MB");
    }

    #[test]
    fn test_format_size_gb() {
        assert_eq!(format_size(3 * 1024 * 1024 * 1024), "3.00 GB");
    }
}
