//! ScraperAPI integration command.
//!
//! Provides the `scraper-api` command for web scraping via ScraperAPI,
//! with optional JavaScript rendering support.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};
use crate::http_client::HttpClient;

/// ScraperAPI base URL.
const SCRAPER_API_URL: &str = "https://api.scraperapi.com";

/// Build the `scraperapi` clap command.
pub fn scraper_api_command() -> Command {
    Command::new("scraperapi")
        .about("Scraper API operations")
        .arg(
            Arg::new("url")
                .required(true)
                .help("URL to scrape"),
        )
        .arg(
            Arg::new("api-key")
                .long("api-key")
                .short('k')
                .help("ScraperAPI key"),
        )
        .arg(
            Arg::new("render")
                .long("render")
                .short('r')
                .action(clap::ArgAction::SetTrue)
                .help("Enable JavaScript rendering"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn scraper_api_meta() -> CommandMeta {
    CommandBuilder::from_clap(scraper_api_command())
        .category(CommandCategory::Developer)
        .build()
}

/// Handle the `scraper-api` command.
pub async fn handle_scraper_api(
    matches: &ArgMatches,
    http: &HttpClient,
) -> Result<()> {
    let url = matches.get_one::<String>("url").unwrap();
    let api_key = matches
        .get_one::<String>("api-key")
        .or_else(|| std::env::var("SCRAPER_API_KEY").ok().as_ref().map(|_| unreachable!()))
        .cloned();

    // Try env var if not provided via CLI
    let api_key = match api_key {
        Some(k) => k,
        None => std::env::var("SCRAPER_API_KEY").map_err(|_| {
            BosuaError::Command(
                "ScraperAPI key not set. Use --api-key or set SCRAPER_API_KEY env var".into(),
            )
        })?,
    };

    let render = matches.get_flag("render");

    let mut request_url = format!(
        "{}/?api_key={}&url={}",
        SCRAPER_API_URL, api_key, url
    );

    if render {
        request_url.push_str("&render=true");
    }

    println!("Scraping: {}", url);

    let client = http.get_client().await;
    let resp = client
        .get(&request_url)
        .send()
        .await
        .map_err(|e| BosuaError::Command(format!("ScraperAPI request failed: {}", e)))?;

    if !resp.status().is_success() {
        return Err(BosuaError::Command(format!(
            "ScraperAPI returned HTTP {}",
            resp.status()
        )));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| BosuaError::Command(format!("Failed to read response: {}", e)))?;

    println!("{}", body);
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scraper_api_command_parses_url() {
        let cmd = scraper_api_command();
        let matches = cmd
            .try_get_matches_from(["scraperapi", "https://example.com"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("url").map(|s| s.as_str()),
            Some("https://example.com"),
        );
    }

    #[test]
    fn test_scraper_api_with_api_key() {
        let cmd = scraper_api_command();
        let matches = cmd
            .try_get_matches_from(["scraperapi", "https://example.com", "--api-key", "key123"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("api-key").map(|s| s.as_str()),
            Some("key123"),
        );
    }

    #[test]
    fn test_scraper_api_render_flag() {
        let cmd = scraper_api_command();
        let matches = cmd
            .try_get_matches_from(["scraperapi", "https://example.com", "--render"])
            .unwrap();
        assert!(matches.get_flag("render"));
    }

    #[test]
    fn test_scraper_api_render_default_false() {
        let cmd = scraper_api_command();
        let matches = cmd
            .try_get_matches_from(["scraperapi", "https://example.com"])
            .unwrap();
        assert!(!matches.get_flag("render"));
    }

    #[test]
    fn test_scraper_api_requires_url() {
        let cmd = scraper_api_command();
        let result = cmd.try_get_matches_from(["scraperapi"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_scraper_api_meta() {
        let meta = scraper_api_meta();
        assert_eq!(meta.name, "scraperapi");
        assert_eq!(meta.category, CommandCategory::Developer);
        assert!(!meta.description.is_empty());
    }
}
