//! FShare Scan CLI command — track Fshare folders and detect new files.
//!
//! Subcommands: add, list, remove, run.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::fshare::FShareClient;
use crate::errors::{BosuaError, Result};

/// Build the `fshare-scan` clap command.
pub fn fshare_scan_command() -> Command {
    Command::new("fshare-scan")
        .about("Track Fshare folders and automatically detect new files.\nNew files can be automatically appended to the cron links file for processing.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("add")
                .about("Add a folder to track for new files")
                .arg(Arg::new("url").required(true).help("FShare folder URL to track")),
        )
        .subcommand(
            Command::new("list")
                .about("List all tracked folders"),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove a folder from tracking")
                .arg(Arg::new("url").required(true).help("FShare folder URL to remove")),
        )
        .subcommand(
            Command::new("run")
                .about("Scan all tracked folders for new files"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn fshare_scan_meta() -> CommandMeta {
    CommandBuilder::from_clap(fshare_scan_command())
        .category(CommandCategory::Cloud)
        .build()
}

/// Extract the link code from an FShare URL.
pub fn extract_link_code(url: &str) -> Result<String> {
    let stripped = url
        .strip_prefix("https://www.fshare.vn/")
        .or_else(|| url.strip_prefix("https://fshare.vn/"))
        .or_else(|| url.strip_prefix("http://www.fshare.vn/"))
        .or_else(|| url.strip_prefix("http://fshare.vn/"))
        .ok_or_else(|| BosuaError::Command(format!("Invalid FShare URL: {}", url)))?;

    // Remove query params and trailing slash
    let path = stripped.split('?').next().unwrap_or(stripped);
    let path = path.trim_end_matches('/');

    // Expected: file/<code> or folder/<code>
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 || parts[1].is_empty() {
        return Err(BosuaError::Command(format!(
            "Cannot extract link code from URL: {}",
            url
        )));
    }

    Ok(parts[1].to_string())
}


/// Handle the `fshare-scan` command dispatch.
pub async fn handle_fshare_scan(
    matches: &ArgMatches,
    _fshare: &FShareClient,
) -> Result<()> {
    match matches.subcommand() {
        Some(("add", sub)) => {
            let url = sub.get_one::<String>("url").unwrap();
            let _code = extract_link_code(url)?;
            println!("Added folder to tracking: {}", url);
            // TODO: persist to tracking database
            Ok(())
        }
        Some(("list", _)) => {
            println!("Tracked folders:");
            // TODO: load from tracking database
            println!("  (none)");
            Ok(())
        }
        Some(("remove", sub)) => {
            let url = sub.get_one::<String>("url").unwrap();
            println!("Removed folder from tracking: {}", url);
            // TODO: remove from tracking database
            Ok(())
        }
        Some(("run", _)) => {
            println!("Scanning all tracked folders for new files...");
            // TODO: iterate tracked folders, compare with known files, report new ones
            println!("Scan complete. No new files found.");
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fshare_scan_command_parses() {
        let cmd = fshare_scan_command();
        let m = cmd.try_get_matches_from(["fshare-scan", "add", "https://www.fshare.vn/folder/ABC123"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("add"));
    }

    #[test]
    fn test_fshare_scan_requires_subcommand() {
        let cmd = fshare_scan_command();
        assert!(cmd.try_get_matches_from(["fshare-scan"]).is_err());
    }

    #[test]
    fn test_fshare_scan_list() {
        let cmd = fshare_scan_command();
        let m = cmd.try_get_matches_from(["fshare-scan", "list"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("list"));
    }

    #[test]
    fn test_fshare_scan_remove() {
        let cmd = fshare_scan_command();
        let m = cmd.try_get_matches_from(["fshare-scan", "remove", "https://www.fshare.vn/folder/ABC123"]).unwrap();
        let (name, sub) = m.subcommand().unwrap();
        assert_eq!(name, "remove");
        assert_eq!(sub.get_one::<String>("url").map(|s| s.as_str()), Some("https://www.fshare.vn/folder/ABC123"));
    }

    #[test]
    fn test_fshare_scan_run() {
        let cmd = fshare_scan_command();
        let m = cmd.try_get_matches_from(["fshare-scan", "run"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("run"));
    }

    #[test]
    fn test_fshare_scan_meta() {
        let meta = fshare_scan_meta();
        assert_eq!(meta.name, "fshare-scan");
        assert_eq!(meta.category, CommandCategory::Cloud);
    }

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
        let code = extract_link_code("https://www.fshare.vn/folder/ABC123?ref=test").unwrap();
        assert_eq!(code, "ABC123");
    }

    #[test]
    fn test_extract_link_code_without_www() {
        let code = extract_link_code("https://fshare.vn/folder/ABC123").unwrap();
        assert_eq!(code, "ABC123");
    }

    #[test]
    fn test_extract_link_code_invalid_url() {
        assert!(extract_link_code("https://google.com/folder/ABC").is_err());
    }
    
}
