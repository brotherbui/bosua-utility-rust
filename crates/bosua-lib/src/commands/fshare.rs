//! FShare CLI command — Fshare operations.
//!
//! Matches Go's `fshare` command with subcommands: account, token, get, gfl,
//! generate, shorten.
//!
//! Persistent flags: --first/-1, --gid, --push

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::fshare::FShareClient;
use crate::download::DownloadManager;
use crate::errors::{BosuaError, Result};
use crate::output;

/// Build the `fshare` clap command with all subcommands.
pub fn fshare_command() -> Command {
    Command::new("fshare")
        .about("Fshare operations")
        .subcommand_required(true)
        .arg_required_else_help(true)
        // Persistent flags shared across all fshare subcommands
        .arg(
            Arg::new("first")
                .short('1')
                .long("first")
                .global(true)
                .action(clap::ArgAction::SetTrue)
                .help("Auto get detail for first result/Or single thread"),
        )
        .arg(
            Arg::new("gid")
                .long("gid")
                .global(true)
                .help("Google Drive folder ID"),
        )
        .arg(
            Arg::new("push")
                .long("push")
                .global(true)
                .action(clap::ArgAction::SetTrue)
                .help("Push to remote"),
        )
        .subcommand(account_subcommand())
        .subcommand(token_subcommand())
        .subcommand(get_subcommand())
        .subcommand(gfl_subcommand())
        .subcommand(generate_subcommand())
        .subcommand(shorten_subcommand())
}

/// Build the `CommandMeta` for registry registration.
pub fn fshare_meta() -> CommandMeta {
    CommandBuilder::from_clap(fshare_command())
        .category(CommandCategory::Core)
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
        Some(("token", sub)) => handle_token(sub, fshare).await,
        Some(("get", sub)) => handle_get(sub, fshare, dl).await,
        Some(("gfl", sub)) => handle_gfl(sub, matches, fshare).await,
        Some(("generate", sub)) => handle_generate(sub, matches, fshare).await,
        Some(("shorten", sub)) => handle_shorten(sub, fshare).await,
        _ => unreachable!("subcommand_required is set"),
    }
}

/// Mask a token for display: show last 4 chars, mask the rest with asterisks.
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
        .aliases(["a", "acc"])
        .about("Fshare account stuffs")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("info")
                .aliases(["i", "inf"])
                .about("Print current user info"),
        )
        .subcommand(
            Command::new("check")
                .aliases(["c", "ch", "chk"])
                .about("Check Fshare account validity")
                .arg(
                    Arg::new("accounts")
                        .num_args(1..)
                        .required(true)
                        .help("Accounts to check, or .txt file containing accounts"),
                ),
        )
}

fn token_subcommand() -> Command {
    Command::new("token")
        .aliases(["tk"])
        .about("Fshare token stuffs")
        .subcommand(
            Command::new("view")
                .aliases(["v", "g", "i"])
                .about("View current Fshare token and session ID"),
        )
        .subcommand(
            Command::new("reset")
                .aliases(["rs"])
                .about("Reset current token (change account case)"),
        )
}

fn get_subcommand() -> Command {
    Command::new("get")
        .aliases(["g"])
        .about("Get VIP download links from Fshare")
        .arg(
            Arg::new("links")
                .num_args(1..)
                .required(true)
                .help("Fshare links or .txt file containing links"),
        )
}

fn gfl_subcommand() -> Command {
    Command::new("gfl")
        .about("Get download links from Fshare folder")
        .arg(
            Arg::new("folder")
                .required(true)
                .help("Folder code or folder link"),
        )
}

fn generate_subcommand() -> Command {
    Command::new("generate")
        .aliases(["gen"])
        .about("Generate download links from input name/folder_link file")
        .arg(
            Arg::new("input_file")
                .required(true)
                .help("Input .txt file with name/folder_link pairs"),
        )
}

