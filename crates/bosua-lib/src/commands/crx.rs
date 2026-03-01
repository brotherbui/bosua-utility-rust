//! CRX CLI command — Chrome CRX extension operations.
//!
//! Subcommands: convert, download.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};
use crate::http_client::HttpClient;

/// Chrome Web Store CRX download URL template.
const CRX_DOWNLOAD_URL: &str = "https://clients2.google.com/service/update2/crx?response=redirect&acceptformat=crx2,crx3&prodversion=120.0&x=id%3D{EXT_ID}%26installsource%3Dondemand%26uc";

/// Build the `crx` clap command.
pub fn crx_command() -> Command {
    Command::new("crx")
        .about("Chrome CRX extension operations")
        .aliases(["chrome"])
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("convert")
                .about("Convert extension")
                .aliases(["cv", "c"])
                .arg(Arg::new("archive").long("archive").action(clap::ArgAction::SetTrue).help("Export xcode project to application"))
                .arg(Arg::new("install").long("install").action(clap::ArgAction::SetTrue).help("Install after export"))
                .arg(Arg::new("remove").long("remove").action(clap::ArgAction::SetTrue).help("Remove the source code after convert"))
                .arg(Arg::new("team").long("team").default_value("Z6322AZ9HJ").help("Developer Account Team ID")),
        )
        .subcommand(
            Command::new("download")
                .about("Download extension")
                .aliases(["dl", "d"])
                .arg(Arg::new("archive").long("archive").action(clap::ArgAction::SetTrue).help("Export xcode project to application"))
                .arg(Arg::new("convert").long("convert").action(clap::ArgAction::SetTrue).help("Convert after extract"))
                .arg(Arg::new("extract").long("extract").action(clap::ArgAction::SetTrue).help("Extract after download"))
                .arg(Arg::new("install").long("install").action(clap::ArgAction::SetTrue).help("Install after export"))
                .arg(Arg::new("team").long("team").default_value("Z6322AZ9HJ").help("Developer Account Team ID")),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn crx_meta() -> CommandMeta {
    CommandBuilder::from_clap(crx_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `crx` command.
pub async fn handle_crx(matches: &ArgMatches, http: &HttpClient) -> Result<()> {
    match matches.subcommand() {
        Some(("convert", sub)) => handle_convert(sub).await,
        Some(("download", sub)) => handle_download(sub, http).await,
        _ => unreachable!("subcommand_required is set"),
    }
}

async fn handle_convert(matches: &ArgMatches) -> Result<()> {
    let archive = matches.get_flag("archive");
    let install = matches.get_flag("install");
    let remove = matches.get_flag("remove");
    let team = matches.get_one::<String>("team").unwrap();

    // Go passes trailing args as file paths to convert
    // In Rust CLI we don't have trailing args, so delegate to Go binary
    let go_bin = "/opt/homebrew/bin/bosua";
    if !std::path::Path::new(go_bin).exists() {
        return Err(BosuaError::Command(
            "CRX convert requires the Go binary at /opt/homebrew/bin/bosua (xcrun/xcodebuild integration not yet ported)".into(),
        ));
    }

    let mut args = vec!["crx".to_string(), "convert".to_string()];
    args.push(format!("--team={}", team));
    if archive { args.push("--archive".to_string()); }
    if install { args.push("--install".to_string()); }
    if remove { args.push("--remove".to_string()); }

    let output = tokio::process::Command::new(go_bin)
        .args(&args)
        .output()
        .await
        .map_err(|e| BosuaError::Command(format!("Failed to run Go binary: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() { print!("{}", stdout); }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() { eprint!("{}", stderr); }
    Ok(())
}

async fn handle_download(matches: &ArgMatches, http: &HttpClient) -> Result<()> {
    let extract = matches.get_flag("extract");
    let convert = matches.get_flag("convert");
    let archive = matches.get_flag("archive");
    let install = matches.get_flag("install");
    let team = matches.get_one::<String>("team").unwrap();

    // Delegate to Go binary which has full CRX download/extract/convert pipeline
    let go_bin = "/opt/homebrew/bin/bosua";
    if !std::path::Path::new(go_bin).exists() {
        return Err(BosuaError::Command(
            "CRX download requires the Go binary at /opt/homebrew/bin/bosua".into(),
        ));
    }

    let mut args = vec!["crx".to_string(), "download".to_string()];
    args.push(format!("--team={}", team));
    if extract { args.push("--extract".to_string()); }
    if convert { args.push("--convert".to_string()); }
    if archive { args.push("--archive".to_string()); }
    if install { args.push("--install".to_string()); }

    let output = tokio::process::Command::new(go_bin)
        .args(&args)
        .output()
        .await
        .map_err(|e| BosuaError::Command(format!("Failed to run Go binary: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() { print!("{}", stdout); }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() { eprint!("{}", stderr); }

    let _ = http;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crx_command_parses() {
        let cmd = crx_command();
        let m = cmd.try_get_matches_from(["crx", "download"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("download"));
    }

    #[test]
    fn test_crx_alias_chrome() {
        let cmd = crx_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"chrome"));
    }

    #[test]
    fn test_crx_convert_flags() {
        let cmd = crx_command();
        let m = cmd.try_get_matches_from(["crx", "convert", "--archive", "--install", "--remove"]).unwrap();
        let (name, sub) = m.subcommand().unwrap();
        assert_eq!(name, "convert");
        assert!(sub.get_flag("archive"));
        assert!(sub.get_flag("install"));
        assert!(sub.get_flag("remove"));
    }

    #[test]
    fn test_crx_download_flags() {
        let cmd = crx_command();
        let m = cmd.try_get_matches_from(["crx", "download", "--extract", "--convert"]).unwrap();
        let (name, sub) = m.subcommand().unwrap();
        assert_eq!(name, "download");
        assert!(sub.get_flag("extract"));
        assert!(sub.get_flag("convert"));
    }

    #[test]
    fn test_crx_requires_subcommand() {
        let cmd = crx_command();
        assert!(cmd.try_get_matches_from(["crx"]).is_err());
    }

    #[test]
    fn test_crx_meta() {
        let meta = crx_meta();
        assert_eq!(meta.name, "crx");
        assert_eq!(meta.category, CommandCategory::Utility);
    }
}
