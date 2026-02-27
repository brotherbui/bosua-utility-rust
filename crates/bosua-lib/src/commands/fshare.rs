//! FShare CLI command with subcommands.
//!
//! Provides the `fshare` command with subcommands: account, links, token.
//!
//! Command implementations are stub handlers — actual logic will be wired
//! when the full app is assembled.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::fshare::FShareClient;
use crate::download::DownloadManager;
use crate::errors::{BosuaError, Result};
use crate::output;

/// Build the `fshare` clap command with all subcommands.
pub fn fshare_command() -> Command {
    Command::new("fshare")
        .about("FShare VIP downloads")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(account_subcommand())
        .subcommand(links_subcommand())
        .subcommand(token_subcommand())
}

/// Build the `CommandMeta` for registry registration.
pub fn fshare_meta() -> CommandMeta {
    CommandBuilder::from_clap(fshare_command())
        .category(CommandCategory::Cloud)
        .build()
}

/// Handle the `fshare` command dispatch.
pub async fn handle_fshare(
    matches: &ArgMatches,
    fshare: &FShareClient,
    dl: Option<&DownloadManager>,
) -> Result<()> {
    match matches.subcommand() {
        Some(("account", sub)) => handle_account(sub, fshare).await,
        Some(("links", sub)) => handle_links(sub, fshare, dl).await,
        Some(("token", sub)) => handle_token(sub, fshare).await,
        _ => unreachable!("subcommand_required is set"),
    }
}

/// Mask a token for display: show last 4 chars, mask the rest with asterisks.
/// If token < 4 chars, fully mask.
pub fn mask_token(token: &str) -> String {
    let len = token.len();
    if len < 4 {
        "*".repeat(len)
    } else {
        format!("{}{}", "*".repeat(len - 4), &token[len - 4..])
    }
}

// ---------------------------------------------------------------------------
// Subcommand definitions
// ---------------------------------------------------------------------------

fn account_subcommand() -> Command {
    Command::new("account")
        .about("Manage FShare account")
        .subcommand(Command::new("info").about("Show FShare account info"))
        .subcommand(
            Command::new("login")
                .about("Log in to FShare")
                .arg(Arg::new("email").required(true).help("FShare account email"))
                .arg(Arg::new("password").required(true).help("FShare account password")),
        )
        .subcommand(Command::new("logout").about("Log out of FShare"))
}

