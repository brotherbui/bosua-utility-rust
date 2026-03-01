//! SMS-Activate CLI command — SMS activation services.
//!
//! Subcommands: balance, cancel, check, generate, list, order.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};
use crate::http_client::HttpClient;

/// SMS-Activate API base URL.
const SMS_ACTIVATE_API_URL: &str = "https://api.sms-activate.org/stubs/handler_api.php";

/// Build the `smsactivate` clap command.
pub fn sms_activate_command() -> Command {
    Command::new("smsactivate")
        .aliases(["sms"])
        .about("Sms-Activate operations")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(Arg::new("services").long("services").action(clap::ArgAction::SetTrue).help("Get service list"))
        .arg(Arg::new("top").long("top").action(clap::ArgAction::SetTrue).help("Get top country by service"))
        .subcommand(Command::new("balance").about("Check account balance"))
        .subcommand(
            Command::new("cancel")
                .about("Cancel the ordered number")
                .aliases(["c"]),
        )
        .subcommand(
            Command::new("check")
                .about("Check SMS")
                .aliases(["ch"]),
        )
        .subcommand(
            Command::new("generate")
                .about("Generate")
                .aliases(["gen", "g"]),
        )
        .subcommand(
            Command::new("list")
                .about("List services/countries")
                .aliases(["l"])
                .subcommand(Command::new("country").about("List countries"))
                .subcommand(Command::new("service").about("List services")),
        )
        .subcommand(
            Command::new("order")
                .about("Order a new number")
                .aliases(["register", "r", "new", "n", "buy", "b", "o"])
                .arg(Arg::new("country").long("country").default_value("Rusia").help("Country Name"))
                .arg(Arg::new("service").long("service").default_value("ot").help("Service code")),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn sms_activate_meta() -> CommandMeta {
    CommandBuilder::from_clap(sms_activate_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Get the API key from environment.
fn get_api_key() -> Result<String> {
    std::env::var("SMS_ACTIVATE_API_KEY").map_err(|_| {
        BosuaError::Command(
            "SMS_ACTIVATE_API_KEY not set. Set the environment variable to use sms-activate.".into(),
        )
    })
}

/// Handle the `smsactivate` command.
pub async fn handle_sms_activate(matches: &ArgMatches, http: &HttpClient) -> Result<()> {
    let api_key = get_api_key()?;
    let client = http.get_client().await;

    match matches.subcommand() {
        Some(("balance", _)) => {
            let url = format!("{}?api_key={}&action=getBalance", SMS_ACTIVATE_API_URL, api_key);
            let resp = client.get(&url).send().await
                .map_err(|e| BosuaError::Command(format!("Failed to check balance: {}", e)))?;
            let body = resp.text().await
                .map_err(|e| BosuaError::Command(format!("Failed to read response: {}", e)))?;
            if let Some(balance) = body.strip_prefix("ACCESS_BALANCE:") {
                println!("Balance: {} RUB", balance.trim());
            } else {
                println!("{}", body);
            }
            Ok(())
        }
        Some(("cancel", _)) => {
            println!("smsactivate cancel: not yet implemented");
            Ok(())
        }
        Some(("check", _)) => {
            println!("smsactivate check: not yet implemented");
            Ok(())
        }
        Some(("generate", _)) => {
            println!("smsactivate generate: not yet implemented");
            Ok(())
        }
        Some(("list", sub)) => {
            match sub.subcommand() {
                Some(("country", _)) => println!("smsactivate list country: not yet implemented"),
                Some(("service", _)) => println!("smsactivate list service: not yet implemented"),
                _ => println!("smsactivate list: use a subcommand (country, service)"),
            }
            Ok(())
        }
        Some(("order", sub)) => {
            let country = sub.get_one::<String>("country").unwrap();
            let service = sub.get_one::<String>("service").unwrap();
            let url = format!(
                "{}?api_key={}&action=getNumber&service={}&country={}",
                SMS_ACTIVATE_API_URL, api_key, service, country
            );
            let resp = client.get(&url).send().await
                .map_err(|e| BosuaError::Command(format!("Failed to order number: {}", e)))?;
            let body = resp.text().await
                .map_err(|e| BosuaError::Command(format!("Failed to read response: {}", e)))?;
            if body.starts_with("ACCESS_NUMBER:") {
                let parts: Vec<&str> = body.splitn(3, ':').collect();
                if parts.len() == 3 {
                    println!("Activation ID: {}", parts[1]);
                    println!("Phone number: {}", parts[2].trim());
                } else {
                    println!("{}", body);
                }
            } else {
                return Err(BosuaError::Command(format!("Failed to order number: {}", body)));
            }
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sms_activate_command_parses() {
        let cmd = sms_activate_command();
        let m = cmd.try_get_matches_from(["smsactivate", "balance"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("balance"));
    }

    #[test]
    fn test_sms_activate_order() {
        let cmd = sms_activate_command();
        let m = cmd.try_get_matches_from(["smsactivate", "order", "--country", "US", "--service", "tg"]).unwrap();
        let (name, sub) = m.subcommand().unwrap();
        assert_eq!(name, "order");
        assert_eq!(sub.get_one::<String>("country").map(|s| s.as_str()), Some("US"));
        assert_eq!(sub.get_one::<String>("service").map(|s| s.as_str()), Some("tg"));
    }

    #[test]
    fn test_sms_activate_order_alias_buy() {
        let cmd = sms_activate_command();
        let m = cmd.try_get_matches_from(["smsactivate", "buy"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("order"));
    }

    #[test]
    fn test_sms_activate_requires_subcommand() {
        let cmd = sms_activate_command();
        assert!(cmd.try_get_matches_from(["smsactivate"]).is_err());
    }

    #[test]
    fn test_sms_activate_meta() {
        let meta = sms_activate_meta();
        assert_eq!(meta.name, "smsactivate");
        assert_eq!(meta.category, CommandCategory::Utility);
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = sms_activate_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        for name in &["balance", "cancel", "check", "generate", "list", "order"] {
            assert!(sub_names.contains(name), "missing: {}", name);
        }
        assert_eq!(sub_names.len(), 6);
    }
}
