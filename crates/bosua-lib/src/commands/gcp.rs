//! GCP CLI command with subcommands.
//!
//! Provides the `gcp` command with subcommands: browse, download, list, play, push.

use std::path::Path;

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::gcp::GcpClient;
use crate::config::dynamic::DynamicConfig;
use crate::errors::Result;
use crate::http_client::HttpClient;
use crate::output;

/// Build the `gcp` clap command with all subcommands.
pub fn gcp_command() -> Command {
    Command::new("gcp")
        .about("GCP stuffs")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(Arg::new("ip").long("ip").help("GCP server IP address (overrides GCP_IP environment variable)"))
        .subcommand(browse_subcommand())
        .subcommand(download_subcommand())
        .subcommand(list_subcommand())
        .subcommand(play_subcommand())
        .subcommand(push_subcommand())
}

/// Build the `CommandMeta` for registry registration.
pub fn gcp_meta() -> CommandMeta {
    CommandBuilder::from_clap(gcp_command())
        .category(CommandCategory::Cloud)
        .build()
}

/// Handle the `gcp` command dispatch.
pub async fn handle_gcp(
    matches: &ArgMatches,
    config: &DynamicConfig,
    http: &HttpClient,
) -> Result<()> {
    let client = GcpClient::new(http.clone(), config);

    match matches.subcommand() {
        Some(("browse", sub)) => handle_browse(sub, &client).await,
        Some(("download", sub)) => handle_download(sub, &client).await,
        Some(("list", sub)) => handle_list(sub, &client).await,
        Some(("play", sub)) => handle_play(sub, &client).await,
        Some(("push", sub)) => handle_push(sub, &client).await,
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// Subcommand definitions
// ---------------------------------------------------------------------------

fn browse_subcommand() -> Command {
    Command::new("browse")
        .about("Interactive remote file browser (type 'help' for commands)")
        .arg(Arg::new("host").long("host").default_value("localhost").help("Host"))
        .arg(Arg::new("pass").long("pass").default_value("conchimnon").help("Password"))
        .arg(Arg::new("path").long("path").short('p').help("Start path"))
        .arg(Arg::new("player").long("player").default_value("kodi").help("Player name"))
        .arg(Arg::new("port").long("port").default_value("6868").help("Port"))
        .arg(Arg::new("user").long("user").short('u').default_value("kodi").help("User"))
}

fn download_subcommand() -> Command {
    Command::new("download")
        .about("Download files from remote server with resumable support")
        .aliases(["dl", "d"])
        .arg(Arg::new("file_path_or_pattern").num_args(0..).help("File path or pattern"))
}

fn list_subcommand() -> Command {
    Command::new("list")
        .about("List remote files from server (/list-files)")
        .aliases(["ls"])
        .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON for machine-readable parsing"))
        .arg(Arg::new("path").long("path").help("Remote subdirectory to list (relative to Downloads)"))
}

fn play_subcommand() -> Command {
    Command::new("play")
        .about("Play remote files using VLC or Kodi")
        .arg(Arg::new("file_pattern").help("File pattern"))
        .arg(Arg::new("host").long("host").default_value("localhost").help("Host"))
        .arg(Arg::new("pass").long("pass").default_value("conchimnon").help("Password"))
        .arg(Arg::new("player").long("player").default_value("kodi").help("Player name"))
        .arg(Arg::new("port").long("port").default_value("6868").help("Port"))
        .arg(Arg::new("user").long("user").short('u').default_value("kodi").help("User"))
        .subcommand(
            Command::new("all")
                .about("Play all videos from directories matching the search terms")
                .arg(Arg::new("terms").num_args(0..).help("Search terms")),
        )
        .subcommand(
            Command::new("file")
                .about("Play a single remote file by matching folder and file names")
                .arg(Arg::new("terms").num_args(0..).help("folder_term file_term")),
        )
}

fn push_subcommand() -> Command {
    Command::new("push")
        .about("Push links/files to GCP remote")
        .arg(Arg::new("folder").long("folder").help("Download folder name"))
        .arg(Arg::new("gdriveid").long("gdriveid").help("Gdrive ID"))
        .arg(Arg::new("path").long("path").help("Remote path (default to Downloads directory)"))
        .subcommand(Command::new("file").about("Push files to GCP remote"))
        .subcommand(Command::new("folder-scan").about("Push Fshare folders to remote for scanning"))
        .subcommand(Command::new("link").about("Push links to GCP remote"))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn handle_browse(matches: &ArgMatches, client: &GcpClient) -> Result<()> {
    let path = matches.get_one::<String>("path").map(|s| s.as_str());
    let files = client.browse(path).await?;

    if files.is_empty() {
        output::info("No files found.");
        return Ok(());
    }

    for f in &files {
        let kind = if f.is_directory() { "DIR " } else { "FILE" };
        let size_str = f
            .size
            .map(|s| format!("{s}"))
            .unwrap_or_else(|| "-".into());
        println!("{kind}  {size_str:>10}  {}", f.name);
    }
    Ok(())
}

async fn handle_download(matches: &ArgMatches, client: &GcpClient) -> Result<()> {
    let remote = matches.get_one::<String>("remote-path").unwrap();
    let local = matches
        .get_one::<String>("output")
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            Path::new(remote)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "download".into())
        });

    output::info(&format!("Downloading {remote} -> {local}"));
    client.download_file(remote, Path::new(&local)).await?;
    output::success(&format!("Downloaded {remote} to {local}"));
    Ok(())
}

