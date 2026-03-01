//! Testmail CLI command — TestMail.app operations.
//!
//! Subcommands: get, list.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};
use crate::http_client::HttpClient;

/// Testmail.app API base URL.
const TESTMAIL_API_URL: &str = "https://api.testmail.app/api/json";

/// Build the `testmail` clap command.
pub fn testmail_command() -> Command {
    Command::new("testmail")
        .about("TestMail.app operations")
        .aliases(["tm", "tmail"])
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(Arg::new("from").long("from").default_value("10").help("Get email from this time onward (minutes)"))
        .arg(Arg::new("type").long("type").default_value("link").help("Type of info: OTP, link"))
        .subcommand(Command::new("get").about("New/get info TestMail.app account"))
        .subcommand(Command::new("list").about("List Testmail.app emails"))
}

/// Build the `CommandMeta` for registry registration.
pub fn testmail_meta() -> CommandMeta {
    CommandBuilder::from_clap(testmail_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `testmail` command.
pub async fn handle_testmail(matches: &ArgMatches, http: &HttpClient) -> Result<()> {
    let api_key = std::env::var("TESTMAIL_API_KEY").map_err(|_| {
        BosuaError::Command("TESTMAIL_API_KEY not set.".into())
    })?;
    let namespace = std::env::var("TESTMAIL_NAMESPACE").unwrap_or_else(|_| "bosua".into());
    let _from = matches.get_one::<String>("from").unwrap();
    let _info_type = matches.get_one::<String>("type").unwrap();

    match matches.subcommand() {
        Some(("get", _)) => {
            let tag = format!("{}.{}", namespace, chrono::Utc::now().timestamp());
            let email = format!("{}.{}@inbox.testmail.app", namespace, tag);
            println!("Created temporary email: {}", email);
            println!("Tag: {}", tag);
            Ok(())
        }
        Some(("list", _)) => {
            let url = format!(
                "{}?apikey={}&namespace={}",
                TESTMAIL_API_URL, api_key, namespace
            );
            let client = http.get_client().await;
            let resp = client.get(&url).send().await
                .map_err(|e| BosuaError::Command(format!("Failed to fetch emails: {}", e)))?;
            let body: serde_json::Value = resp.json().await
                .map_err(|e| BosuaError::Command(format!("Failed to parse response: {}", e)))?;

            if let Some(emails) = body.get("emails").and_then(|v| v.as_array()) {
                if emails.is_empty() {
                    println!("No emails found");
                } else {
                    for msg in emails {
                        let from = msg.get("from").and_then(|v| v.as_str()).unwrap_or("unknown");
                        let subject = msg.get("subject").and_then(|v| v.as_str()).unwrap_or("(no subject)");
                        let id = msg.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        println!("[{}] From: {} — {}", id, from, subject);
                    }
                }
            } else {
                println!("No emails found");
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
    fn test_testmail_command_parses() {
        let cmd = testmail_command();
        let m = cmd.try_get_matches_from(["testmail", "get"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("get"));
    }

    #[test]
    fn test_testmail_alias_tm() {
        let cmd = testmail_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"tm"));
        assert!(aliases.contains(&"tmail"));
    }

    #[test]
    fn test_testmail_list() {
        let cmd = testmail_command();
        let m = cmd.try_get_matches_from(["testmail", "list"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("list"));
    }

    #[test]
    fn test_testmail_persistent_flags() {
        let cmd = testmail_command();
        let m = cmd.try_get_matches_from(["testmail", "--from", "30", "--type", "OTP", "get"]).unwrap();
        assert_eq!(m.get_one::<String>("from").map(|s| s.as_str()), Some("30"));
        assert_eq!(m.get_one::<String>("type").map(|s| s.as_str()), Some("OTP"));
    }

    #[test]
    fn test_testmail_requires_subcommand() {
        let cmd = testmail_command();
        assert!(cmd.try_get_matches_from(["testmail"]).is_err());
    }

    #[test]
    fn test_testmail_meta() {
        let meta = testmail_meta();
        assert_eq!(meta.name, "testmail");
        assert_eq!(meta.category, CommandCategory::Utility);
    }
}
