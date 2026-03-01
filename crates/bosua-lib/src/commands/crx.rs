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

    println!("Converting extension (team: {})", team);
    if archive { println!("  Will archive to application"); }
    if install { println!("  Will install after export"); }
    if remove { println!("  Will remove source after convert"); }

    // TODO: implement CRX to Xcode project conversion
    println!("crx convert: not yet implemented");
    Ok(())
}

async fn handle_download(matches: &ArgMatches, http: &HttpClient) -> Result<()> {
    let extract = matches.get_flag("extract");
    let convert = matches.get_flag("convert");
    let archive = matches.get_flag("archive");
    let install = matches.get_flag("install");
    let _team = matches.get_one::<String>("team").unwrap();

    // TODO: prompt for extension ID or accept as arg
    println!("crx download: not yet fully implemented");
    if extract { println!("  Will extract after download"); }
    if convert { println!("  Will convert after extract"); }
    if archive { println!("  Will archive to application"); }
    if install { println!("  Will install after export"); }

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
