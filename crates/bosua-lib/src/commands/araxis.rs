//! Araxis CLI command — Araxis Merge integration.
//!
//! Launches Araxis Merge (or falls back to diff) for file comparison.

use clap::{ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::Result;

/// Build the `araxis` clap command.
pub fn araxis_command() -> Command {
    Command::new("araxis")
        .about("Araxis Merge stuffs")
        .aliases(["a"])
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("register").about("Register Araxis with auto-generated credentials"))
}

/// Build the `CommandMeta` for registry registration.
pub fn araxis_meta() -> CommandMeta {
    CommandBuilder::from_clap(araxis_command())
        .category(CommandCategory::Developer)
        .build()
}

/// Handle the `araxis` command.
///
/// Registers Araxis Merge using testmail.app API for email verification
/// and AppleScript for clipboard/browser automation.
pub async fn handle_araxis(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("register", _)) => {
            use crate::errors::BosuaError;

            // Generate a random email tag for testmail
            let tag = format!("araxis-{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis());

            let email = format!("{}.phongblack@inbox.testmail.app", tag);
            println!("Registration email: {}", email);

            // Copy email to clipboard via pbcopy
            let mut child = std::process::Command::new("pbcopy")
                .stdin(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| BosuaError::Command(format!("Failed to run pbcopy: {}", e)))?;
            if let Some(ref mut stdin) = child.stdin {
                use std::io::Write;
                let _ = stdin.write_all(email.as_bytes());
            }
            let _ = child.wait();
            println!("Email copied to clipboard");

            // Open Araxis registration page
            let reg_url = "https://www.araxis.com/merge/register";
            let _ = std::process::Command::new("open").arg(reg_url).status();
            println!("Opened {} in browser", reg_url);
            println!("Complete the registration form with the email above, then press Enter to check for the license key...");

            // Wait for user
            let mut buf = String::new();
            let _ = std::io::stdin().read_line(&mut buf);

            // Poll testmail API for the registration email
            println!("Checking for registration email...");
            let api_key = "3fb58953-1366-71b0-3019-ebb7b64a9835";
            let namespace = "phongblack";
            let testmail_url = format!(
                "https://api.testmail.app/api/json?apikey={}&namespace={}&tag={}&livequery=true&timestamp_from={}",
                api_key, namespace, tag,
                (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() - 300) * 1000
            );

            let client = reqwest::Client::new();
            for attempt in 1..=12 {
                let resp = client.get(&testmail_url).send().await
                    .map_err(|e| BosuaError::Command(format!("Testmail API error: {}", e)))?;
                let body: serde_json::Value = resp.json().await
                    .map_err(|e| BosuaError::Command(format!("Failed to parse testmail response: {}", e)))?;

                if let Some(emails) = body.get("emails").and_then(|v| v.as_array()) {
                    if let Some(email_obj) = emails.first() {
                        let text = email_obj.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        let html = email_obj.get("html").and_then(|v| v.as_str()).unwrap_or("");
                        let content = if !text.is_empty() { text } else { html };
                        println!("Registration email received:");
                        println!("{}", content);
                        return Ok(());
                    }
                }

                if attempt < 12 {
                    println!("  Attempt {}/12 - no email yet, waiting 10s...", attempt);
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                }
            }

            println!("No registration email received after 2 minutes. Check your Araxis registration.");
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_araxis_command_parses_register() {
        let cmd = araxis_command();
        let matches = cmd.try_get_matches_from(["araxis", "register"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("register"));
    }

    #[test]
    fn test_araxis_requires_subcommand() {
        let cmd = araxis_command();
        let result = cmd.try_get_matches_from(["araxis"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_araxis_alias() {
        let cmd = araxis_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"a"));
    }

    #[test]
    fn test_araxis_meta() {
        let meta = araxis_meta();
        assert_eq!(meta.name, "araxis");
        assert_eq!(meta.category, CommandCategory::Developer);
    }
}
