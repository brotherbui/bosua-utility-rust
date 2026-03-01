//! Download CLI command — download files from URLs or a links file.
//!
//! Matches Go's `download` command with alias `dl`.
//! Usage: `bosua download <url1> [url2]... | <file.txt>`
//!
//! Links are normalized before download:
//! - Bare FShare codes (e.g. `8AG52MO8S4TL6UIP`) become `https://www.fshare.vn/file/<code>`
//! - FShare URLs are resolved to VIP direct-download links
//! - Direct HTTP(S) URLs are downloaded as-is

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::fshare::FShareClient;
use crate::download::DownloadManager;
use crate::errors::Result;
use crate::output;
use crate::signal::SignalHandler;

/// Build the `download` clap command.
pub fn download_command() -> Command {
    Command::new("download")
        .aliases(["dl"])
        .about("Download operations")
        .arg(
            Arg::new("urls")
                .num_args(0..)
                .help("URLs to download, or .txt file containing URLs"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn download_meta() -> CommandMeta {
    CommandBuilder::from_clap(download_command())
        .category(CommandCategory::Network)
        .build()
}

/// Normalize a link input to a downloadable URL.
///
/// Matches Go's `fshare.NormalizeLink()`:
/// - If it already contains "http", return as-is
/// - If it looks like a bare FShare code (>=10 chars, no "http"), wrap it
/// - Short codes are treated as shortened URLs (resolved via redirect)
fn normalize_link(input: &str) -> String {
    let input = input.trim();

    // Already a full URL
    if input.contains("http") {
        return input.to_string();
    }

    // Already an fshare URL without scheme
    if input.contains("fshare") {
        return format!("https://{}", input);
    }

    // Bare FShare code (>= 10 chars alphanumeric)
    if input.len() >= 10 {
        return format!("https://www.fshare.vn/file/{}", input);
    }

    // Short code — return as-is, caller should handle redirect resolution
    input.to_string()
}

/// Process a list of links: normalize, resolve FShare VIP links, then download.
///
/// Matches Go's `ProcessLinksWithContext` flow:
/// 1. Normalize each link (bare codes → FShare URLs)
/// 2. FShare links → resolve VIP direct download URL
/// 3. Direct URLs → download directly
async fn process_links(
    links: &[String],
    fshare: Option<&FShareClient>,
    dl: &DownloadManager,
    token: tokio_util::sync::CancellationToken,
) -> Result<usize> {
    let mut count = 0usize;

    for link in links {
        if token.is_cancelled() {
            output::warning("Download cancelled by signal.");
            break;
        }

        let normalized = normalize_link(link);

        // Handle FShare folder links
        if normalized.contains("fshare") && normalized.contains("folder") {
            if let Some(fs) = fshare {
                match fs.scan_folder(&normalized, None).await {
                    Ok(resp) => {
                        for entry in &resp.files {
                            if token.is_cancelled() {
                                break;
                            }
                            if let Some(ref code) = entry.link_code {
                                let file_url = format!("https://www.fshare.vn/file/{}", code);
                                match resolve_and_download(
                                    &file_url, fshare, dl, &token,
                                ).await {
                                    Ok(true) => count += 1,
                                    Ok(false) => {}
                                    Err(e) => output::error(&format!("{}: {}", file_url, e)),
                                }
                            }
                        }
                    }
                    Err(e) => {
                        output::error(&format!(
                            "Warning: failed to get links from folder {}: {}",
                            normalized, e
                        ));
                    }
                }
            } else {
                output::error(&format!(
                    "FShare folder link requires authentication: {}",
                    normalized
                ));
            }
            continue;
        }

        // Handle comma-separated links (Go supports "link1,link2")
        let sub_links: Vec<&str> = normalized.split(',').collect();
        for sub_link in sub_links {
            if token.is_cancelled() {
                break;
            }
            let sub = normalize_link(sub_link);
            if sub.is_empty() {
                continue;
            }
            match resolve_and_download(&sub, fshare, dl, &token).await {
                Ok(true) => count += 1,
                Ok(false) => {}
                Err(e) => output::error(&format!("Failed to download {}: {}", sub, e)),
            }
        }
    }

    Ok(count)
}

/// Resolve an FShare link to a VIP URL (if applicable) and download it.
/// Returns Ok(true) on success, Ok(false) on skip, Err on failure.
async fn resolve_and_download(
    url: &str,
    fshare: Option<&FShareClient>,
    dl: &DownloadManager,
    token: &tokio_util::sync::CancellationToken,
) -> Result<bool> {
    // FShare links: use aria2 with 25% VIP link renewal trick
    if url.contains("fshare.vn") && !url.contains("folder") {
        if let Some(fs) = fshare {
            match dl.do_fshare_download(url, fs, token, false, 0).await {
                Ok(_result) => return Ok(true),
                Err(e) => {
                    output::error(&format!("{} -> download failed: {}", url, e));
                    return Ok(false);
                }
            }
        } else {
            output::error(&format!(
                "FShare link requires authentication: {}",
                url
            ));
            return Ok(false);
        }
    }

    // Direct URLs: simple HTTP download
    let results = dl
        .download_urls_with_context(token.clone(), &[url.to_string()], false, 0)
        .await?;

    Ok(!results.is_empty())
}

/// Handle the `download` command.
pub async fn handle_download(
    matches: &ArgMatches,
    dl: &DownloadManager,
    fshare: Option<&FShareClient>,
) -> Result<()> {
    let signal_handler = SignalHandler::new();
    let token = signal_handler.token();

    tokio::spawn(async move {
        signal_handler.listen().await;
    });

    // Collect URLs from positional args
    let urls: Vec<String> = matches
        .get_many::<String>("urls")
        .map(|vals| vals.cloned().collect())
        .unwrap_or_default();

    if urls.is_empty() {
        // No args — fall back to default links file
        let results = dl.download_with_context(token, false, 0).await?;
        println!(
            "================{} link(s) completed successfully================",
            results.len()
        );
        return Ok(());
    }

    // Expand .txt files into individual links
    let mut links = Vec::new();
    for url in &urls {
        if url.ends_with(".txt") {
            match tokio::fs::read_to_string(url).await {
                Ok(content) => {
                    for line in content.lines() {
                        let line = line.trim();
                        if !line.is_empty() && !line.starts_with('#') {
                            links.push(line.to_string());
                        }
                    }
                }
                Err(e) => {
                    output::error(&format!("Failed to read {}: {}", url, e));
                }
            }
        } else {
            links.push(url.clone());
        }
    }

    if links.is_empty() {
        output::info("Input required!");
        return Ok(());
    }

    let count = process_links(&links, fshare, dl, token).await?;
    println!(
        "================{} link(s) completed successfully================",
        count
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_command_parses_urls() {
        let cmd = download_command();
        let matches = cmd
            .try_get_matches_from([
                "download",
                "https://example.com/file.zip",
                "https://example.com/file2.zip",
            ])
            .unwrap();
        let urls: Vec<&String> = matches.get_many::<String>("urls").unwrap().collect();
        assert_eq!(urls.len(), 2);
    }

    #[test]
    fn test_download_command_parses_txt_file() {
        let cmd = download_command();
        let matches = cmd
            .try_get_matches_from(["download", "links.txt"])
            .unwrap();
        let urls: Vec<&String> = matches.get_many::<String>("urls").unwrap().collect();
        assert_eq!(urls[0], "links.txt");
    }

    #[test]
    fn test_download_command_no_args() {
        let cmd = download_command();
        let matches = cmd.try_get_matches_from(["download"]).unwrap();
        assert!(matches.get_many::<String>("urls").is_none());
    }

    #[test]
    fn test_download_alias() {
        let cmd = download_command();
        assert!(cmd.get_all_aliases().collect::<Vec<_>>().contains(&"dl"));
    }

    #[test]
    fn test_download_meta() {
        let meta = download_meta();
        assert_eq!(meta.name, "download");
        assert_eq!(meta.category, CommandCategory::Network);
    }

    #[test]
    fn test_normalize_link_full_url() {
        assert_eq!(
            normalize_link("https://example.com/file.zip"),
            "https://example.com/file.zip"
        );
    }

    #[test]
    fn test_normalize_link_fshare_code() {
        assert_eq!(
            normalize_link("8AG52MO8S4TL6UIP"),
            "https://www.fshare.vn/file/8AG52MO8S4TL6UIP"
        );
    }

    #[test]
    fn test_normalize_link_fshare_url() {
        assert_eq!(
            normalize_link("https://www.fshare.vn/file/ABC123DEF456"),
            "https://www.fshare.vn/file/ABC123DEF456"
        );
    }

    #[test]
    fn test_normalize_link_short_code() {
        // Short codes (< 10 chars) are returned as-is
        assert_eq!(normalize_link("abc"), "abc");
    }

    #[test]
    fn test_normalize_link_trims_whitespace() {
        assert_eq!(
            normalize_link("  8AG52MO8S4TL6UIP  "),
            "https://www.fshare.vn/file/8AG52MO8S4TL6UIP"
        );
    }
}
