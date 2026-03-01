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
            // Get active activations, find the latest, cancel it
            let url = format!("{}?api_key={}&action=getActiveActivations", SMS_ACTIVATE_API_URL, api_key);
            let resp = client.get(&url).send().await
                .map_err(|e| BosuaError::Command(format!("Failed to get activations: {}", e)))?;
            let body = resp.text().await
                .map_err(|e| BosuaError::Command(format!("Failed to read response: {}", e)))?;

            // Parse response to find activation ID
            let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
            let activations = parsed.get("activeActivations");
            let activation_id = match activations {
                Some(serde_json::Value::Array(arr)) => {
                    arr.first()
                        .and_then(|a| a.get("activationId"))
                        .and_then(|v| v.as_str().or_else(|| v.as_i64().map(|_| "")))
                        .map(|s| s.to_string())
                        .or_else(|| arr.first().and_then(|a| a.get("activationId")).and_then(|v| v.as_i64()).map(|n| n.to_string()))
                }
                Some(serde_json::Value::Object(map)) => {
                    map.values().next()
                        .and_then(|a| a.get("activationId"))
                        .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|n| n.to_string())))
                }
                _ => None,
            };

            match activation_id {
                Some(id) => {
                    let cancel_url = format!("{}?api_key={}&action=setStatus&id={}&status=-1", SMS_ACTIVATE_API_URL, api_key, id);
                    let cancel_resp = client.get(&cancel_url).send().await
                        .map_err(|e| BosuaError::Command(format!("Failed to cancel: {}", e)))?;
                    let cancel_body = cancel_resp.text().await.unwrap_or_default();
                    println!("{}", cancel_body);
                }
                None => {
                    println!("No active activations found to cancel");
                }
            }
            Ok(())
        }
        Some(("check", _)) => {
            // Poll active activations for SMS code
            let url = format!("{}?api_key={}&action=getActiveActivations", SMS_ACTIVATE_API_URL, api_key);
            let resp = client.get(&url).send().await
                .map_err(|e| BosuaError::Command(format!("Failed to get activations: {}", e)))?;
            let body = resp.text().await
                .map_err(|e| BosuaError::Command(format!("Failed to read response: {}", e)))?;

            let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
            let activations = parsed.get("activeActivations");

            // Look for smsCode in any activation
            let mut found_code = false;
            if let Some(acts) = activations {
                let items: Vec<&serde_json::Value> = match acts {
                    serde_json::Value::Array(arr) => arr.iter().collect(),
                    serde_json::Value::Object(map) => map.values().collect(),
                    _ => vec![],
                };
                for item in items {
                    if let Some(code) = item.get("smsCode").and_then(|v| v.as_str()) {
                        if !code.is_empty() {
                            let phone = item.get("phoneNumber").and_then(|v| v.as_str()).unwrap_or("unknown");
                            println!("Phone: {} -> Code: {}", phone, code);
                            found_code = true;
                        }
                    }
                }
            }
            if !found_code {
                println!("No SMS codes received yet. Waiting...");
            }
            Ok(())
        }
        Some(("generate", _)) => {
            // Generate country/service maps from API
            let countries_url = format!("{}?api_key={}&action=getCountries", SMS_ACTIVATE_API_URL, api_key);
            let resp = client.get(&countries_url).send().await
                .map_err(|e| BosuaError::Command(format!("Failed to get countries: {}", e)))?;
            let body = resp.text().await
                .map_err(|e| BosuaError::Command(format!("Failed to read response: {}", e)))?;
            println!("Countries data:");
            println!("{}", body);
            Ok(())
        }
        Some(("list", sub)) => {
            match sub.subcommand() {
                Some(("country", _)) => {
                    let url = format!("{}?api_key={}&action=getCountries", SMS_ACTIVATE_API_URL, api_key);
                    let resp = client.get(&url).send().await
                        .map_err(|e| BosuaError::Command(format!("Failed to get countries: {}", e)))?;
                    let body: serde_json::Value = resp.json().await
                        .map_err(|e| BosuaError::Command(format!("Failed to parse response: {}", e)))?;
                    if let Some(obj) = body.as_object() {
                        println!("{:<6} {}", "ID", "Country");
                        let mut entries: Vec<_> = obj.iter().collect();
                        entries.sort_by_key(|(k, _)| k.parse::<i64>().unwrap_or(0));
                        for (id, info) in entries {
                            let name = info.get("eng").and_then(|v| v.as_str()).unwrap_or("Unknown");
                            println!("{:<6} {}", id, name);
                        }
                    } else {
                        println!("{}", serde_json::to_string_pretty(&body).unwrap_or_default());
                    }
                }
                Some(("service", _)) => {
                    let url = format!("{}?api_key={}&action=getServicesList", SMS_ACTIVATE_API_URL, api_key);
                    let resp = client.get(&url).send().await
                        .map_err(|e| BosuaError::Command(format!("Failed to get services: {}", e)))?;
                    let body: serde_json::Value = resp.json().await
                        .map_err(|e| BosuaError::Command(format!("Failed to parse response: {}", e)))?;
                    if let Some(services) = body.get("services") {
                        if let Some(arr) = services.as_array() {
                            println!("{:<8} {}", "Code", "Name");
                            for svc in arr {
                                let code = svc.get("code").and_then(|v| v.as_str()).unwrap_or("");
                                let name = svc.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                println!("{:<8} {}", code, name);
                            }
                        } else {
                            println!("{}", serde_json::to_string_pretty(services).unwrap_or_default());
                        }
                    } else {
                        println!("{}", serde_json::to_string_pretty(&body).unwrap_or_default());
                    }
                }
                _ => {
                    println!("smsactivate list: use a subcommand (country, service)");
                }
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