async fn handle_list(matches: &ArgMatches, client: &GcpClient) -> Result<()> {
    let path = matches.get_one::<String>("path").map(|s| s.as_str());
    let file_list = client.list_files(path).await?;

    if file_list.files.is_empty() {
        output::info("No files found.");
        return Ok(());
    }

    println!("Path: {}", file_list.path);
    for f in &file_list.files {
        let kind = if f.is_directory() { "DIR " } else { "FILE" };
        let size_str = f
            .size
            .map(|s| format!("{s}"))
            .unwrap_or_else(|| "-".into());
        println!("{kind}  {size_str:>10}  {}", f.name);
    }
    Ok(())
}

async fn handle_play(matches: &ArgMatches, client: &GcpClient) -> Result<()> {
    let remote = matches.get_one::<String>("remote-path").unwrap();
    let player = matches.get_one::<String>("player").map(|s| s.as_str());

    output::info(&format!("Playing {remote} via {}", player.unwrap_or("mpv")));
    let message = client.play(remote, player).await?;
    output::success(&message);
    Ok(())
}

async fn handle_push(matches: &ArgMatches, client: &GcpClient) -> Result<()> {
    let local = matches.get_one::<String>("local-path").unwrap();
    let remote = matches.get_one::<String>("remote-path").unwrap();

    output::info(&format!("Pushing {local} -> {remote}"));
    let result = client.push_file(Path::new(local), remote).await?;

    if result.success {
        output::success(&format!("Pushed to {}", result.remote_path));
    } else {
        output::warning(&format!("Push completed with message: {}", result.message));
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
    fn test_gcp_command_parses() {
        let cmd = gcp_command();
        let matches = cmd.try_get_matches_from(["gcp", "list"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("list"));
    }

    #[test]
    fn test_gcp_requires_subcommand() {
        let cmd = gcp_command();
        let result = cmd.try_get_matches_from(["gcp"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_gcp_ip_flag() {
        let cmd = gcp_command();
        let matches = cmd.try_get_matches_from(["gcp", "--ip", "10.0.0.1", "list"]).unwrap();
        assert_eq!(matches.get_one::<String>("ip").map(|s| s.as_str()), Some("10.0.0.1"));
    }

    #[test]
    fn test_gcp_browse_with_kodi_flags() {
        let cmd = gcp_command();
        let matches = cmd
            .try_get_matches_from(["gcp", "browse", "--host", "192.168.1.1", "--port", "9090"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "browse");
        assert_eq!(sub.get_one::<String>("host").map(|s| s.as_str()), Some("192.168.1.1"));
        assert_eq!(sub.get_one::<String>("port").map(|s| s.as_str()), Some("9090"));
    }

    #[test]
    fn test_gcp_download_alias() {
        let cmd = gcp_command();
        let matches = cmd.try_get_matches_from(["gcp", "dl"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("download"));
    }

    #[test]
    fn test_gcp_list_json_flag() {
        let cmd = gcp_command();
        let matches = cmd.try_get_matches_from(["gcp", "list", "--json"]).unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert!(sub.get_flag("json"));
    }

    #[test]
    fn test_gcp_list_path_flag() {
        let cmd = gcp_command();
        let matches = cmd.try_get_matches_from(["gcp", "list", "--path", "movies"]).unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(sub.get_one::<String>("path").map(|s| s.as_str()), Some("movies"));
    }

    #[test]
    fn test_gcp_play_subcommands() {
        let cmd = gcp_command();
        let matches = cmd.try_get_matches_from(["gcp", "play", "all", "movie"]).unwrap();
        let (_, play_sub) = matches.subcommand().unwrap();
        assert_eq!(play_sub.subcommand_name(), Some("all"));
    }

    #[test]
    fn test_gcp_push_subcommands() {
        let cmd = gcp_command();
        let matches = cmd.try_get_matches_from(["gcp", "push", "link"]).unwrap();
        let (_, push_sub) = matches.subcommand().unwrap();
        assert_eq!(push_sub.subcommand_name(), Some("link"));
    }

    #[test]
    fn test_gcp_meta() {
        let meta = gcp_meta();
        assert_eq!(meta.name, "gcp");
        assert_eq!(meta.category, CommandCategory::Cloud);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = gcp_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"browse"));
        assert!(sub_names.contains(&"download"));
        assert!(sub_names.contains(&"list"));
        assert!(sub_names.contains(&"play"));
        assert!(sub_names.contains(&"push"));
        assert_eq!(sub_names.len(), 5);
    }
}
