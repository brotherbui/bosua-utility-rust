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
        .about("Play media from various sources")
        .arg(
            Arg::new("source")
                .required(true)
                .help("URL or path to media source"),
        )
        .arg(
            Arg::new("type")
                .long("type")
                .short('t')
                .value_parser(["url", "fshare", "gdrive", "gcp", "kodi"])
                .default_value("url")
                .help("Source type (url, fshare, gdrive, gcp, kodi)"),
        )
        .arg(
            Arg::new("kodi-host")
                .long("kodi-host")
                .help("Kodi host address for Kodi integration"),
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
    let source = matches.get_one::<String>("source").unwrap();
    let source_type = matches
        .get_one::<String>("type")
        .and_then(|s| SourceType::from_str_value(s))
        .unwrap_or(SourceType::Url);
    let kodi_host = matches.get_one::<String>("kodi-host");

    match source_type {
        SourceType::Url => {
            launch_player(source).await?;
        }
        SourceType::Fshare => {
            output::info(&format!("Resolving FShare VIP link for: {source}"));
            let fshare = services.fshare().await?;
            let results = fshare.resolve_vip_links(&[source.clone()]).await;
            match results.into_iter().next() {
                Some((_, Ok(direct_url))) => {
                    output::success(&format!("Resolved: {direct_url}"));
                    launch_player(&direct_url).await?;
                }
                Some((_, Err(e))) => return Err(e),
                None => {
                    return Err(BosuaError::Command(
                        "FShare VIP link resolution returned no results".into(),
                    ));
                }
            }
        }
        SourceType::Gdrive => {
            output::info(&format!("Getting GDrive streamable URL for file: {source}"));
            let gdrive = services.gdrive().await?;
            let file = gdrive.get_file_metadata(source).await?;
            let stream_url = file
                .web_content_link
                .unwrap_or_else(|| format!("https://drive.google.com/uc?export=download&id={}", source));
            output::success(&format!("Stream URL: {stream_url}"));
            launch_player(&stream_url).await?;
        }
        SourceType::Gcp => {
            let config = services.config_manager.get_config().await;
            let host = if !config.gcp_domain.is_empty() {
                &config.gcp_domain
            } else if !config.gcp_ip.is_empty() {
                &config.gcp_ip
            } else {
                return Err(BosuaError::Config(
                    "GCP IP or domain not configured. Run `config set gcpIp <ip>` or `config set gcpDomain <domain>`.".into(),
                ));
            };
            // Trim leading slash to avoid double-slash in URL
            let path = source.trim_start_matches('/');
            let stream_url = format!("https://{host}/stream/{path}");
            output::info(&format!("Streaming from GCP: {stream_url}"));
            launch_player(&stream_url).await?;
        }
        SourceType::Kodi => {
            let config = services.config_manager.get_config().await;
            let host = kodi_host
                .map(|h| h.to_string())
                .unwrap_or_else(|| "localhost:8080".to_string());

            output::info(&format!("Sending play request to Kodi at {host}"));

            let request_body = build_kodi_request(source);
            let url = format!("http://{host}/jsonrpc", host = host);

            let client = services.http_client.get_client().await;
            let resp = client
                .post(&url)
                .basic_auth(&config.kodi_username, Some(&config.kodi_password))
                .json(&request_body)
                .send()
                .await
                .map_err(|e| BosuaError::Cloud {
                    service: "kodi".into(),
                    message: format!("Failed to send play request: {e}"),
                })?;

            if resp.status().is_success() {
                output::success(&format!("Kodi playback started: {source}"));
            } else {
                let status = resp.status().as_u16();
                let body = resp.text().await.unwrap_or_default();
                return Err(BosuaError::Cloud {
                    service: "kodi".into(),
                    message: format!("Kodi returned error ({status}): {body}"),
                });
            }
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
    fn test_play_command_parses_source() {
        let cmd = play_command();
        let matches = cmd
            .try_get_matches_from(["play", "https://example.com/video.mp4"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("source").map(|s| s.as_str()),
            Some("https://example.com/video.mp4"),
        );
    }

    #[test]
    fn test_play_default_type_is_url() {
        let cmd = play_command();
        let matches = cmd
            .try_get_matches_from(["play", "https://example.com/video.mp4"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("type").map(|s| s.as_str()),
            Some("url"),
        );
    }

    #[test]
    fn test_play_with_type_flag() {
        let cmd = play_command();
        let matches = cmd
            .try_get_matches_from(["play", "https://fshare.vn/file/ABC", "--type", "fshare"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("type").map(|s| s.as_str()),
            Some("fshare"),
        );
    }

    #[test]
    fn test_play_with_kodi_host() {
        let cmd = play_command();
        let matches = cmd
            .try_get_matches_from([
                "play",
                "movie.mkv",
                "--type",
                "kodi",
                "--kodi-host",
                "192.168.1.100:8080",
            ])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("kodi-host").map(|s| s.as_str()),
            Some("192.168.1.100:8080"),
        );
    }

    #[test]
    fn test_play_requires_source() {
        let cmd = play_command();
        let result = cmd.try_get_matches_from(["play"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_play_invalid_type_rejected() {
        let cmd = play_command();
        let result = cmd.try_get_matches_from(["play", "source", "--type", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_play_all_source_types() {
        for st in ["url", "fshare", "gdrive", "gcp", "kodi"] {
            let cmd = play_command();
            let matches = cmd
                .try_get_matches_from(["play", "source", "--type", st])
                .unwrap();
            assert_eq!(
                matches.get_one::<String>("type").map(|s| s.as_str()),
                Some(st),
            );
        }
    }

    #[test]
    fn test_source_type_from_str() {
        assert_eq!(SourceType::from_str_value("url"), Some(SourceType::Url));
        assert_eq!(SourceType::from_str_value("fshare"), Some(SourceType::Fshare));
        assert_eq!(SourceType::from_str_value("gdrive"), Some(SourceType::Gdrive));
        assert_eq!(SourceType::from_str_value("gcp"), Some(SourceType::Gcp));
        assert_eq!(SourceType::from_str_value("kodi"), Some(SourceType::Kodi));
        assert_eq!(SourceType::from_str_value("invalid"), None);
    }

    #[test]
    fn test_source_type_as_str() {
        assert_eq!(SourceType::Url.as_str(), "url");
        assert_eq!(SourceType::Fshare.as_str(), "fshare");
        assert_eq!(SourceType::Gdrive.as_str(), "gdrive");
        assert_eq!(SourceType::Gcp.as_str(), "gcp");
        assert_eq!(SourceType::Kodi.as_str(), "kodi");
    }

    #[test]
    fn test_play_meta() {
        let meta = play_meta();
        assert_eq!(meta.name, "play");
        assert_eq!(meta.category, CommandCategory::Media);
        assert!(!meta.description.is_empty());
    }
}
