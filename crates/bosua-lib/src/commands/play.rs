//! Play CLI command for media playback from various sources.
//!
//! Supports direct URLs, FShare links, remote storage, and Kodi integration.
//! Kodi credentials are read from DynamicConfig (KodiUsername, KodiPassword).

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::commands::registry_cmd::ServiceRegistry;
use crate::errors::{BosuaError, Result};
use crate::output;

/// Supported source types for media playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Url,
    Fshare,
    Gdrive,
    Gcp,
    Kodi,
}

impl SourceType {
    pub fn from_str_value(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "url" => Some(Self::Url),
            "fshare" => Some(Self::Fshare),
            "gdrive" => Some(Self::Gdrive),
            "gcp" => Some(Self::Gcp),
            "kodi" => Some(Self::Kodi),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Url => "url",
            Self::Fshare => "fshare",
            Self::Gdrive => "gdrive",
            Self::Gcp => "gcp",
            Self::Kodi => "kodi",
        }
    }
}

/// Build the `play` clap command.
pub fn play_command() -> Command {
    Command::new("play")
        .about("Play operations")
        .arg(Arg::new("source").help("URL or episode pattern"))
        .arg(Arg::new("auto-discover").long("auto-discover").action(clap::ArgAction::SetTrue).help("Automatically discover Kodi instances on the local network"))
        .arg(Arg::new("host").long("host").default_value("localhost").help("Host"))
        .arg(Arg::new("pass").long("pass").default_value("conchimnon").help("Password"))
        .arg(Arg::new("player").long("player").default_value("kodi").help("Player name"))
        .arg(Arg::new("port").long("port").default_value("6868").help("Port"))
        .arg(Arg::new("user").long("user").short('u').default_value("kodi").help("User"))
        .subcommand(
            Command::new("all")
                .about("Play all videos from directories matching the search terms")
                .arg(Arg::new("terms").num_args(0..).help("Search terms"))
                .arg(Arg::new("path").long("path").help("Remote subdirectory containing videos (overrides search behavior)")),
        )
        .subcommand(
            Command::new("file")
                .about("Play a single remote file by matching folder and file names")
                .arg(Arg::new("terms").num_args(0..).help("folder_term file_term"))
                .arg(Arg::new("path").long("path").help("Remote file relative path (overrides search behavior)")),
        )
        .subcommand(
            Command::new("scan")
                .about("Scan local network for Kodi instances"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn play_meta() -> CommandMeta {
    CommandBuilder::from_clap(play_command())
        .category(CommandCategory::Media)
        .build()
}

/// Launch a media player (mpv by default) with the given URL.
async fn launch_player(url: &str) -> Result<()> {
    output::info(&format!("Launching mpv with: {url}"));
    let result = tokio::process::Command::new("mpv")
        .arg(url)
        .spawn();

    match result {
        Ok(mut child) => {
            // Wait for the player to finish
            let status = child.wait().await.map_err(|e| {
                BosuaError::Command(format!("Failed to wait for media player: {e}"))
            })?;
            if status.success() {
                output::success("Playback finished.");
            } else {
                output::warning(&format!(
                    "Media player exited with status: {}",
                    status.code().unwrap_or(-1)
                ));
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(BosuaError::Command(
                "Media player 'mpv' not found. Install mpv to use playback features.".into(),
            ))
        }
        Err(e) => Err(BosuaError::Command(format!(
            "Failed to start media player: {e}"
        ))),
    }
}

/// Build a Kodi JSON-RPC play request body.
fn build_kodi_request(source: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "Player.Open",
        "params": {
            "item": {
                "file": source
            }
        }
    })
}

/// Handle the `play` command.
pub async fn handle_play(matches: &ArgMatches, services: &ServiceRegistry) -> Result<()> {
    let _host = matches.get_one::<String>("host").unwrap();
    let _pass = matches.get_one::<String>("pass").unwrap();
    let _player = matches.get_one::<String>("player").unwrap();
    let _port = matches.get_one::<String>("port").unwrap();
    let _user = matches.get_one::<String>("user").unwrap();
    let _auto_discover = matches.get_flag("auto-discover");

    match matches.subcommand() {
        Some(("all", sub)) => {
            let _path = sub.get_one::<String>("path");
            let _terms: Vec<&String> = sub.get_many::<String>("terms").map(|v| v.collect()).unwrap_or_default();
            println!("play all: not yet implemented");
            Ok(())
        }
        Some(("file", sub)) => {
            let _path = sub.get_one::<String>("path");
            let _terms: Vec<&String> = sub.get_many::<String>("terms").map(|v| v.collect()).unwrap_or_default();
            println!("play file: not yet implemented");
            Ok(())
        }
        Some(("scan", _)) => {
            println!("play scan: not yet implemented");
            Ok(())
        }
        _ => {
            // Direct play with source argument
            if let Some(source) = matches.get_one::<String>("source") {
                launch_player(source).await?;
            } else {
                println!("play: provide a URL/pattern or use a subcommand (all, file, scan)");
            }
            let _ = services;
            Ok(())
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
    fn test_play_command_parses_source() {
        let cmd = play_command();
        let m = cmd.try_get_matches_from(["play", "https://example.com/video.mp4"]).unwrap();
        assert_eq!(m.get_one::<String>("source").map(|s| s.as_str()), Some("https://example.com/video.mp4"));
    }

    #[test]
    fn test_play_all_subcommand() {
        let cmd = play_command();
        let m = cmd.try_get_matches_from(["play", "all", "movie", "name"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("all"));
    }

    #[test]
    fn test_play_file_subcommand() {
        let cmd = play_command();
        let m = cmd.try_get_matches_from(["play", "file", "--path", "Folder/Episode01.mkv"]).unwrap();
        let (name, sub) = m.subcommand().unwrap();
        assert_eq!(name, "file");
        assert_eq!(sub.get_one::<String>("path").map(|s| s.as_str()), Some("Folder/Episode01.mkv"));
    }

    #[test]
    fn test_play_scan_subcommand() {
        let cmd = play_command();
        let m = cmd.try_get_matches_from(["play", "scan"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("scan"));
    }

    #[test]
    fn test_play_persistent_flags() {
        let cmd = play_command();
        let m = cmd.try_get_matches_from(["play", "--host", "192.168.1.1", "--port", "9090", "--player", "vlc", "scan"]).unwrap();
        assert_eq!(m.get_one::<String>("host").map(|s| s.as_str()), Some("192.168.1.1"));
        assert_eq!(m.get_one::<String>("port").map(|s| s.as_str()), Some("9090"));
        assert_eq!(m.get_one::<String>("player").map(|s| s.as_str()), Some("vlc"));
    }

    #[test]
    fn test_play_meta() {
        let meta = play_meta();
        assert_eq!(meta.name, "play");
        assert_eq!(meta.category, CommandCategory::Media);
    }

    #[test]
    fn test_source_type_from_str() {
        assert_eq!(SourceType::from_str_value("url"), Some(SourceType::Url));
        assert_eq!(SourceType::from_str_value("kodi"), Some(SourceType::Kodi));
        assert_eq!(SourceType::from_str_value("invalid"), None);
    }
}
