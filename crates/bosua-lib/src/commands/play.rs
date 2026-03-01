//! Play CLI command for media playback from various sources.
//!
//! Matches Go's `cmd_play.go` behavior:
//! - Default player is Kodi (JSON-RPC at `http://{host}:{port}/jsonrpc`)
//! - Resolves FShare VIP links for bare codes / FShare URLs
//! - Resolves episode patterns (s01e07, e7, 07) via GCP backend API
//! - Subcommands: `all` (playlist), `file` (single file search), `scan` (network discovery)

use std::net::TcpStream;
use std::time::Duration;

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::gcp::{self, GcpClient};
use crate::commands::registry_cmd::ServiceRegistry;
use crate::errors::{BosuaError, Result};
use crate::output;

// ---------------------------------------------------------------------------
// Constants (matching Go's constants package)
// ---------------------------------------------------------------------------

const DEFAULT_KODI_PORT: &str = "6868";
const DEFAULT_KODI_USERNAME: &str = "kodi";
const DEFAULT_KODI_PASSWORD: &str = "conchimnon";
const LOCALHOST: &str = "localhost";

// ---------------------------------------------------------------------------
// Kodi config
// ---------------------------------------------------------------------------

struct KodiConfig {
    ip: String,
    port: String,
    username: String,
    password: String,
}

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

/// Build the `play` clap command.
pub fn play_command() -> Command {
    Command::new("play")
        .about("Play operations")
        .arg(Arg::new("source").help("URL or episode pattern"))
        .arg(Arg::new("auto-discover").long("auto-discover").action(clap::ArgAction::SetTrue).help("Automatically discover Kodi instances on the local network"))
        .arg(Arg::new("host").long("host").default_value(LOCALHOST).help("Host"))
        .arg(Arg::new("pass").long("pass").default_value(DEFAULT_KODI_PASSWORD).help("Password"))
        .arg(Arg::new("player").long("player").default_value("kodi").help("Player name"))
        .arg(Arg::new("port").long("port").default_value(DEFAULT_KODI_PORT).help("Port"))
        .arg(Arg::new("user").long("user").short('u').default_value(DEFAULT_KODI_USERNAME).help("User"))
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_kodi(player: &str) -> bool {
    player.to_lowercase() == "kodi"
}

/// Check if a link is a direct HTTP(S) URL (not an FShare link).
fn is_direct_link(link: &str) -> bool {
    (link.contains("http") || link.contains("https")) && !link.contains("www.fshare.vn")
}

/// Check if input matches episode patterns: s01e07, s1e7, e7, e07, 07, 7
fn is_episode_pattern(input: &str) -> bool {
    let input = input.to_lowercase();
    let bytes = input.as_bytes();

    // Pattern: s##e## or s#e#
    if bytes.first() == Some(&b's') {
        let rest = &input[1..];
        if let Some(e_pos) = rest.find('e') {
            let s_part = &rest[..e_pos];
            let e_part = &rest[e_pos + 1..];
            return (1..=2).contains(&s_part.len())
                && s_part.chars().all(|c| c.is_ascii_digit())
                && (1..=2).contains(&e_part.len())
                && e_part.chars().all(|c| c.is_ascii_digit());
        }
        return false;
    }

    // Pattern: e## or e#
    if bytes.first() == Some(&b'e') {
        let rest = &input[1..];
        return (1..=2).contains(&rest.len()) && rest.chars().all(|c| c.is_ascii_digit());
    }

    // Pattern: just 1-2 digits
    (1..=2).contains(&input.len()) && input.chars().all(|c| c.is_ascii_digit())
}

/// Normalize an FShare code/URL to a full FShare URL.
fn normalize_fshare_link(input: &str) -> String {
    let input = input.trim();
    if input.contains("fshare.vn") {
        input.to_string()
    } else {
        format!("https://www.fshare.vn/file/{input}")
    }
}

// ---------------------------------------------------------------------------
// Kodi JSON-RPC
// ---------------------------------------------------------------------------

/// Play a single URL on Kodi via JSON-RPC Player.Open.
async fn kodi_play(url: &str, config: &KodiConfig, http: &reqwest::Client) -> Result<()> {
    let is_localhost = config.ip == LOCALHOST || config.ip == "127.0.0.1";

    // On macOS localhost, try to open Kodi app if not running
    #[cfg(feature = "macos")]
    if is_localhost {
        if let Ok(output) = tokio::process::Command::new("pgrep").arg("-x").arg("Kodi").output().await {
            if !output.status.success() {
                let _ = tokio::process::Command::new("open").arg("-a").arg("Kodi").spawn();
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    }

    let rpc_url = format!("http://{}:{}/jsonrpc", config.ip, config.port);
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "Player.Open",
        "params": { "item": { "file": url } }
    });

    let max_retries = 5;
    let delay = Duration::from_secs(2);
    let mut last_err = None;

    for i in 0..max_retries {
        let result = http
            .post(&rpc_url)
            .basic_auth(&config.username, Some(&config.password))
            .json(&body)
            .send()
            .await;

        match result {
            Ok(resp) => {
                if resp.status().as_u16() == 401 {
                    return Err(BosuaError::Command("authentication failed (401)".into()));
                }
                output::success("Playing on Kodi...");
                return Ok(());
            }
            Err(e) => {
                if i == 0 && is_localhost {
                    output::info("Waiting for Kodi to fully start...");
                } else {
                    println!("Attempt {}/{}: Could not connect to Kodi RPC, retrying in {:?}...", i + 1, max_retries, delay);
                }
                last_err = Some(e);
                tokio::time::sleep(delay).await;
            }
        }
    }

    Err(BosuaError::Command(format!(
        "failed to connect to Kodi after retries: {}",
        last_err.map(|e| e.to_string()).unwrap_or_default()
    )))
}

/// Clear Kodi playlist, enqueue URLs, and start playback.
async fn kodi_playlist(urls: &[String], config: &KodiConfig, http: &reqwest::Client) {
    if urls.is_empty() { return; }
    let rpc_url = format!("http://{}:{}/jsonrpc", config.ip, config.port);

    // 1) Clear playlist
    let clear = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "Playlist.Clear",
        "params": { "playlistid": 1 }
    });
    let _ = http.post(&rpc_url)
        .basic_auth(&config.username, Some(&config.password))
        .json(&clear).send().await;

    // 2) Add each URL
    for u in urls {
        let add = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "Playlist.Add",
            "params": { "playlistid": 1, "item": { "file": u } }
        });
        let _ = http.post(&rpc_url)
            .basic_auth(&config.username, Some(&config.password))
            .json(&add).send().await;
    }

    // 3) Start playback
    let open = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "Player.Open",
        "params": { "item": { "playlistid": 1, "position": 0 } }
    });
    let _ = http.post(&rpc_url)
        .basic_auth(&config.username, Some(&config.password))
        .json(&open).send().await;

    output::success(&format!("Queued {} items and started playback on Kodi...", urls.len()));
}

