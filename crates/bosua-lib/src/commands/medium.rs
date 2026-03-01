//! Medium article access command.
//!
//! Provides the `medium` command for accessing Medium articles using a
//! configurable MediumPremiumDomain from DynamicConfig.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::config::dynamic::DynamicConfig;
use crate::errors::{BosuaError, Result};
use crate::http_client::HttpClient;

/// Build the `medium` clap command.
pub fn medium_command() -> Command {
    Command::new("medium")
        .about("Medium PDF generator")
        .aliases(["m", "me"])
        .arg(
            Arg::new("url")
                .required(true)
                .help("Medium article URL or search terms"),
        )
        .arg(
            Arg::new("domain")
                .long("domain")
                .default_value("freedium.cfd")
                .help("Premium domain"),
        )
        .arg(
            Arg::new("file")
                .long("file")
                .help("Input file"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .help("Output directory"),
        )
        .arg(
            Arg::new("pdf")
                .long("pdf")
                .action(clap::ArgAction::SetTrue)
                .help("Generate PDF instead of HTML (default: HTML for faster generation)"),
        )
        .arg(
            Arg::new("scraperapi")
                .long("scraperapi")
                .help("User Scraper API"),
        )
        .arg(
            Arg::new("worker")
                .long("worker")
                .value_parser(clap::value_parser!(i64))
                .default_value("5")
                .help("Number of workers"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn medium_meta() -> CommandMeta {
    CommandBuilder::from_clap(medium_command())
        .category(CommandCategory::Developer)
        .build()
}

/// Rewrite a Medium URL to use the configured premium domain.
/// Extracts the path from the original URL and prepends the premium domain.
fn rewrite_url(original_url: &str, premium_domain: &str) -> Result<String> {
    // Find the path portion after the host
    let stripped = original_url
        .strip_prefix("https://")
        .or_else(|| original_url.strip_prefix("http://"))
        .ok_or_else(|| BosuaError::Command(format!("Invalid URL (missing scheme): {}", original_url)))?;

    // Find the first '/' after the host to get the path
    let path = stripped.find('/').map(|i| &stripped[i..]).unwrap_or("/");

    Ok(format!("https://{}{}", premium_domain, path))
}

/// Handle the `medium` command.
pub async fn handle_medium(
    matches: &ArgMatches,
    config: &DynamicConfig,
    http: &HttpClient,
) -> Result<()> {
    let url = matches.get_one::<String>("url").unwrap();
    let domain = matches
        .get_one::<String>("domain")
        .map(|s| s.as_str())
        .unwrap_or("freedium.cfd");

    let rewritten = rewrite_url(url, domain)?;
    println!("Fetching article via: {}", rewritten);

    let client = http.get_client().await;
    let resp = client
        .get(&rewritten)
        .send()
        .await
        .map_err(|e| BosuaError::Command(format!("Failed to fetch article: {}", e)))?;

    if !resp.status().is_success() {
        return Err(BosuaError::Command(format!(
            "Failed to fetch article: HTTP {}",
            resp.status()
        )));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| BosuaError::Command(format!("Failed to read response: {}", e)))?;

    // Extract readable text from HTML using article/body selectors
    let doc = crate::utils::html::parse(&body);
    let elements = crate::utils::html::select(&doc, "article, .postArticle, p")
        .unwrap_or_default();
    if elements.is_empty() {
        println!("{}", body);
    } else {
        for el in &elements {
            let text = crate::utils::html::text(el);
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                println!("{}", trimmed);
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
    fn test_medium_command_parses_url() {
        let cmd = medium_command();
        let matches = cmd.try_get_matches_from(["medium", "https://medium.com/@user/article-123"]).unwrap();
        assert_eq!(matches.get_one::<String>("url").map(|s| s.as_str()), Some("https://medium.com/@user/article-123"));
    }

    #[test]
    fn test_medium_default_domain() {
        let cmd = medium_command();
        let matches = cmd.try_get_matches_from(["medium", "https://medium.com/@user/article"]).unwrap();
        assert_eq!(matches.get_one::<String>("domain").map(|s| s.as_str()), Some("freedium.cfd"));
    }

    #[test]
    fn test_medium_with_domain_override() {
        let cmd = medium_command();
        let matches = cmd.try_get_matches_from(["medium", "https://medium.com/@user/article", "--domain", "other.cfd"]).unwrap();
        assert_eq!(matches.get_one::<String>("domain").map(|s| s.as_str()), Some("other.cfd"));
    }

    #[test]
    fn test_medium_pdf_flag() {
        let cmd = medium_command();
        let matches = cmd.try_get_matches_from(["medium", "https://medium.com/@user/article", "--pdf"]).unwrap();
        assert!(matches.get_flag("pdf"));
    }

    #[test]
    fn test_medium_worker_flag() {
        let cmd = medium_command();
        let matches = cmd.try_get_matches_from(["medium", "https://medium.com/@user/article", "--worker", "10"]).unwrap();
        assert_eq!(matches.get_one::<i64>("worker").copied(), Some(10));
    }

    #[test]
    fn test_medium_requires_url() {
        let cmd = medium_command();
        let result = cmd.try_get_matches_from(["medium"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_medium_meta() {
        let meta = medium_meta();
        assert_eq!(meta.name, "medium");
        assert_eq!(meta.category, CommandCategory::Developer);
    }

    #[test]
    fn test_medium_aliases() {
        let cmd = medium_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"m"));
        assert!(aliases.contains(&"me"));
    }

    #[test]
    fn test_rewrite_url_basic() {
        let result = rewrite_url("https://medium.com/@user/my-article-123", "freedium.cfd");
        assert_eq!(result.unwrap(), "https://freedium.cfd/@user/my-article-123");
    }

    #[test]
    fn test_rewrite_url_http() {
        let result = rewrite_url("http://medium.com/@user/article", "freedium.cfd");
        assert_eq!(result.unwrap(), "https://freedium.cfd/@user/article");
    }

    #[test]
    fn test_rewrite_url_invalid() {
        let result = rewrite_url("not-a-url", "freedium.cfd");
        assert!(result.is_err());
    }
}
