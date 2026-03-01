//! Search CLI command — Search operations across multiple sources.
//!
//! Subcommands: blogtienao, coinphoton, decrypt, fshare, haxmac, imdb, macked, maclife, rophim, thuvienhd, vmf.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};
use crate::search::{SearchEngine, SearchResult, SearchSource};

/// Build the `search` clap command with all subcommands.
pub fn search_command() -> Command {
    Command::new("search")
        .about("Search operations")
        .aliases(["s"])
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(Arg::new("check").long("check").short('c').action(clap::ArgAction::SetTrue).help("Check result links for accessibility/availability"))
        .arg(Arg::new("first").long("first").short('1').action(clap::ArgAction::SetTrue).help("Auto get detail for first result/Or single thread"))
        .arg(Arg::new("info").long("info").short('i').action(clap::ArgAction::SetTrue).help("Get info"))
        .arg(Arg::new("sort").long("sort").default_value("created").help("Sort by: name, size, created"))
        .subcommand(Command::new("blogtienao").about("Get latest articles from blogtienao.com (optionally search for specific terms)").aliases(["bta", "tienao"]).arg(Arg::new("query").help("Search term")))
        .subcommand(Command::new("coinphoton").about("Get latest articles from coinphoton.com (optionally search for specific terms)").arg(Arg::new("query").help("Search term")))
        .subcommand(Command::new("decrypt").about("Search decrypt.day for iOS/Mac apps").arg(Arg::new("query").required(true).help("Search query")))
        .subcommand(Command::new("fshare").about("Search TimFshare for files").arg(Arg::new("query").required(true).help("Search query")))
        .subcommand(Command::new("haxmac").about("Search haxmac.cc for Mac software").arg(Arg::new("query").required(true).help("Search query")))
        .subcommand(Command::new("imdb").about("Search IMDB").arg(Arg::new("query").required(true).help("Search query")))
        .subcommand(Command::new("macked").about("Search macked.app for Mac software").arg(Arg::new("query").required(true).help("Search query")))
        .subcommand(Command::new("maclife").about("Search maclife").arg(Arg::new("query").required(true).help("Search query")))
        .subcommand(Command::new("rophim").about("Search Rophim for Vietnamese movies and TV shows").arg(Arg::new("query").required(true).help("Search query")))
        .subcommand(Command::new("thuvienhd").about("Search thuvienhd.top for movies and TV shows").arg(Arg::new("query").required(true).help("Search query")))
        .subcommand(Command::new("vmf").about("Search VMF media (uses BACKEND_IP for remote cached search if available)").arg(Arg::new("query").required(true).help("Search query")))
}

/// Build the `CommandMeta` for registry registration.
pub fn search_meta() -> CommandMeta {
    CommandBuilder::from_clap(search_command())
        .category(CommandCategory::Media)
        .build()
}

/// Handle the `search` command dispatch.
pub async fn handle_search(matches: &ArgMatches, engine: &SearchEngine) -> Result<()> {
    let _check = matches.get_flag("check");
    let _first = matches.get_flag("first");
    let _info = matches.get_flag("info");
    let _sort = matches.get_one::<String>("sort").unwrap();

    match matches.subcommand() {
        Some((source_name, sub)) => {
            let query = sub.get_one::<String>("query").map(|s| s.as_str());
            // Map subcommand name to SearchSource category
            let source = match source_name {
                "imdb" | "rophim" | "thuvienhd" | "vmf" => SearchSource::Media,
                "blogtienao" | "coinphoton" | "decrypt" => SearchSource::News,
                "fshare" | "haxmac" | "macked" | "maclife" => SearchSource::Software,
                _ => SearchSource::Media,
            };
            let results = engine.search(source, query).await?;
            if results.is_empty() {
                println!("No results found");
            } else {
                display_results(&results);
            }
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

fn display_results(results: &[SearchResult]) {
    println!("{:<50} {:<60} {}", "TITLE", "URL", "DESCRIPTION");
    println!("{}", "-".repeat(140));
    for r in results {
        let title = if r.title.len() > 48 { format!("{}…", &r.title[..47]) } else { r.title.clone() };
        let url = if r.url.len() > 58 { format!("{}…", &r.url[..57]) } else { r.url.clone() };
        let desc = if r.description.len() > 50 { format!("{}…", &r.description[..49]) } else { r.description.clone() };
        println!("{:<50} {:<60} {}", title, url, desc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_command_parses() {
        let cmd = search_command();
        let m = cmd.try_get_matches_from(["search", "imdb", "inception"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("imdb"));
    }

    #[test]
    fn test_search_alias_s() {
        let cmd = search_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"s"));
    }

    #[test]
    fn test_search_persistent_flags() {
        let cmd = search_command();
        let m = cmd.try_get_matches_from(["search", "--check", "--first", "--sort", "name", "fshare", "test"]).unwrap();
        assert!(m.get_flag("check"));
        assert!(m.get_flag("first"));
        assert_eq!(m.get_one::<String>("sort").map(|s| s.as_str()), Some("name"));
    }

    #[test]
    fn test_search_requires_subcommand() {
        let cmd = search_command();
        assert!(cmd.try_get_matches_from(["search"]).is_err());
    }

    #[test]
    fn test_search_blogtienao_alias() {
        let cmd = search_command();
        let m = cmd.try_get_matches_from(["search", "bta"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("blogtienao"));
    }

    #[test]
    fn test_search_meta() {
        let meta = search_meta();
        assert_eq!(meta.name, "search");
        assert_eq!(meta.category, CommandCategory::Media);
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = search_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        for name in &["blogtienao", "coinphoton", "decrypt", "fshare", "haxmac", "imdb", "macked", "maclife", "rophim", "thuvienhd", "vmf"] {
            assert!(sub_names.contains(name), "missing: {}", name);
        }
        assert_eq!(sub_names.len(), 11);
    }
}