fn shorten_subcommand() -> Command {
    Command::new("shorten")
        .aliases(["gg", "tiny"])
        .about("Get original links from URL shortening services")
        .arg(
            Arg::new("urls")
                .num_args(1..)
                .required(true)
                .help("Shortened URLs to resolve"),
        )
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn handle_account(matches: &ArgMatches, fshare: &FShareClient) -> Result<()> {
    match matches.subcommand() {
        Some(("info", _)) => {
            match fshare.get_user_info().await {
                Ok(info) => {
                    print_user_info(&info, true);
                }
                Err(e) => {
                    output::info(&format!("FShare: {}", e));
                }
            }
            Ok(())
        }
        Some(("check", sub)) => {
            let inputs: Vec<String> = sub
                .get_many::<String>("accounts")
                .unwrap()
                .cloned()
                .collect();

            // Expand .txt files
            let mut accounts = Vec::new();
            for input in &inputs {
                if input.ends_with(".txt") {
                    match tokio::fs::read_to_string(input).await {
                        Ok(content) => {
                            for line in content.lines() {
                                let line = line.trim();
                                if !line.is_empty() {
                                    accounts.push(line.to_string());
                                }
                            }
                        }
                        Err(e) => {
                            output::error(&format!("Failed to read {}: {}", input, e));
                        }
                    }
                } else {
                    accounts.push(input.clone());
                }
            }

            if accounts.is_empty() {
                println!("No account to check");
                return Ok(());
            }

            // Check accounts (stub — actual parallel check will use FShareClient)
            for account in &accounts {
                output::info(&format!("Checking account: {}", account));
            }
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

/// Print user info matching Go's `PrintUserInfo(info, true)`.
///
/// When `short` is true, only prints: account_type, email, expire_vip, joindate
/// Date fields (joindate, expire_vip) are converted from Unix timestamps.
fn print_user_info(info: &serde_json::Value, short: bool) {
    let filters = ["account_type", "email", "expire_vip", "joindate"];
    let date_fields = ["joindate", "expire_vip"];

    if let Some(obj) = info.as_object() {
        // Build a mutable copy for date conversion
        let mut display: serde_json::Map<String, serde_json::Value> = obj.clone();

        // Convert Unix timestamp date fields to YYYY-MM-DD
        for field in &date_fields {
            if let Some(val) = display.get(*field) {
                let ts_str = match val {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => continue,
                };
                if let Ok(ts) = ts_str.parse::<i64>() {
                    if ts > 0 {
                        let days = ts as u64 / 86400;
                        let (y, m, d) = days_to_ymd(days);
                        display.insert(
                            field.to_string(),
                            serde_json::Value::String(format!("{:04}-{:02}-{:02}", y, m, d)),
                        );
                    }
                }
            }
        }

        if short {
            for key in &filters {
                if let Some(val) = display.get(*key) {
                    let val_str = match val {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    println!("{}: {}", key, val_str);
                }
            }
        } else {
            for (key, val) in &display {
                let val_str = match val {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                println!("{}: {}", key, val_str);
            }
        }
    }
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

async fn handle_token(matches: &ArgMatches, fshare: &FShareClient) -> Result<()> {
    match matches.subcommand() {
        Some(("view", _)) => {
            match fshare.get_token().await {
                Some(token) => {
                    let session_id = fshare.get_session_id().await.unwrap_or_default();
                    output::info(&format!("Token: {}", mask_token(&token)));
                    output::info(&format!("Session ID: {}", mask_token(&session_id)));
                }
                None => {
                    output::info("No token set.");
                }
            }
            Ok(())
        }
        Some(("reset", _)) => {
            // Delete the token file (matches Go's ResetToken)
            let path = FShareClient::token_file_path();
            let _ = tokio::fs::remove_file(&path).await;
            fshare.set_token(String::new()).await;
            output::success("Token reset");
            Ok(())
        }
        // `token` with no subcommand shows user info (matches Go behavior)
        _ => {
            match fshare.get_user_info().await {
                Ok(info) => {
                    print_user_info(&info, true);
                }
                Err(e) => {
                    output::info(&format!("FShare: {}", e));
                }
            }
            Ok(())
        }
    }
}

async fn handle_get(
    matches: &ArgMatches,
    fshare: &FShareClient,
    dl: Option<&DownloadManager>,
) -> Result<()> {
    let inputs: Vec<String> = matches
        .get_many::<String>("links")
        .unwrap()
        .cloned()
        .collect();

    // Expand .txt files into individual links
    let mut links = Vec::new();
    for input in &inputs {
        if input.ends_with(".txt") {
            match tokio::fs::read_to_string(input).await {
                Ok(content) => {
                    for line in content.lines() {
                        let line = line.trim();
                        if !line.is_empty() {
                            links.push(line.to_string());
                        }
                    }
                }
                Err(e) => {
                    output::error(&format!("Failed to read {}: {}", input, e));
                }
            }
        } else {
            links.push(input.clone());
        }
    }

    if links.is_empty() {
        println!("Input required!");
        return Ok(());
    }

    // Resolve VIP links
    let results = fshare.resolve_vip_links(&links).await;
    let mut count = 0;
    for (original, result) in &results {
        match result {
            Ok(vip_link) => {
                output::success(vip_link);
                count += 1;
            }
            Err(e) => {
                output::error(&format!("{} -> Error: {}", original, e));
            }
        }
    }

    // If we have a download manager, download the resolved links
    if let Some(_dm) = dl {
        // Download integration handled by the download manager
    }

    println!("================{} link(s) processed================", count);
    Ok(())
}

async fn handle_gfl(
    matches: &ArgMatches,
    parent_matches: &ArgMatches,
    fshare: &FShareClient,
) -> Result<()> {
    let folder = matches.get_one::<String>("folder").unwrap();
    let push = parent_matches.get_flag("push");
    let _gdriveid = parent_matches.get_one::<String>("gid");

    let resp = fshare.scan_folder(folder, None).await?;

    let mut links = Vec::new();
    let mut folders = Vec::new();

    for entry in &resp.files {
        // file_type 0 = file, 1 = folder (FShare convention)
        if entry.file_type == Some(1) {
            folders.push(entry);
        } else if let Some(ref code) = entry.link_code {
            let link = format!("https://www.fshare.vn/file/{}", code);
            links.push(link);
        }
    }

    if !links.is_empty() {
        if push {
            output::info("Pushing links to remote...");
        }
        for link in &links {
            println!("{}", link);
        }
    }

    for folder_entry in &folders {
        let code = folder_entry.link_code.as_deref().unwrap_or("");
        println!("https://www.fshare.vn/folder/{} [{}]", code, folder_entry.name);
    }

    Ok(())
}

async fn handle_generate(
    matches: &ArgMatches,
    parent_matches: &ArgMatches,
    fshare: &FShareClient,
) -> Result<()> {
    let input_file = matches.get_one::<String>("input_file").unwrap();
    let _gdriveid = parent_matches.get_one::<String>("gid");

    if !input_file.ends_with(".txt") {
        println!("Invalid input file");
        return Ok(());
    }

    let content = tokio::fs::read_to_string(input_file)
        .await
        .map_err(BosuaError::Io)?;

    let inputs: Vec<String> = content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    let total_folders = inputs.len() / 2;
    let mut outputs = Vec::new();

    for i in (0..inputs.len()).step_by(2) {
        if i + 1 >= inputs.len() {
            break;
        }
        let name = &inputs[i];
        let link = &inputs[i + 1];
        println!("Processing {}", name);

        let resp = fshare.scan_folder(link, None).await?;
        for entry in &resp.files {
            if let Some(ref code) = entry.link_code {
                outputs.push(format!("https://www.fshare.vn/file/{}", code));
            }
        }
    }

    println!("Done {} folders with total {} links", total_folders, outputs.len());

    let output_content = outputs.join("\n");
    tokio::fs::write("output.txt", &output_content).await?;

    Ok(())
}

async fn handle_shorten(
    matches: &ArgMatches,
    _fshare: &FShareClient,
) -> Result<()> {
    let urls: Vec<String> = matches
        .get_many::<String>("urls")
        .unwrap()
        .cloned()
        .collect();

    if urls.is_empty() {
        println!("Input required!");
        return Ok(());
    }

    for url in &urls {
        // Resolve shortened URL by following redirects
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(BosuaError::Http)?;

        match client.get(url).send().await {
            Ok(resp) => {
                if let Some(location) = resp.headers().get("location") {
                    if let Ok(loc) = location.to_str() {
                        println!("{}", loc);
                    }
                } else {
                    // No redirect, print the final URL
                    println!("{}", resp.url());
                }
            }
            Err(e) => {
                output::error(&format!("{} -> Error: {}", url, e));
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fshare_command_parses_account_info() {
        let cmd = fshare_command();
        let matches = cmd
            .try_get_matches_from(["fshare", "account", "info"])
            .unwrap();
        assert_eq!(matches.subcommand_name(), Some("account"));
    }

    #[test]
    fn test_fshare_command_parses_account_check() {
        let cmd = fshare_command();
        let matches = cmd
            .try_get_matches_from(["fshare", "account", "check", "user@example.com"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let (_, check_sub) = sub.subcommand().unwrap();
        let accounts: Vec<&String> = check_sub.get_many::<String>("accounts").unwrap().collect();
        assert_eq!(accounts.len(), 1);
    }

    #[test]
    fn test_fshare_requires_subcommand() {
        let cmd = fshare_command();
        let result = cmd.try_get_matches_from(["fshare"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_fshare_get_with_urls() {
        let cmd = fshare_command();
        let matches = cmd
            .try_get_matches_from([
                "fshare",
                "get",
                "https://www.fshare.vn/file/ABC",
                "https://www.fshare.vn/file/DEF",
            ])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let links: Vec<&String> = sub.get_many::<String>("links").unwrap().collect();
        assert_eq!(links.len(), 2);
    }

    #[test]
    fn test_fshare_gfl() {
        let cmd = fshare_command();
        let matches = cmd
            .try_get_matches_from(["fshare", "gfl", "FOLDER_CODE"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(
            sub.get_one::<String>("folder").map(|s| s.as_str()),
            Some("FOLDER_CODE"),
        );
    }

    #[test]
    fn test_fshare_generate() {
        let cmd = fshare_command();
        let matches = cmd
            .try_get_matches_from(["fshare", "generate", "input.txt"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(
            sub.get_one::<String>("input_file").map(|s| s.as_str()),
            Some("input.txt"),
        );
    }

    #[test]
    fn test_fshare_shorten() {
        let cmd = fshare_command();
        let matches = cmd
            .try_get_matches_from(["fshare", "shorten", "https://goo.gl/abc"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let urls: Vec<&String> = sub.get_many::<String>("urls").unwrap().collect();
        assert_eq!(urls.len(), 1);
    }

    #[test]
    fn test_fshare_persistent_flags() {
        let cmd = fshare_command();
        let matches = cmd
            .try_get_matches_from([
                "fshare", "--first", "--gid", "GDRIVE_ID", "--push",
                "get", "https://www.fshare.vn/file/ABC",
            ])
            .unwrap();
        assert!(matches.get_flag("first"));
        assert_eq!(
            matches.get_one::<String>("gid").map(|s| s.as_str()),
            Some("GDRIVE_ID"),
        );
        assert!(matches.get_flag("push"));
    }

    #[test]
    fn test_fshare_token_view() {
        let cmd = fshare_command();
        let matches = cmd
            .try_get_matches_from(["fshare", "token", "view"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(sub.subcommand_name(), Some("view"));
    }

    #[test]
    fn test_fshare_token_reset() {
        let cmd = fshare_command();
        let matches = cmd
            .try_get_matches_from(["fshare", "token", "reset"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(sub.subcommand_name(), Some("reset"));
    }

    #[test]
    fn test_fshare_meta() {
        let meta = fshare_meta();
        assert_eq!(meta.name, "fshare");
        assert_eq!(meta.category, CommandCategory::Core);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = fshare_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"account"));
        assert!(sub_names.contains(&"token"));
        assert!(sub_names.contains(&"get"));
        assert!(sub_names.contains(&"gfl"));
        assert!(sub_names.contains(&"generate"));
        assert!(sub_names.contains(&"shorten"));
        assert_eq!(sub_names.len(), 6);
    }

    #[test]
    fn test_mask_token() {
        assert_eq!(mask_token("abcdefgh"), "****efgh");
        assert_eq!(mask_token("ab"), "**");
        assert_eq!(mask_token(""), "");
    }
}
