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
            Arg::new("apikey")
                .long("apikey")
                .default_value("d07d69f1-ceb6-49b8-b2b3-701c0cb986d3")
                .help("Testmail.app API KEY"),
        )
        .arg(
            Arg::new("namespace")
                .long("namespace")
                .default_value("cn6bs")
                .help("Testmail.app Namespace"),
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
    let apikey = matches.get_one::<String>("apikey").unwrap();
    let namespace = matches.get_one::<String>("namespace").unwrap();

    println!("ScraperAPI - apikey: {}, namespace: {}", apikey, namespace);

    let request_url = format!(
        "{}/?api_key={}&namespace={}",
        SCRAPER_API_URL, apikey, namespace
    );

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
    fn test_scraper_api_command_defaults() {
        let cmd = scraper_api_command();
        let matches = cmd.try_get_matches_from(["scraperapi"]).unwrap();
        assert_eq!(matches.get_one::<String>("apikey").map(|s| s.as_str()), Some("d07d69f1-ceb6-49b8-b2b3-701c0cb986d3"));
        assert_eq!(matches.get_one::<String>("namespace").map(|s| s.as_str()), Some("cn6bs"));
    }

    #[test]
    fn test_scraper_api_with_apikey() {
        let cmd = scraper_api_command();
        let matches = cmd.try_get_matches_from(["scraperapi", "--apikey", "key123"]).unwrap();
        assert_eq!(matches.get_one::<String>("apikey").map(|s| s.as_str()), Some("key123"));
    }

    #[test]
    fn test_scraper_api_with_namespace() {
        let cmd = scraper_api_command();
        let matches = cmd.try_get_matches_from(["scraperapi", "--namespace", "myns"]).unwrap();
        assert_eq!(matches.get_one::<String>("namespace").map(|s| s.as_str()), Some("myns"));
    }

    #[test]
    fn test_scraper_api_meta() {
        let meta = scraper_api_meta();
        assert_eq!(meta.name, "scraperapi");
        assert_eq!(meta.category, CommandCategory::Developer);
    }
}
