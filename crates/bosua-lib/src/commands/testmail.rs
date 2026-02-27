//! Testmail CLI command — temporary email utilities.
//!
//! Subcommands: create, inbox, read.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};
use crate::http_client::HttpClient;

/// Testmail.app API base URL.
const TESTMAIL_API_URL: &str = "https://api.testmail.app/api/json";

/// Build the `testmail` clap command.
pub fn testmail_command() -> Command {
    Command::new("testmail")
        .about("Temporary email utilities")
        .subcommand(Command::new("create").about("Create a temporary email address"))
        .subcommand(
            Command::new("inbox")
                .about("List inbox messages")
                .arg(
                    Arg::new("email")
                        .required(true)
                        .help("Email address to check"),
                ),
        )
        .subcommand(
            Command::new("read")
                .about("Read a specific email message")
                .arg(
                    Arg::new("message-id")
                        .required(true)
                        .help("Message ID to read"),
                ),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn testmail_meta() -> CommandMeta {
    CommandBuilder::from_clap(testmail_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `testmail` command.
pub async fn handle_testmail(
    matches: &ArgMatches,
    http: &HttpClient,
) -> Result<()> {
    let api_key = std::env::var("TESTMAIL_API_KEY").map_err(|_| {
        BosuaError::Command(
            "TESTMAIL_API_KEY not set. Set the environment variable to use testmail.".into(),
        )
    })?;

    let namespace = std::env::var("TESTMAIL_NAMESPACE").unwrap_or_else(|_| "bosua".into());

    match matches.subcommand() {
        Some(("create", _)) => {
            let tag = format!("{}.{}", namespace, chrono::Utc::now().timestamp());
            let email = format!("{}.{}@inbox.testmail.app", namespace, tag);
            println!("Created temporary email: {}", email);
            println!("Tag: {}", tag);
            Ok(())
        }
        Some(("inbox", sub)) => {
            let email = sub.get_one::<String>("email").unwrap();
            let tag = email
                .split('@')
                .next()
                .and_then(|local| local.strip_prefix(&format!("{}.", namespace)))
                .unwrap_or(email);

            let url = format!(
                "{}?apikey={}&namespace={}&tag={}",
                TESTMAIL_API_URL, api_key, namespace, tag
            );

            let client = http.get_client().await;
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| BosuaError::Command(format!("Failed to fetch inbox: {}", e)))?;

            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| BosuaError::Command(format!("Failed to parse response: {}", e)))?;

            if let Some(emails) = body.get("emails").and_then(|v| v.as_array()) {
                if emails.is_empty() {
                    println!("No messages in inbox for {}", email);
                } else {
                    for msg in emails {
                        let from = msg.get("from").and_then(|v| v.as_str()).unwrap_or("unknown");
                        let subject = msg.get("subject").and_then(|v| v.as_str()).unwrap_or("(no subject)");
                        let id = msg.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        println!("[{}] From: {} — {}", id, from, subject);
                    }
                }
            } else {
                println!("No messages found");
            }
            Ok(())
        }
        Some(("read", sub)) => {
            let msg_id = sub.get_one::<String>("message-id").unwrap();
            let url = format!(
                "{}?apikey={}&namespace={}&message_id={}",
                TESTMAIL_API_URL, api_key, namespace, msg_id
            );

            let client = http.get_client().await;
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| BosuaError::Command(format!("Failed to fetch message: {}", e)))?;

            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| BosuaError::Command(format!("Failed to parse response: {}", e)))?;

            if let Some(emails) = body.get("emails").and_then(|v| v.as_array()) {
                if let Some(msg) = emails.first() {
                    let from = msg.get("from").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let subject = msg.get("subject").and_then(|v| v.as_str()).unwrap_or("(no subject)");
                    let text = msg.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    println!("From: {}", from);
                    println!("Subject: {}", subject);
                    println!("---");
                    println!("{}", text);
                } else {
                    println!("Message not found: {}", msg_id);
                }
            } else {
                println!("Message not found: {}", msg_id);
            }
            Ok(())
        }
        _ => {
            println!("testmail: use a subcommand (create, inbox, read)");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_testmail_command_parses_create() {
        let cmd = testmail_command();
        let matches = cmd.try_get_matches_from(["testmail", "create"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("create"));
    }

    #[test]
    fn test_testmail_command_parses_inbox() {
        let cmd = testmail_command();
        let matches = cmd
            .try_get_matches_from(["testmail", "inbox", "test@example.com"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "inbox");
        assert_eq!(
            sub.get_one::<String>("email").map(|s| s.as_str()),
            Some("test@example.com"),
        );
    }

    #[test]
    fn test_testmail_command_parses_read() {
        let cmd = testmail_command();
        let matches = cmd
            .try_get_matches_from(["testmail", "read", "msg-42"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "read");
        assert_eq!(
            sub.get_one::<String>("message-id").map(|s| s.as_str()),
            Some("msg-42"),
        );
    }

    #[test]
    fn test_testmail_inbox_requires_email() {
        let cmd = testmail_command();
        let result = cmd.try_get_matches_from(["testmail", "inbox"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_testmail_meta() {
        let meta = testmail_meta();
        assert_eq!(meta.name, "testmail");
        assert_eq!(meta.category, CommandCategory::Utility);
        assert!(!meta.description.is_empty());
    }
}
