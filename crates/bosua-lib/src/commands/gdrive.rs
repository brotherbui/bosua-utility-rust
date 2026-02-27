//! Google Drive CLI command with subcommands.
//!
//! Provides the `gdrive` command with subcommands: account, browse, file,
//! drive, permission, play, proxy, oauth2.

use std::path::Path;

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::gdrive::{GDriveClient, GDrivePermission};
use crate::errors::{BosuaError, Result};
use crate::output;

/// Build the `gdrive` clap command with all subcommands.
pub fn gdrive_command() -> Command {
    Command::new("gdrive")
        .about("Google Drive management")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(account_subcommand())
        .subcommand(browse_subcommand())
        .subcommand(file_subcommand())
        .subcommand(drive_subcommand())
        .subcommand(permission_subcommand())
        .subcommand(play_subcommand())
        .subcommand(proxy_subcommand())
        .subcommand(oauth2_subcommand())
}

/// Build the `CommandMeta` for registry registration.
pub fn gdrive_meta() -> CommandMeta {
    CommandBuilder::from_clap(gdrive_command())
        .category(CommandCategory::Cloud)
        .build()
}

/// Handle the `gdrive` command dispatch.
pub async fn handle_gdrive(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    match matches.subcommand() {
        Some(("account", sub)) => handle_account(sub, gdrive).await,
        Some(("browse", sub)) => handle_browse(sub, gdrive).await,
        Some(("file", sub)) => handle_file(sub, gdrive).await,
        Some(("drive", sub)) => handle_drive(sub),
        Some(("permission", sub)) => handle_permission(sub, gdrive).await,
        Some(("play", sub)) => handle_play(sub),
        Some(("proxy", sub)) => handle_proxy(sub),
        Some(("oauth2", sub)) => handle_oauth2(sub, gdrive).await,
        _ => unreachable!("subcommand_required is set"),
    }
}


// ---------------------------------------------------------------------------
// Subcommand definitions
// ---------------------------------------------------------------------------

fn account_subcommand() -> Command {
    Command::new("account")
        .about("Manage Google Drive accounts")
        .subcommand(
            Command::new("list").about("List configured accounts"),
        )
        .subcommand(
            Command::new("set")
                .about("Set the default account")
                .arg(
                    Arg::new("email")
                        .required(true)
                        .help("Account email to set as default"),
                ),
        )
        .subcommand(
            Command::new("info").about("Show current account info"),
        )
}

fn browse_subcommand() -> Command {
    Command::new("browse")
        .about("Interactive Google Drive file browser")
        .arg(
            Arg::new("folder-id")
                .long("folder-id")
                .short('f')
                .help("Start browsing from a specific folder ID"),
        )
        .arg(
            Arg::new("account")
                .long("account")
                .short('a')
                .help("Account to use (overrides default)"),
        )
}

fn file_subcommand() -> Command {
    Command::new("file")
        .about("File operations on Google Drive")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("list")
                .about("List files in a folder")
                .arg(
                    Arg::new("folder-id")
                        .long("folder-id")
                        .short('f')
                        .help("Folder ID to list (defaults to root)"),
                )
                .arg(
                    Arg::new("page-size")
                        .long("page-size")
                        .short('n')
                        .value_parser(clap::value_parser!(u32))
                        .default_value("100")
                        .help("Number of files per page"),
                ),
        )
        .subcommand(
            Command::new("info")
                .about("Show file metadata")
                .arg(
                    Arg::new("file-id")
                        .required(true)
                        .help("File ID to inspect"),
                ),
        )
        .subcommand(
            Command::new("upload")
                .about("Upload a file")
                .arg(
                    Arg::new("path")
                        .required(true)
                        .help("Local file path to upload"),
                )
                .arg(
                    Arg::new("parent-id")
                        .long("parent-id")
                        .short('p')
                        .help("Parent folder ID"),
                ),
        )
        .subcommand(
            Command::new("download")
                .about("Download a file")
                .arg(
                    Arg::new("file-id")
                        .required(true)
                        .help("File ID to download"),
                )
                .arg(
                    Arg::new("output")
                        .long("output")
                        .short('o')
                        .help("Output file path"),
                ),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete a file or folder")
                .arg(
                    Arg::new("file-id")
                        .required(true)
                        .help("File ID to delete"),
                ),
        )
        .subcommand(
            Command::new("move")
                .about("Move a file to a different folder")
                .arg(
                    Arg::new("file-id")
                        .required(true)
                        .help("File ID to move"),
                )
                .arg(
                    Arg::new("target-folder-id")
                        .required(true)
                        .help("Target folder ID"),
                ),
        )
        .subcommand(
            Command::new("copy")
                .about("Copy a file")
                .arg(
                    Arg::new("file-id")
                        .required(true)
                        .help("File ID to copy"),
                )
                .arg(
                    Arg::new("name")
                        .long("name")
                        .help("New name for the copy"),
                )
                .arg(
                    Arg::new("parent-id")
                        .long("parent-id")
                        .short('p')
                        .help("Target parent folder ID"),
                ),
        )
        .subcommand(
            Command::new("mkdir")
                .about("Create a folder")
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Folder name"),
                )
                .arg(
                    Arg::new("parent-id")
                        .long("parent-id")
                        .short('p')
                        .help("Parent folder ID"),
                ),
        )
}

