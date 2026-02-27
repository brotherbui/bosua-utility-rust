//! Download CLI command — download files from URLs or a links file.
//!
//! Matches Go's `download` command with alias `dl`.
//! Usage: `bosua download <url1> [url2]... | <file.txt>`

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::download::DownloadManager;
use crate::errors::Result;
use crate::signal::SignalHandler;
use crate::output;

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

/// Handle the `download` command.
///
/// If positional args are provided, they are treated as URLs (or .txt files
/// containing URLs). Otherwise falls back to the default links file via
/// DownloadManager.
pub async fn handle_download(
    matches: &ArgMatches,
    dl: &DownloadManager,
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

    let results = dl.download_urls_with_context(token, &links, false, 0).await?;
    println!(
        "================{} link(s) completed successfully================",
        results.len()
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
            .try_get_matches_from(["download", "https://example.com/file.zip", "https://example.com/file2.zip"])
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
}
