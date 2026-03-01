//! Bitcoin CLI command — Bitcoin operations.
//!
//! Subcommands: price, signet.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::Result;
use crate::http_client::HttpClient;

/// Build the `bitcoin` clap command.
pub fn bitcoin_command() -> Command {
    Command::new("bitcoin")
        .about("Bitcoin operations")
        .aliases(["btc"])
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(
            Arg::new("scraperapi")
                .long("scraperapi")
                .default_value("7a469cce138ed6bbdf9cdaf466767d11")
                .help("Scraper API Key"),
        )
        .subcommand(
            Command::new("price")
                .about("Get price in USDT of a coin")
                .arg(Arg::new("coin").help("Coin symbol (e.g. BTC, ETH)")),
        )
        .subcommand(
            Command::new("signet")
                .about("Get Bitcoin signet coins"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn bitcoin_meta() -> CommandMeta {
    CommandBuilder::from_clap(bitcoin_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `bitcoin` command.
pub async fn handle_bitcoin(matches: &ArgMatches, http: &HttpClient) -> Result<()> {
    let _scraperapi = matches.get_one::<String>("scraperapi").unwrap();

    match matches.subcommand() {
        Some(("price", sub)) => {
            let coin = sub.get_one::<String>("coin").map(|s| s.as_str()).unwrap_or("BTC");
            let client = http.get_client().await;
            let url = format!("https://api.binance.com/api/v3/ticker/price?symbol={}USDT", coin.to_uppercase());
            let resp = client.get(&url).send().await
                .map_err(|e| crate::errors::BosuaError::Command(format!("Failed to fetch price: {}", e)))?;
            let body: serde_json::Value = resp.json().await
                .map_err(|e| crate::errors::BosuaError::Command(format!("Failed to parse response: {}", e)))?;
            if let Some(price) = body.get("price").and_then(|v| v.as_str()) {
                println!("{}: {} USDT", coin.to_uppercase(), price);
            } else {
                println!("Could not fetch price for {}", coin);
            }
            Ok(())
        }
        Some(("signet", _)) => {
            println!("bitcoin signet: not yet implemented");
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitcoin_command_parses() {
        let cmd = bitcoin_command();
        let m = cmd.try_get_matches_from(["bitcoin", "price"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("price"));
    }

    #[test]
    fn test_bitcoin_alias_btc() {
        let cmd = bitcoin_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"btc"));
    }

    #[test]
    fn test_bitcoin_price_with_coin() {
        let cmd = bitcoin_command();
        let m = cmd.try_get_matches_from(["bitcoin", "price", "ETH"]).unwrap();
        let (name, sub) = m.subcommand().unwrap();
        assert_eq!(name, "price");
        assert_eq!(sub.get_one::<String>("coin").map(|s| s.as_str()), Some("ETH"));
    }

    #[test]
    fn test_bitcoin_scraperapi_flag() {
        let cmd = bitcoin_command();
        let m = cmd.try_get_matches_from(["bitcoin", "--scraperapi", "mykey", "price"]).unwrap();
        assert_eq!(m.get_one::<String>("scraperapi").map(|s| s.as_str()), Some("mykey"));
    }

    #[test]
    fn test_bitcoin_requires_subcommand() {
        let cmd = bitcoin_command();
        assert!(cmd.try_get_matches_from(["bitcoin"]).is_err());
    }

    #[test]
    fn test_bitcoin_meta() {
        let meta = bitcoin_meta();
        assert_eq!(meta.name, "bitcoin");
        assert_eq!(meta.category, CommandCategory::Utility);
    }
}