/// Play via VLC.
async fn vlc_play(url: &str) -> Result<()> {
    let result = tokio::process::Command::new("vlc").arg(url).spawn();
    match result {
        Ok(mut child) => {
            let _ = child.wait().await;
            output::success("Playing on VLC...");
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(BosuaError::Command("VLC is not installed.".into()))
        }
        Err(e) => Err(BosuaError::Command(format!("Failed to start VLC: {e}"))),
    }
}

/// Play via VLC batch.
async fn vlc_batch(urls: &[String]) -> Result<()> {
    let mut cmd = tokio::process::Command::new("vlc");
    for u in urls { cmd.arg(u); }
    match cmd.spawn() {
        Ok(mut child) => {
            let _ = child.wait().await;
            output::success(&format!("Playing {} items on VLC...", urls.len()));
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(BosuaError::Command("VLC is not installed.".into()))
        }
        Err(e) => Err(BosuaError::Command(format!("Failed to start VLC: {e}"))),
    }
}

// ---------------------------------------------------------------------------
// Auto-discover
// ---------------------------------------------------------------------------

struct KodiInstance {
    ip: String,
    port: String,
}

fn is_port_open(ip: &str, port: &str, timeout: Duration) -> bool {
    let addr = format!("{ip}:{port}");
    TcpStream::connect_timeout(
        &addr.parse().unwrap_or_else(|_| format!("0.0.0.0:{port}").parse().unwrap()),
        timeout,
    ).is_ok()
}

/// Scan 192.168.1.0/24 for Kodi instances on port 6868.
async fn scan_local_network() -> Vec<KodiInstance> {
    let mut instances = Vec::new();
    let mut handles = Vec::new();

    for i in 2..=254u8 {
        let ip = format!("192.168.1.{i}");
        handles.push(tokio::task::spawn_blocking(move || {
            if is_port_open(&ip, DEFAULT_KODI_PORT, Duration::from_millis(500)) {
                Some(KodiInstance { ip, port: DEFAULT_KODI_PORT.to_string() })
            } else {
                None
            }
        }));
    }

    for h in handles {
        if let Ok(Some(inst)) = h.await {
            instances.push(inst);
        }
    }
    instances
}

