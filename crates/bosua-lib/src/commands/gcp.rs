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
        .about("Google Cloud Platform operations")
        .subcommand_required(true)
        .arg_required_else_help(true)
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
        .about("Browse files on a GCP VM")
        .arg(
            Arg::new("path")
                .long("path")
                .short('p')
                .help("Remote path to browse (defaults to home directory)"),
        )
}

fn download_subcommand() -> Command {
    Command::new("download")
        .about("Download a file from a GCP VM")
        .arg(
            Arg::new("remote-path")
                .required(true)
                .help("Remote file path to download"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Local output file path"),
        )
}

fn list_subcommand() -> Command {
    Command::new("list")
        .about("List files on a GCP VM")
        .arg(
            Arg::new("path")
                .long("path")
                .short('p')
                .help("Remote path to list (defaults to home directory)"),
        )
}

fn play_subcommand() -> Command {
    Command::new("play")
        .about("Play media from a GCP VM")
        .arg(
            Arg::new("remote-path")
                .required(true)
                .help("Remote file path to play"),
        )
        .arg(
            Arg::new("player")
                .long("player")
                .default_value("mpv")
                .help("Media player to use"),
        )
}

fn push_subcommand() -> Command {
    Command::new("push")
        .about("Push (upload) a file to a GCP VM")
        .arg(
            Arg::new("local-path")
                .required(true)
                .help("Local file path to upload"),
        )
        .arg(
            Arg::new("remote-path")
                .required(true)
                .help("Remote destination path"),
        )
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
        let kind = if f.is_dir { "DIR " } else { "FILE" };
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
        let kind = if f.is_dir { "DIR " } else { "FILE" };
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
    fn test_gcp_browse_with_path() {
        let cmd = gcp_command();
        let matches = cmd
            .try_get_matches_from(["gcp", "browse", "--path", "/home/user"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "browse");
        assert_eq!(
            sub.get_one::<String>("path").map(|s| s.as_str()),
            Some("/home/user"),
        );
    }

    #[test]
    fn test_gcp_download_subcommand() {
        let cmd = gcp_command();
        let matches = cmd
            .try_get_matches_from(["gcp", "download", "/remote/file.txt", "-o", "/local/file.txt"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(
            sub.get_one::<String>("remote-path").map(|s| s.as_str()),
            Some("/remote/file.txt"),
        );
        assert_eq!(
            sub.get_one::<String>("output").map(|s| s.as_str()),
            Some("/local/file.txt"),
        );
    }

    #[test]
    fn test_gcp_play_subcommand() {
        let cmd = gcp_command();
        let matches = cmd
            .try_get_matches_from(["gcp", "play", "/media/video.mp4", "--player", "vlc"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(
            sub.get_one::<String>("remote-path").map(|s| s.as_str()),
            Some("/media/video.mp4"),
        );
        assert_eq!(
            sub.get_one::<String>("player").map(|s| s.as_str()),
            Some("vlc"),
        );
    }

    #[test]
    fn test_gcp_push_subcommand() {
        let cmd = gcp_command();
        let matches = cmd
            .try_get_matches_from(["gcp", "push", "/local/file.txt", "/remote/file.txt"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(
            sub.get_one::<String>("local-path").map(|s| s.as_str()),
            Some("/local/file.txt"),
        );
        assert_eq!(
            sub.get_one::<String>("remote-path").map(|s| s.as_str()),
            Some("/remote/file.txt"),
        );
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