fn drive_subcommand() -> Command {
    Command::new("drive")
        .about("Shared drive operations")
        .subcommand(
            Command::new("list").about("List shared drives"),
        )
        .subcommand(
            Command::new("info")
                .about("Show shared drive info")
                .arg(
                    Arg::new("drive-id")
                        .required(true)
                        .help("Shared drive ID"),
                ),
        )
}

fn permission_subcommand() -> Command {
    Command::new("permission")
        .about("Manage file permissions and sharing")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("share")
                .about("Share a file with a user")
                .arg(
                    Arg::new("file-id")
                        .required(true)
                        .help("File ID to share"),
                )
                .arg(
                    Arg::new("email")
                        .required(true)
                        .help("Email address to share with"),
                )
                .arg(
                    Arg::new("role")
                        .long("role")
                        .short('r')
                        .default_value("reader")
                        .value_parser(["reader", "writer", "commenter", "owner"])
                        .help("Permission role"),
                ),
        )
        .subcommand(
            Command::new("list")
                .about("List permissions on a file")
                .arg(
                    Arg::new("file-id")
                        .required(true)
                        .help("File ID to list permissions for"),
                ),
        )
        .subcommand(
            Command::new("revoke")
                .about("Revoke a permission")
                .arg(
                    Arg::new("file-id")
                        .required(true)
                        .help("File ID"),
                )
                .arg(
                    Arg::new("permission-id")
                        .required(true)
                        .help("Permission ID to revoke"),
                ),
        )
}

fn play_subcommand() -> Command {
    Command::new("play")
        .about("Stream media from Google Drive")
        .arg(
            Arg::new("file-id")
                .required(true)
                .help("File ID to stream"),
        )
        .arg(
            Arg::new("player")
                .long("player")
                .default_value("mpv")
                .help("Media player to use"),
        )
}

fn proxy_subcommand() -> Command {
    Command::new("proxy")
        .about("Proxy Google Drive file downloads")
        .arg(
            Arg::new("file-id")
                .required(true)
                .help("File ID to proxy"),
        )
        .arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .value_parser(clap::value_parser!(u16))
                .default_value("8080")
                .help("Local proxy port"),
        )
}

fn oauth2_subcommand() -> Command {
    Command::new("oauth2")
        .about("OAuth2 authentication flow")
        .subcommand(
            Command::new("login").about("Start OAuth2 login flow"),
        )
        .subcommand(
            Command::new("refresh").about("Refresh the access token"),
        )
        .subcommand(
            Command::new("status").about("Show current auth status"),
        )
}


// ---------------------------------------------------------------------------
// Handler implementations
// ---------------------------------------------------------------------------

