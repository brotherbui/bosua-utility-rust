//! Search CLI command with subcommands: media, news, software.
//!
//! Provides the `search` command for searching across multiple sources.
//! Each subcommand takes a positional `query` argument and an optional
//! `--json` flag for machine-readable output.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};
use crate::search::{SearchEngine, SearchResult, SearchSource};

/// Build the `search` clap command with all subcommands.
pub fn search_command() -> Command {
    Command::new("search")
        .about("Search across multiple sources")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(media_subcommand())
        .subcommand(news_subcommand())
        .subcommand(software_subcommand())
}

/// Build the `CommandMeta` for registry registration.
pub fn search_meta() -> CommandMeta {
    CommandBuilder::from_clap(search_command())
        .category(CommandCategory::Media)
        .build()
}

/// Handle the `search` command dispatch.
pub async fn handle_search(matches: &ArgMatches, engine: &SearchEngine) -> Result<()> {
    match matches.subcommand() {
        Some(("media", sub)) => search_and_display(engine, SearchSource::Media, sub).await,
        Some(("news", sub)) => search_and_display(engine, SearchSource::News, sub).await,
        Some(("software", sub)) => {
            search_and_display(engine, SearchSource::Software, sub).await
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// Subcommand definitions
// ---------------------------------------------------------------------------

fn search_args() -> Vec<Arg> {
    vec![
        Arg::new("query")
            .required(true)
            .help("Search query string"),
        Arg::new("json")
            .long("json")
            .action(clap::ArgAction::SetTrue)
            .help("Output results as JSON"),
    ]
}

fn media_subcommand() -> Command {
    Command::new("media")
        .about("Search media sources")
        .args(search_args())
}

fn news_subcommand() -> Command {
    Command::new("news")
        .about("Search news sources")
        .args(search_args())
}

fn software_subcommand() -> Command {
    Command::new("software")
        .about("Search software sources")
        .args(search_args())
}

// ---------------------------------------------------------------------------
// Shared search + display helper
// ---------------------------------------------------------------------------

async fn search_and_display(
    engine: &SearchEngine,
    source: SearchSource,
    matches: &ArgMatches,
) -> Result<()> {
    let query = matches.get_one::<String>("query").unwrap();
    let json = matches.get_flag("json");

    let results = engine.search(source, Some(query)).await?;

    if json {
        let json_output = serde_json::to_string_pretty(&results)
            .map_err(|e| BosuaError::Application(format!("JSON serialization error: {}", e)))?;
        println!("{}", json_output);
    } else if results.is_empty() {
        println!("No results found for '{}'", query);
    } else {
        display_results(&results);
    }

    Ok(())
}

fn display_results(results: &[SearchResult]) {
    println!(
        "{:<50} {:<60} {}",
        "TITLE", "URL", "DESCRIPTION"
    );
    println!("{}", "-".repeat(140));
    for r in results {
        let title = if r.title.len() > 48 {
            format!("{}…", &r.title[..47])
        } else {
            r.title.clone()
        };
        let url = if r.url.len() > 58 {
            format!("{}…", &r.url[..57])
        } else {
            r.url.clone()
        };
        let desc = if r.description.len() > 50 {
            format!("{}…", &r.description[..49])
        } else {
            r.description.clone()
        };
        println!("{:<50} {:<60} {}", title, url, desc);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_command_parses_media() {
        let cmd = search_command();
        let matches = cmd
            .try_get_matches_from(["search", "media", "rust programming"])
            .unwrap();
        assert_eq!(matches.subcommand_name(), Some("media"));
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(
            sub.get_one::<String>("query").map(|s| s.as_str()),
            Some("rust programming"),
        );
    }

    #[test]
    fn test_search_command_parses_news() {
        let cmd = search_command();
        let matches = cmd
            .try_get_matches_from(["search", "news", "tech updates"])
            .unwrap();
        assert_eq!(matches.subcommand_name(), Some("news"));
    }

    #[test]
    fn test_search_command_parses_software() {
        let cmd = search_command();
        let matches = cmd
            .try_get_matches_from(["search", "software", "editor"])
            .unwrap();
        assert_eq!(matches.subcommand_name(), Some("software"));
    }

    #[test]
    fn test_search_requires_subcommand() {
        let cmd = search_command();
        let result = cmd.try_get_matches_from(["search"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_subcommand_requires_query() {
        let cmd = search_command();
        let result = cmd.try_get_matches_from(["search", "media"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_json_flag() {
        let cmd = search_command();
        let matches = cmd
            .try_get_matches_from(["search", "media", "test", "--json"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert!(sub.get_flag("json"));
    }

    #[test]
    fn test_search_no_json_flag() {
        let cmd = search_command();
        let matches = cmd
            .try_get_matches_from(["search", "news", "test"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert!(!sub.get_flag("json"));
    }

    #[test]
    fn test_search_meta() {
        let meta = search_meta();
        assert_eq!(meta.name, "search");
        assert_eq!(meta.category, CommandCategory::Media);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = search_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"media"));
        assert!(sub_names.contains(&"news"));
        assert!(sub_names.contains(&"software"));
        assert_eq!(sub_names.len(), 3);
    }
}
