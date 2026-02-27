//! SMS-Activate CLI command — SMS activation services.
//!
//! Subcommands: balance, buy, status.

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
        .subcommand(Command::new("balance").about("Check account balance"))
        .subcommand(
            Command::new("buy")
                .about("Buy a phone number for activation")
                .arg(
                    Arg::new("service")
                        .required(true)
                        .help("Service to activate"),
                )
                .arg(
                    Arg::new("country")
                        .long("country")
                        .short('c')
                        .help("Country code"),
                ),
        )
        .subcommand(
            Command::new("status")
                .about("Check activation status")
                .arg(
                    Arg::new("id")
                        .required(true)
                        .help("Activation ID"),
                ),
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
            "SMS_ACTIVATE_API_KEY not set. Set the environment variable to use sms-activate."
                .into(),
        )
    })
}

/// Handle the `sms-activate` command.
pub async fn handle_sms_activate(
    matches: &ArgMatches,
    http: &HttpClient,
) -> Result<()> {
    let api_key = get_api_key()?;
    let client = http.get_client().await;

    match matches.subcommand() {
        Some(("balance", _)) => {
            let url = format!(
                "{}?api_key={}&action=getBalance",
                SMS_ACTIVATE_API_URL, api_key
            );
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| BosuaError::Command(format!("Failed to check balance: {}", e)))?;
            let body = resp
                .text()
                .await
                .map_err(|e| BosuaError::Command(format!("Failed to read response: {}", e)))?;

            // Response format: ACCESS_BALANCE:123.45
            if let Some(balance) = body.strip_prefix("ACCESS_BALANCE:") {
                println!("Balance: {} RUB", balance.trim());
            } else {
                println!("{}", body);
            }
            Ok(())
        }
        Some(("buy", sub)) => {
            let service = sub.get_one::<String>("service").unwrap();
            let country = sub
                .get_one::<String>("country")
                .map(|s| s.as_str())
                .unwrap_or("0");

            let url = format!(
                "{}?api_key={}&action=getNumber&service={}&country={}",
                SMS_ACTIVATE_API_URL, api_key, service, country
            );
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| BosuaError::Command(format!("Failed to buy number: {}", e)))?;
            let body = resp
                .text()
                .await
                .map_err(|e| BosuaError::Command(format!("Failed to read response: {}", e)))?;

            // Response format: ACCESS_NUMBER:id:number
            if body.starts_with("ACCESS_NUMBER:") {
                let parts: Vec<&str> = body.splitn(3, ':').collect();
                if parts.len() == 3 {
                    println!("Activation ID: {}", parts[1]);
                    println!("Phone number: {}", parts[2].trim());
                } else {
                    println!("{}", body);
                }
            } else {
                return Err(BosuaError::Command(format!(
                    "Failed to buy number: {}",
                    body
                )));
            }
            Ok(())
        }
        Some(("status", sub)) => {
            let id = sub.get_one::<String>("id").unwrap();
            let url = format!(
                "{}?api_key={}&action=getStatus&id={}",
                SMS_ACTIVATE_API_URL, api_key, id
            );
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| BosuaError::Command(format!("Failed to check status: {}", e)))?;
            let body = resp
                .text()
                .await
                .map_err(|e| BosuaError::Command(format!("Failed to read response: {}", e)))?;

            // Response format: STATUS_OK:code or STATUS_WAIT_CODE etc.
            if body.starts_with("STATUS_OK:") {
                let code = body.strip_prefix("STATUS_OK:").unwrap_or("").trim();
                println!("Activation code received: {}", code);
            } else if body.starts_with("STATUS_WAIT_CODE") {
                println!("Waiting for SMS code...");
            } else if body.starts_with("STATUS_CANCEL") {
                println!("Activation cancelled");
            } else {
                println!("Status: {}", body.trim());
            }
            Ok(())
        }
        _ => {
            println!("sms-activate: use a subcommand (balance, buy, status)");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sms_activate_command_parses_balance() {
        let cmd = sms_activate_command();
        let matches = cmd.try_get_matches_from(["smsactivate", "balance"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("balance"));
    }

    #[test]
    fn test_sms_activate_command_parses_buy() {
        let cmd = sms_activate_command();
        let matches = cmd
            .try_get_matches_from(["smsactivate", "buy", "telegram", "--country", "US"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "buy");
        assert_eq!(sub.get_one::<String>("service").map(|s| s.as_str()), Some("telegram"));
        assert_eq!(sub.get_one::<String>("country").map(|s| s.as_str()), Some("US"));
    }

    #[test]
    fn test_sms_activate_command_parses_status() {
        let cmd = sms_activate_command();
        let matches = cmd
            .try_get_matches_from(["smsactivate", "status", "12345"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "status");
        assert_eq!(sub.get_one::<String>("id").map(|s| s.as_str()), Some("12345"));
    }

    #[test]
    fn test_sms_activate_buy_requires_service() {
        let cmd = sms_activate_command();
        let result = cmd.try_get_matches_from(["smsactivate", "buy"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_sms_activate_meta() {
        let meta = sms_activate_meta();
        assert_eq!(meta.name, "smsactivate");
        assert_eq!(meta.category, CommandCategory::Utility);
        assert!(!meta.description.is_empty());
    }
}