async fn handle_oauth2(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    match matches.subcommand() {
        Some(("login", _)) => {
            let (url, _csrf) = gdrive.authorization_url()?;
            output::info("Open this URL in your browser to authorize:");
            println!("{}", url);
            println!();
            output::info("Enter the authorization code:");

            let mut code = String::new();
            std::io::stdin()
                .read_line(&mut code)
                .map_err(|e| BosuaError::Command(format!("Failed to read auth code: {}", e)))?;
            let code = code.trim();

            if code.is_empty() {
                return Err(BosuaError::Command("Authorization code cannot be empty".into()));
            }

            let token = gdrive.exchange_code(code).await?;
            output::success(&format!(
                "Successfully authenticated. Token type: {}",
                token.token_type
            ));
            Ok(())
        }
        Some(("refresh", _)) => {
            let token = gdrive.refresh_token().await?;
            output::success(&format!(
                "Token refreshed. Expires: {}",
                token.expiry.as_deref().unwrap_or("unknown")
            ));
            Ok(())
        }
        Some(("status", _)) => {
            match gdrive.load_token().await? {
                Some(token) => {
                    output::success("Authenticated");
                    println!("  Token type: {}", token.token_type);
                    println!(
                        "  Expires: {}",
                        token.expiry.as_deref().unwrap_or("unknown")
                    );
                    println!(
                        "  Refresh token: {}",
                        if token.refresh_token.is_some() { "present" } else { "none" }
                    );
                }
                None => {
                    output::warning("Not authenticated");
                    output::info("Run `bosua gdrive oauth2 login` to authenticate.");
                }
            }
            Ok(())
        }
        _ => {
            output::info("oauth2: use a subcommand (login, refresh, status)");
            Ok(())
        }
    }
}

async fn handle_file(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", sub)) => {
            let folder_id = sub.get_one::<String>("folder-id");
            let page_size = sub.get_one::<u32>("page-size").copied();
            let file_list = gdrive
                .list_files(folder_id.map(|s| s.as_str()), None, page_size)
                .await?;

            if file_list.files.is_empty() {
                output::info("No files found.");
            } else {
                println!("{:<44} {:<40} {:>10} {}", "ID", "Name", "Size", "Type");
                println!("{}", "-".repeat(100));
                for f in &file_list.files {
                    let size = f
                        .size
                        .map(|s| format_size(s))
                        .unwrap_or_else(|| "-".to_string());
                    println!("{:<44} {:<40} {:>10} {}", f.id, f.name, size, f.mime_type);
                }
                println!("\n{} file(s)", file_list.files.len());
                if file_list.next_page_token.is_some() {
                    output::info("More files available. Use pagination to see all.");
                }
            }
            Ok(())
        }
        Some(("info", sub)) => {
            let file_id = sub.get_one::<String>("file-id").unwrap();
            let file = gdrive.get_file_metadata(file_id).await?;
            display_file_metadata(&file);
            Ok(())
        }
        Some(("upload", sub)) => {
            let path_str = sub.get_one::<String>("path").unwrap();
            let parent_id = sub.get_one::<String>("parent-id");
            let path = Path::new(path_str);

            if !path.exists() {
                return Err(BosuaError::Command(format!(
                    "File not found: {}",
                    path.display()
                )));
            }

            let file = gdrive
                .upload_file(path, parent_id.map(|s| s.as_str()))
                .await?;
            output::success(&format!("Uploaded: {} ({})", file.name, file.id));
            Ok(())
        }
        Some(("download", sub)) => {
            let file_id = sub.get_one::<String>("file-id").unwrap();
            let output_path = sub.get_one::<String>("output");

            // Get metadata to determine filename
            let meta = gdrive.get_file_metadata(file_id).await?;
            let dest = match output_path {
                Some(p) => std::path::PathBuf::from(p),
                None => std::path::PathBuf::from(&meta.name),
            };

            let bytes = gdrive.download_file(file_id).await?;
            tokio::fs::write(&dest, &bytes).await?;
            output::success(&format!(
                "Downloaded: {} ({} bytes) -> {}",
                meta.name,
                bytes.len(),
                dest.display()
            ));
            Ok(())
        }
        Some(("delete", sub)) => {
            let file_id = sub.get_one::<String>("file-id").unwrap();
            gdrive.delete_file(file_id).await?;
            output::success(&format!("Deleted: {}", file_id));
            Ok(())
        }
        Some(("move", sub)) => {
            let file_id = sub.get_one::<String>("file-id").unwrap();
            let target = sub.get_one::<String>("target-folder-id").unwrap();
            let file = gdrive.move_file(file_id, target).await?;
            output::success(&format!("Moved: {} -> folder {}", file.name, target));
            Ok(())
        }
        Some(("copy", sub)) => {
            let file_id = sub.get_one::<String>("file-id").unwrap();
            let new_name = sub.get_one::<String>("name");
            let parent_id = sub.get_one::<String>("parent-id");
            let file = gdrive
                .copy_file(
                    file_id,
                    new_name.map(|s| s.as_str()),
                    parent_id.map(|s| s.as_str()),
                )
                .await?;
            output::success(&format!("Copied: {} ({})", file.name, file.id));
            Ok(())
        }
        Some(("mkdir", sub)) => {
            let name = sub.get_one::<String>("name").unwrap();
            let parent_id = sub.get_one::<String>("parent-id");
            let folder = gdrive
                .create_folder(name, parent_id.map(|s| s.as_str()))
                .await?;
            output::success(&format!("Created folder: {} ({})", folder.name, folder.id));
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

async fn handle_account(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let account = gdrive.default_account().await;
            println!("Default account: {}", account);
            Ok(())
        }
        Some(("set", sub)) => {
            let email = sub.get_one::<String>("email").unwrap();
            gdrive.update_default_account(email).await;
            output::success(&format!("Default account set to: {}", email));
            Ok(())
        }
        Some(("info", _)) => {
            let account = gdrive.default_account().await;
            println!("Current account: {}", account);
            match gdrive.load_token().await? {
                Some(token) => {
                    println!("  Status: authenticated");
                    println!(
                        "  Expires: {}",
                        token.expiry.as_deref().unwrap_or("unknown")
                    );
                }
                None => {
                    println!("  Status: not authenticated");
                    output::info("Run `bosua gdrive oauth2 login` to authenticate.");
                }
            }
            Ok(())
        }
        _ => {
            output::info("account: use a subcommand (list, set, info)");
            Ok(())
        }
    }
}