/// Auto-discover Kodi and update config. Returns false if no instances found.
async fn auto_discover_kodi(auto_discover: bool, kodi_config: &mut KodiConfig) -> bool {
    if !auto_discover || kodi_config.ip != LOCALHOST {
        return true;
    }
    output::info("Auto-discovering Kodi instances...");
    let instances = scan_local_network().await;
    if instances.is_empty() {
        output::warning("No Kodi instances found on the network.");
        output::info("Skipping Kodi playback. Use --host <ip> to specify a Kodi instance manually.");
        return false;
    }
    let inst = &instances[0];
    kodi_config.ip = inst.ip.clone();
    kodi_config.port = inst.port.clone();
    if inst.ip == LOCALHOST || inst.ip == "127.0.0.1" {
        output::success(&format!("Found local Kodi instance: {}:{}", inst.ip, inst.port));
    } else {
        output::success(&format!("Found remote Kodi instance: {}:{}", inst.ip, inst.port));
    }
    true
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// Handle the `play` command (matches Go's cmd_play.go).
pub async fn handle_play(matches: &ArgMatches, services: &ServiceRegistry) -> Result<()> {
    let host = matches.get_one::<String>("host").unwrap().clone();
    let pass = matches.get_one::<String>("pass").unwrap().clone();
    let player = matches.get_one::<String>("player").unwrap().clone();
    let port = matches.get_one::<String>("port").unwrap().clone();
    let user = matches.get_one::<String>("user").unwrap().clone();
    let auto_discover = matches.get_flag("auto-discover");

    let mut kodi_config = KodiConfig {
        ip: host, port, username: user, password: pass,
    };

    let config = services.config_manager.get_config().await;
    let http_raw = services.http_client.get_client().await;
    let gcp_client = GcpClient::new(services.http_client.clone(), &config);

    match matches.subcommand() {
        Some(("all", sub)) => {
            handle_play_all(sub, &player, auto_discover, &mut kodi_config, &gcp_client, &http_raw).await
        }
        Some(("file", sub)) => {
            handle_play_file(sub, &player, auto_discover, &mut kodi_config, &gcp_client, &http_raw).await
        }
        Some(("scan", _)) => {
            handle_play_scan().await
        }
        _ => {
            // Direct play with source argument
            let source = match matches.get_one::<String>("source") {
                Some(s) => s.clone(),
                None => {
                    println!("Input required!");
                    return Ok(());
                }
            };

            // Resolve the source to a playable URL
            let vip_link = resolve_play_source(&source, &gcp_client, services).await?;
            if vip_link.is_empty() { return Ok(()); }

            if is_kodi(&player) {
                if !auto_discover_kodi(auto_discover, &mut kodi_config).await { return Ok(()); }
                kodi_play(&vip_link, &kodi_config, &http_raw).await?;
            } else {
                vlc_play(&vip_link).await?;
            }
            Ok(())
        }
    }
}

/// Resolve a play source: episode pattern → GCP API, FShare code → VIP link, or direct URL.
async fn resolve_play_source(
    source: &str,
    gcp_client: &GcpClient,
    services: &ServiceRegistry,
) -> Result<String> {
    if is_episode_pattern(source) {
        match gcp_client.resolve_episode(source).await {
            Ok(url) => return Ok(url),
            Err(e) => {
                println!("{e}");
                return Ok(String::new());
            }
        }
    }

    if is_direct_link(source) {
        return Ok(source.to_string());
    }

    // Treat as FShare code/URL
    let fshare_url = normalize_fshare_link(source);
    match services.fshare().await {
        Ok(fs) => match fs.resolve_vip_link(&fshare_url).await {
            Ok(link) => Ok(link),
            Err(e) => {
                println!("Could not get vip link: {e}");
                Ok(String::new())
            }
        },
        Err(e) => {
            println!("FShare not available: {e}");
            Ok(String::new())
        }
    }
}

// ---------------------------------------------------------------------------
// Subcommand: play all
// ---------------------------------------------------------------------------

async fn handle_play_all(
    sub: &ArgMatches,
    player: &str,
    auto_discover: bool,
    kodi_config: &mut KodiConfig,
    gcp_client: &GcpClient,
    http: &reqwest::Client,
) -> Result<()> {
    let play_all_path = sub.get_one::<String>("path").map(|s| s.as_str());
    let terms: Vec<String> = sub.get_many::<String>("terms")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();

    // If --path is provided, play from that specific directory
    if let Some(path) = play_all_path {
        let resp = gcp_client.fetch_remote_files(path).await?;
        if !resp.success {
            output::error(&format!("Server error: {}", resp.error));
            return Ok(());
        }
        let urls = gcp::collect_video_urls(&resp.files);
        if urls.is_empty() {
            output::warning("No video files found.");
            return Ok(());
        }
        return play_urls(&urls, player, auto_discover, kodi_config, http).await;
    }

    // No args: play from root
    if terms.is_empty() {
        let resp = gcp_client.fetch_remote_files("").await?;
        if !resp.success {
            output::error(&format!("Server error: {}", resp.error));
            return Ok(());
        }
        let urls = gcp::collect_video_urls(&resp.files);
        if urls.is_empty() {
            output::warning("No video files found in root directory.");
            return Ok(());
        }
        println!("Found {} video(s) in root directory", urls.len());
        return play_urls(&urls, player, auto_discover, kodi_config, http).await;
    }

    // Search for matching directories
    let matching_dirs = gcp_client.find_matching_directories(&terms).await?;
    if matching_dirs.is_empty() {
        output::warning(&format!("No directories found matching: {}", terms.join(", ")));
        return Ok(());
    }

    let mut all_urls = Vec::new();
    println!("Found {} matching directories:", matching_dirs.len());
    for dir in &matching_dirs {
        println!("  - {}", dir.path);
        match gcp_client.fetch_remote_files(&dir.path).await {
            Ok(resp) if resp.success => {
                let dir_urls = gcp::collect_video_urls(&resp.files);
                if !dir_urls.is_empty() {
                    println!("    Found {} video(s)", dir_urls.len());
                    all_urls.extend(dir_urls);
                } else {
                    println!("    No videos found");
                }
            }
            Ok(resp) => println!("    Warning: Server error for {}: {}", dir.path, resp.error),
            Err(e) => println!("    Warning: Failed to fetch files from {}: {e}", dir.path),
        }
    }

    if all_urls.is_empty() {
        output::warning("No video files found in any matching directories.");
        return Ok(());
    }

    println!("\nTotal videos found: {}", all_urls.len());
    play_urls(&all_urls, player, auto_discover, kodi_config, http).await
}

// ---------------------------------------------------------------------------
// Subcommand: play file
// ---------------------------------------------------------------------------

async fn handle_play_file(
    sub: &ArgMatches,
    player: &str,
    auto_discover: bool,
    kodi_config: &mut KodiConfig,
    gcp_client: &GcpClient,
    http: &reqwest::Client,
) -> Result<()> {
    let play_file_path = sub.get_one::<String>("path").map(|s| s.as_str());
    let terms: Vec<String> = sub.get_many::<String>("terms")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();

    // If --path is provided, find the file by exact path
    if let Some(file_path) = play_file_path {
        let file_path = file_path.trim();
        if !file_path.is_empty() {
            let parent = file_path.rfind('/').map(|i| &file_path[..i]).unwrap_or("");
            let resp = gcp_client.fetch_remote_files(parent).await?;
            if !resp.success {
                output::error(&format!("Server error: {}", resp.error));
                return Ok(());
            }
            let target = resp.files.iter()
                .find(|f| !f.is_directory() && f.path == file_path);
            match target.and_then(|f| f.signed_url.as_ref()) {
                Some(url) => return play_single(url, player, auto_discover, kodi_config, http).await,
                None => {
                    output::error("File not found in listing.");
                    return Ok(());
                }
            }
        }
    }

    if terms.is_empty() {
        output::error("Please provide folder (optional) and file search terms or use --path flag");
        println!("Usage: bosua play file [folder-term] [file-term]");
        println!("   or: bosua play file --path <full-file-path>");
        println!();
        println!("Example: bosua play file moon 38");
        println!("  This will match folder 'Moonlight Mystique - Bach Nguyet Phan Tinh 2025'");
        println!("  and file 'Moonlight.Mystique.S01E38.ViE.2160p.iQ.WEB-DL.DDP5.1.H.265-MrHulk.mkv'");
        return Ok(());
    }

    if terms.len() == 1 {
        // Single term: search root directory for matching file
        let file_term = &terms[0];
        match gcp_client.find_best_matching_file("", file_term).await? {
            Some(f) => {
                println!("Found matching file in root: {}", f.path);
                match f.signed_url.as_ref() {
                    Some(url) if !url.is_empty() => {
                        println!("Playing: {}", f.name);
                        return play_single(url, player, auto_discover, kodi_config, http).await;
                    }
                    _ => {
                        output::error("No signed URL available for the file.");
                        return Ok(());
                    }
                }
            }
            None => {
                output::warning(&format!("No file found matching: {file_term} in root directory"));
                return Ok(());
            }
        }
    }

    // Two or more terms: first is folder, second is file
    let folder_term = &terms[0];
    let file_term = &terms[1];

    let matching_dir = match gcp_client.find_best_matching_directory(folder_term).await? {
        Some(d) => d,
        None => {
            output::warning(&format!("No folder found matching: {folder_term}"));
            return Ok(());
        }
    };
    println!("Found matching folder: {}", matching_dir.path);

    match gcp_client.find_best_matching_file(&matching_dir.path, file_term).await? {
        Some(f) => {
            println!("Found matching file: {}", f.path);
            match f.signed_url.as_ref() {
                Some(url) if !url.is_empty() => {
                    println!("Playing: {}", f.name);
                    play_single(url, player, auto_discover, kodi_config, http).await
                }
                _ => {
                    output::error("No signed URL available for the file.");
                    Ok(())
                }
            }
        }
        None => {
            output::warning(&format!("No file found matching: {file_term} in folder {}", matching_dir.path));
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Subcommand: play scan
// ---------------------------------------------------------------------------

async fn handle_play_scan() -> Result<()> {
    output::info("Scanning local network for Kodi instances...");
    println!("This may take a few seconds...");

    let instances = scan_local_network().await;
    if instances.is_empty() {
        output::warning("No Kodi instances found on the local network.");
        println!();
        println!("Make sure:");
        println!("• Kodi is running on your devices");
        println!("• HTTP control is enabled in Kodi (Settings > Services > Control > HTTP)");
        println!("• Port 6868 is configured in Kodi settings");
        println!("• Your device is on the same network");
        return Ok(());
    }

    output::success(&format!("\nFound {} Kodi instance(s):", instances.len()));
    for (i, inst) in instances.iter().enumerate() {
        println!("{}. {}:{}", i + 1, inst.ip, inst.port);
    }
    println!();
    println!("To use a discovered instance, run:");
    println!("  bosua play --host {} <video_url>", instances[0].ip);
    println!("  bosua play all --host <ip_address>");
    println!("  bosua play file --host <ip_address> <search_terms>");
    Ok(())
}

// ---------------------------------------------------------------------------
// Shared play helpers
// ---------------------------------------------------------------------------

async fn play_urls(
    urls: &[String],
    player: &str,
    auto_discover: bool,
    kodi_config: &mut KodiConfig,
    http: &reqwest::Client,
) -> Result<()> {
    if is_kodi(player) {
        if !auto_discover_kodi(auto_discover, kodi_config).await { return Ok(()); }
        kodi_playlist(urls, kodi_config, http).await;
    } else {
        vlc_batch(urls).await?;
    }
    Ok(())
}

async fn play_single(
    url: &str,
    player: &str,
    auto_discover: bool,
    kodi_config: &mut KodiConfig,
    http: &reqwest::Client,
) -> Result<()> {
    if is_kodi(player) {
        if !auto_discover_kodi(auto_discover, kodi_config).await { return Ok(()); }
        kodi_play(url, kodi_config, http).await?;
    } else {
        vlc_play(url).await?;
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
    fn test_is_episode_pattern() {
        assert!(is_episode_pattern("s01e07"));
        assert!(is_episode_pattern("S1E7"));
        assert!(is_episode_pattern("e7"));
        assert!(is_episode_pattern("E07"));
        assert!(is_episode_pattern("07"));
        assert!(is_episode_pattern("7"));
        assert!(!is_episode_pattern("hello"));
        assert!(!is_episode_pattern("s01e07extra"));
        assert!(!is_episode_pattern("123")); // 3 digits
        assert!(!is_episode_pattern("https://example.com"));
    }

    #[test]
    fn test_is_direct_link() {
        assert!(is_direct_link("https://example.com/video.mp4"));
        assert!(is_direct_link("http://cdn.example.com/file"));
        assert!(!is_direct_link("V8871B7PREEJAHB"));
        assert!(!is_direct_link("https://www.fshare.vn/file/ABC123"));
    }

    #[test]
    fn test_normalize_fshare_link() {
        assert_eq!(
            normalize_fshare_link("V8871B7PREEJAHB"),
            "https://www.fshare.vn/file/V8871B7PREEJAHB"
        );
        assert_eq!(
            normalize_fshare_link("https://www.fshare.vn/file/ABC"),
            "https://www.fshare.vn/file/ABC"
        );
    }

    #[test]
    fn test_is_kodi() {
        assert!(is_kodi("kodi"));
        assert!(is_kodi("Kodi"));
        assert!(is_kodi("KODI"));
        assert!(!is_kodi("vlc"));
    }
}