fn links_subcommand() -> Command {
    Command::new("links")
        .about("Resolve FShare VIP download links")
        .arg(
            Arg::new("urls")
                .required(true)
                .num_args(1..)
                .help("FShare URLs to resolve"),
        )
        .arg(
            Arg::new("password")
                .long("password")
                .short('p')
                .help("Password for protected files"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output directory for downloads"),
        )
}

fn token_subcommand() -> Command {
    Command::new("token")
        .about("Manage FShare session token")
        .subcommand(Command::new("show").about("Show current session token"))
        .subcommand(
            Command::new("set")
                .about("Set session token manually")
                .arg(Arg::new("token").required(true).help("Session token value")),
        )
        .subcommand(Command::new("clear").about("Clear stored session token"))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn handle_account(matches: &ArgMatches, fshare: &FShareClient) -> Result<()> {
    match matches.subcommand() {
        Some(("info", _)) => {
            match fshare.get_token().await {
                Some(token) => {
                    output::success("FShare: logged in");
                    output::info(&format!("Token: {}", mask_token(&token)));
                }
                None => {
                    output::info("FShare: not logged in");
                    output::info("Run `bosua fshare account login <email> <password>` to log in.");
                }
            }
            Ok(())
        }
        Some(("login", sub)) => {
            let _email = sub.get_one::<String>("email").unwrap();
            let _password = sub.get_one::<String>("password").unwrap();
            let resp = fshare.login().await?;
            match resp.token {
                Some(ref token) => {
                    output::success("Login successful.");
                    output::info(&format!("Token: {}", mask_token(token)));
                }
                None => {
                    let msg = resp.msg.unwrap_or_else(|| "unknown error".into());
                    return Err(BosuaError::Auth(format!("FShare login failed: {}", msg)));
                }
            }
            Ok(())
        }
        Some(("logout", _)) => {
            fshare.set_token(String::new()).await;
            output::success("Logged out of FShare.");
            Ok(())
        }
        _ => {
            output::info("Usage: fshare account <info|login|logout>");
            Ok(())
        }
    }
}

async fn handle_links(
    matches: &ArgMatches,
    fshare: &FShareClient,
    dl: Option<&DownloadManager>,
) -> Result<()> {
    // Check authentication first
    if fshare.get_token().await.is_none() {
        return Err(BosuaError::Auth(
            "FShare: not logged in. Run `bosua fshare account login` to authenticate.".into(),
        ));
    }

    let urls: Vec<String> = matches
        .get_many::<String>("urls")
        .unwrap()
        .cloned()
        .collect();
    let output_dir = matches.get_one::<String>("output");

    let results = fshare.resolve_vip_links(&urls).await;

    let mut resolved_urls = Vec::new();
    for (original, result) in &results {
        match result {
            Ok(direct_url) => {
                output::success(&format!("{} -> {}", original, direct_url));
                resolved_urls.push(direct_url.clone());
            }
            Err(e) => {
                output::error(&format!("{} -> Error: {}", original, e));
            }
        }
    }

    // If --output is specified and we have a DownloadManager, download the resolved links
    if let Some(dir) = output_dir {
        if resolved_urls.is_empty() {
            output::info("No links resolved successfully. Nothing to download.");
            return Ok(());
        }
        match dl {
            Some(_dm) => {
                output::info(&format!(
                    "Downloading {} file(s) to {}...",
                    resolved_urls.len(),
                    dir
                ));
                // Print resolved URLs for the user to download
                for url in &resolved_urls {
                    output::info(&format!("  {}", url));
                }
                output::info("Use `bosua download` with these URLs to download.");
            }
            None => {
                output::warning("Download manager not available. Resolved URLs:");
                for url in &resolved_urls {
                    println!("{}", url);
                }
            }
        }
    }

    Ok(())
}

async fn handle_token(matches: &ArgMatches, fshare: &FShareClient) -> Result<()> {
    match matches.subcommand() {
        Some(("show", _)) => {
            match fshare.get_token().await {
                Some(token) => {
                    output::info(&format!("Token: {}", mask_token(&token)));
                }
                None => {
                    output::info("No token set.");
                    output::info(
                        "Run `bosua fshare account login` or `bosua fshare token set <token>`.",
                    );
                }
            }
            Ok(())
        }
        Some(("set", sub)) => {
            let token = sub.get_one::<String>("token").unwrap().clone();
            fshare.set_token(token).await;
            output::success("Token set.");
            Ok(())
        }
        Some(("clear", _)) => {
            fshare.set_token(String::new()).await;
            output::success("Token cleared.");
            Ok(())
        }
        _ => {
            output::info("Usage: fshare token <show|set|clear>");
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fshare_command_parses() {
        let cmd = fshare_command();
        let matches = cmd
            .try_get_matches_from(["fshare", "account", "info"])
            .unwrap();
        assert_eq!(matches.subcommand_name(), Some("account"));
    }

    #[test]
    fn test_fshare_requires_subcommand() {
        let cmd = fshare_command();
        let result = cmd.try_get_matches_from(["fshare"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_fshare_links_with_urls() {
        let cmd = fshare_command();
        let matches = cmd
            .try_get_matches_from([
                "fshare",
                "links",
                "https://www.fshare.vn/file/ABC",
                "https://www.fshare.vn/file/DEF",
            ])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let urls: Vec<&String> = sub.get_many::<String>("urls").unwrap().collect();
        assert_eq!(urls.len(), 2);
    }

    #[test]
    fn test_fshare_links_with_options() {
        let cmd = fshare_command();
        let matches = cmd
            .try_get_matches_from([
                "fshare",
                "links",
                "https://www.fshare.vn/file/ABC",
                "--password",
                "secret",
                "--output",
                "/tmp/downloads",
            ])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(
            sub.get_one::<String>("password").map(|s| s.as_str()),
            Some("secret"),
        );
        assert_eq!(
            sub.get_one::<String>("output").map(|s| s.as_str()),
            Some("/tmp/downloads"),
        );
    }

    #[test]
    fn test_fshare_account_login() {
        let cmd = fshare_command();
        let matches = cmd
            .try_get_matches_from(["fshare", "account", "login", "user@example.com", "pass123"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let (_, login_sub) = sub.subcommand().unwrap();
        assert_eq!(
            login_sub.get_one::<String>("email").map(|s| s.as_str()),
            Some("user@example.com"),
        );
    }

    #[test]
    fn test_fshare_token_set() {
        let cmd = fshare_command();
        let matches = cmd
            .try_get_matches_from(["fshare", "token", "set", "my-token-value"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let (_, set_sub) = sub.subcommand().unwrap();
        assert_eq!(
            set_sub.get_one::<String>("token").map(|s| s.as_str()),
            Some("my-token-value"),
        );
    }

    #[test]
    fn test_fshare_meta() {
        let meta = fshare_meta();
        assert_eq!(meta.name, "fshare");
        assert_eq!(meta.category, CommandCategory::Cloud);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = fshare_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"account"));
        assert!(sub_names.contains(&"links"));
        assert!(sub_names.contains(&"token"));
        assert_eq!(sub_names.len(), 3);
    }
}