async fn handle_browse(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    let folder_id = matches.get_one::<String>("folder-id");
    gdrive
        .browse_interactive(folder_id.map(|s| s.as_str()))
        .await?;
    Ok(())
}

async fn handle_permission(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    match matches.subcommand() {
        Some(("share", sub)) => {
            let file_id = sub.get_one::<String>("file-id").unwrap();
            let email = sub.get_one::<String>("email").unwrap();
            let role = sub.get_one::<String>("role").unwrap();

            let permission = GDrivePermission {
                role: role.clone(),
                permission_type: "user".to_string(),
                email_address: Some(email.clone()),
            };

            gdrive.share_file(file_id, &permission).await?;
            output::success(&format!(
                "Shared {} with {} as {}",
                file_id, email, role
            ));
            Ok(())
        }
        Some(("list", sub)) => {
            let file_id = sub.get_one::<String>("file-id").unwrap();
            // list_files permissions not directly available; show file info instead
            let file = gdrive.get_file_metadata(file_id).await?;
            output::info(&format!("Permissions for: {} ({})", file.name, file.id));
            output::info("Use the Google Drive web interface for detailed permission listing.");
            Ok(())
        }
        Some(("revoke", sub)) => {
            let file_id = sub.get_one::<String>("file-id").unwrap();
            let _perm_id = sub.get_one::<String>("permission-id").unwrap();
            // No direct revoke method on GDriveClient; report limitation
            output::info(&format!(
                "Permission revocation for file {} is not yet supported via the API.",
                file_id
            ));
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

fn handle_drive(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            output::info("Shared drive listing is not yet implemented.");
            Ok(())
        }
        Some(("info", sub)) => {
            let id = sub.get_one::<String>("drive-id").unwrap();
            output::info(&format!("Shared drive info for {}: not yet implemented.", id));
            Ok(())
        }
        _ => {
            output::info("drive: use a subcommand (list, info)");
            Ok(())
        }
    }
}

fn handle_play(matches: &ArgMatches) -> Result<()> {
    let file_id = matches.get_one::<String>("file-id").unwrap();
    let player = matches.get_one::<String>("player").unwrap();
    output::info(&format!(
        "gdrive play {} with {}: use `bosua play --type gdrive` instead.",
        file_id, player
    ));
    Ok(())
}

fn handle_proxy(matches: &ArgMatches) -> Result<()> {
    let file_id = matches.get_one::<String>("file-id").unwrap();
    let port = matches.get_one::<u16>("port").unwrap();
    output::info(&format!(
        "gdrive proxy {} on port {}: use `bosua proxy start` instead.",
        file_id, port
    ));
    Ok(())
}


// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn display_file_metadata(file: &crate::cloud::gdrive::GDriveFile) {
    println!("ID:            {}", file.id);
    println!("Name:          {}", file.name);
    println!("MIME Type:     {}", file.mime_type);
    if let Some(size) = file.size {
        println!("Size:          {}", format_size(size));
    }
    if !file.parents.is_empty() {
        println!("Parents:       {}", file.parents.join(", "));
    }
    if let Some(ref t) = file.created_time {
        println!("Created:       {}", t);
    }
    if let Some(ref t) = file.modified_time {
        println!("Modified:      {}", t);
    }
    if let Some(ref link) = file.web_view_link {
        println!("View Link:     {}", link);
    }
    if let Some(ref link) = file.web_content_link {
        println!("Download Link: {}", link);
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gdrive_command_parses() {
        let cmd = gdrive_command();
        let matches = cmd
            .try_get_matches_from(["gdrive", "browse"])
            .unwrap();
        assert_eq!(matches.subcommand_name(), Some("browse"));
    }

    #[test]
    fn test_gdrive_requires_subcommand() {
        let cmd = gdrive_command();
        let result = cmd.try_get_matches_from(["gdrive"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_gdrive_file_list_subcommand() {
        let cmd = gdrive_command();
        let matches = cmd
            .try_get_matches_from(["gdrive", "file", "list", "--folder-id", "abc123"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "file");
        let (file_sub_name, file_sub) = sub.subcommand().unwrap();
        assert_eq!(file_sub_name, "list");
        assert_eq!(
            file_sub.get_one::<String>("folder-id").map(|s| s.as_str()),
            Some("abc123")
        );
    }

    #[test]
    fn test_gdrive_permission_share() {
        let cmd = gdrive_command();
        let matches = cmd
            .try_get_matches_from([
                "gdrive", "permission", "share", "file123", "user@example.com", "--role", "writer",
            ])
            .unwrap();
        let (_, perm_sub) = matches.subcommand().unwrap();
        let (_, share_sub) = perm_sub.subcommand().unwrap();
        assert_eq!(
            share_sub.get_one::<String>("file-id").map(|s| s.as_str()),
            Some("file123")
        );
        assert_eq!(
            share_sub.get_one::<String>("role").map(|s| s.as_str()),
            Some("writer")
        );
    }

    #[test]
    fn test_gdrive_oauth2_login() {
        let cmd = gdrive_command();
        let matches = cmd
            .try_get_matches_from(["gdrive", "oauth2", "login"])
            .unwrap();
        let (name, _) = matches.subcommand().unwrap();
        assert_eq!(name, "oauth2");
    }

    #[test]
    fn test_gdrive_play_subcommand() {
        let cmd = gdrive_command();
        let matches = cmd
            .try_get_matches_from(["gdrive", "play", "file123", "--player", "vlc"])
            .unwrap();
        let (_, play_sub) = matches.subcommand().unwrap();
        assert_eq!(
            play_sub.get_one::<String>("file-id").map(|s| s.as_str()),
            Some("file123")
        );
        assert_eq!(
            play_sub.get_one::<String>("player").map(|s| s.as_str()),
            Some("vlc")
        );
    }

    #[test]
    fn test_gdrive_proxy_subcommand() {
        let cmd = gdrive_command();
        let matches = cmd
            .try_get_matches_from(["gdrive", "proxy", "file123", "--port", "9090"])
            .unwrap();
        let (_, proxy_sub) = matches.subcommand().unwrap();
        assert_eq!(
            proxy_sub.get_one::<String>("file-id").map(|s| s.as_str()),
            Some("file123")
        );
        assert_eq!(proxy_sub.get_one::<u16>("port"), Some(&9090));
    }

    #[test]
    fn test_gdrive_meta() {
        let meta = gdrive_meta();
        assert_eq!(meta.name, "gdrive");
        assert_eq!(meta.category, CommandCategory::Cloud);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = gdrive_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"account"));
        assert!(sub_names.contains(&"browse"));
        assert!(sub_names.contains(&"file"));
        assert!(sub_names.contains(&"drive"));
        assert!(sub_names.contains(&"permission"));
        assert!(sub_names.contains(&"play"));
        assert!(sub_names.contains(&"proxy"));
        assert!(sub_names.contains(&"oauth2"));
        assert_eq!(sub_names.len(), 8);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(1073741824), "1.0 GB");
    }
}
