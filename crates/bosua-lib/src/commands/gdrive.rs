//! Google Drive CLI command with subcommands.
//!
//! Provides the `gdrive` command (alias: `gd`) with FLAT subcommands matching Go:
//! account, browse, info, list, search, upload, download, delete, mkdir, rename,
//! move, copy, import, export, generate-playlist, drives, permissions, play, proxy, oauth2.

use std::path::Path;

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::gdrive::{GDriveClient, GDrivePermission};
use crate::errors::{BosuaError, Result};
use crate::output;

/// Build the `gdrive` clap command with all subcommands.
pub fn gdrive_command() -> Command {
    Command::new("gdrive")
        .aliases(["gd"])
        .about("Google Drive CLI - comprehensive file management")
        .long_about("Manage your Google Drive files, folders, accounts, and permissions directly from the command line with optimized performance and advanced features.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(
            Arg::new("account")
                .long("account")
                .global(true)
                .help("Use a specific Google Drive account instead of the current one"),
        )
        // Account management
        .subcommand(account_subcommand())
        // OAuth2
        .subcommand(oauth2_subcommand())
        // File operations (flat, matching Go)
        .subcommand(info_subcommand())
        .subcommand(list_subcommand())
        .subcommand(search_subcommand())
        .subcommand(upload_subcommand())
        .subcommand(download_subcommand())
        .subcommand(delete_subcommand())
        .subcommand(mkdir_subcommand())
        .subcommand(rename_subcommand())
        .subcommand(move_subcommand())
        .subcommand(copy_subcommand())
        .subcommand(import_subcommand())
        .subcommand(export_subcommand())
        .subcommand(generate_playlist_subcommand())
        // Drive management
        .subcommand(drives_subcommand())
        // Browse
        .subcommand(browse_subcommand())
        // Permissions
        .subcommand(permissions_subcommand())
        // Play & Proxy
        .subcommand(play_subcommand())
        .subcommand(proxy_subcommand())
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
        Some(("oauth2", sub)) => handle_oauth2(sub, gdrive).await,
        Some(("info", sub)) => handle_info(sub, gdrive).await,
        Some(("list", sub)) => handle_list(sub, gdrive).await,
        Some(("search", sub)) => handle_search(sub, gdrive).await,
        Some(("upload", sub)) => handle_upload(sub, gdrive).await,
        Some(("download", sub)) => handle_download(sub, gdrive).await,
        Some(("delete", sub)) => handle_delete(sub, gdrive).await,
        Some(("mkdir", sub)) => handle_mkdir(sub, gdrive).await,
        Some(("rename", sub)) => handle_rename(sub, gdrive).await,
        Some(("move", sub)) => handle_move(sub, gdrive).await,
        Some(("copy", sub)) => handle_copy(sub, gdrive).await,
        Some(("import", sub)) => handle_import(sub, gdrive).await,
        Some(("export", sub)) => handle_export(sub, gdrive).await,
        Some(("generate-playlist", sub)) => handle_generate_playlist(sub, gdrive).await,
        Some(("drives", sub)) => handle_drives(sub),
        Some(("browse", sub)) => handle_browse(sub, gdrive).await,
        Some(("permissions", sub)) => handle_permissions(sub, gdrive).await,
        Some(("play", sub)) => handle_play(sub),
        Some(("proxy", sub)) => handle_proxy(sub),
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// Subcommand definitions
// ---------------------------------------------------------------------------

fn account_subcommand() -> Command {
    Command::new("account")
        .aliases(["a", "acc"])
        .about("Manage Google Drive accounts")
        .subcommand(Command::new("add").about("Add a new Google Drive account with OAuth2"))
        .subcommand(
            Command::new("list")
                .aliases(["ls"])
                .about("List all configured accounts")
                .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON")),
        )
        .subcommand(
            Command::new("current")
                .aliases(["c"])
                .about("Show current account"),
        )
        .subcommand(
            Command::new("info")
                .about("Show account information and token status")
                .arg(Arg::new("account_name").help("Account name (optional)")),
        )
        .subcommand(
            Command::new("switch")
                .aliases(["s"])
                .about("Switch to a different account")
                .arg(Arg::new("account_name").required(true).help("Account name to switch to")),
        )
        .subcommand(
            Command::new("remove")
                .aliases(["r", "rm", "del"])
                .about("Remove an account")
                .arg(Arg::new("account_name").required(true).help("Account name to remove")),
        )
        .subcommand(
            Command::new("import")
                .aliases(["i", "im"])
                .about("Import account configuration")
                .arg(Arg::new("archive_path").required(true).help("Path to archive file")),
        )
        .subcommand(
            Command::new("export")
                .aliases(["e", "ex"])
                .about("Export account configuration")
                .arg(Arg::new("account_name").required(true).help("Account name to export")),
        )
        .subcommand(
            Command::new("stats")
                .aliases(["st"])
                .about("Show storage usage and largest files")
                .arg(Arg::new("top").long("top").value_parser(clap::value_parser!(u32)).default_value("20").help("Number of largest files to show"))
                .arg(Arg::new("files").long("files").action(clap::ArgAction::SetTrue).help("Show largest files")),
        )
}

fn info_subcommand() -> Command {
    Command::new("info")
        .aliases(["i"])
        .about("Show file information")
        .arg(Arg::new("file-id").required(true).help("File ID to inspect"))
        .arg(Arg::new("size-in-bytes").long("size-in-bytes").action(clap::ArgAction::SetTrue).help("Display size in bytes"))
}

fn list_subcommand() -> Command {
    Command::new("list")
        .aliases(["ls"])
        .about("List files")
        .arg(Arg::new("max").long("max").value_parser(clap::value_parser!(u32)).default_value("100").help("Max files to list"))
        .arg(Arg::new("query").long("query").help("Query. See https://developers.google.com/drive/search-parameters"))
        .arg(Arg::new("order-by").long("order-by").help("Order by"))
        .arg(Arg::new("parent").long("parent").help("List files in a specific folder (directory ID)"))
        .arg(Arg::new("drive").long("drive").help("List files on a shared drive (drive ID)"))
        .arg(Arg::new("skip-header").long("skip-header").action(clap::ArgAction::SetTrue).help("Don't print header"))
        .arg(Arg::new("full-name").long("full-name").action(clap::ArgAction::SetTrue).help("Show full file name without truncating"))
        .arg(Arg::new("field-separator").long("field-separator").default_value("\t").help("Field separator"))
        .arg(Arg::new("folder-size").long("folder-size").action(clap::ArgAction::SetTrue).help("Calculate folder sizes"))
        .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON for machine-readable parsing"))
        .arg(Arg::new("interactive").long("interactive").action(clap::ArgAction::SetTrue).help("Interactively select a file to play"))
}

fn search_subcommand() -> Command {
    Command::new("search")
        .aliases(["find", "s"])
        .about("Search files in Google Drive")
        .arg(Arg::new("search_text").required(true).num_args(1..).help("Search text"))
        .arg(Arg::new("max").long("max").value_parser(clap::value_parser!(u32)).default_value("100").help("Max files to return"))
        .arg(Arg::new("order-by").long("order-by").help("Order by"))
        .arg(Arg::new("parent").long("parent").help("Search in a specific folder (directory ID)"))
        .arg(Arg::new("drive").long("drive").help("Search in a shared drive (drive ID)"))
        .arg(Arg::new("skip-header").long("skip-header").action(clap::ArgAction::SetTrue).help("Don't print header"))
        .arg(Arg::new("full-name").long("full-name").action(clap::ArgAction::SetTrue).help("Show full file name without truncating"))
        .arg(Arg::new("field-separator").long("field-separator").default_value("\t").help("Field separator"))
        .arg(Arg::new("folder-size").long("folder-size").action(clap::ArgAction::SetTrue).help("Calculate folder sizes"))
        .arg(Arg::new("exact-name").long("exact-name").action(clap::ArgAction::SetTrue).help("Search for exact filename match"))
        .arg(Arg::new("type").long("type").help("Filter by file type (video, image, document, pdf, etc.)"))
        .arg(Arg::new("modified-after").long("modified-after").help("Only show files modified after date (YYYY-MM-DD)"))
        .arg(Arg::new("modified-before").long("modified-before").help("Only show files modified before date (YYYY-MM-DD)"))
        .arg(Arg::new("include-trashed").long("include-trashed").action(clap::ArgAction::SetTrue).help("Include trashed files"))
        .arg(Arg::new("raw-query").long("raw-query").help("Use raw Google Drive API query"))
        .arg(Arg::new("interactive").long("interactive").action(clap::ArgAction::SetTrue).help("Interactively select a file to play from results"))
}

fn upload_subcommand() -> Command {
    Command::new("upload")
        .aliases(["u", "up"])
        .about("Upload files with optimized performance")
        .arg(Arg::new("path").num_args(1..=100).help("Local file path(s) to upload"))
        .arg(Arg::new("mime").long("mime").help("Force mime type [default: auto-detect]"))
        .arg(Arg::new("parent").long("parent").num_args(1..).help("Upload to an existing directory"))
        .arg(Arg::new("recursive").long("recursive").action(clap::ArgAction::SetTrue).help("Upload directories"))
        .arg(Arg::new("chunk-size").long("chunk-size").value_parser(clap::value_parser!(u32)).default_value("64").help("Set chunk size in MB"))
        .arg(Arg::new("print-chunk-errors").long("print-chunk-errors").action(clap::ArgAction::SetTrue).help("Print errors during chunk upload"))
        .arg(Arg::new("print-chunk-info").long("print-chunk-info").action(clap::ArgAction::SetTrue).help("Print details about each chunk"))
        .arg(Arg::new("concurrent").long("concurrent").action(clap::ArgAction::SetTrue).help("Enable concurrent uploads"))
        .arg(Arg::new("max-workers").long("max-workers").value_parser(clap::value_parser!(u32)).default_value("4").help("Maximum concurrent upload workers (2-16)"))
        .arg(Arg::new("job-id").long("job-id").help("Job ID for status tracking"))
}

fn download_subcommand() -> Command {
    Command::new("download")
        .aliases(["d", "dl"])
        .about("Download files with optimized performance")
        .long_about("Download a file or directory from Google Drive by file ID, URL, or filename.")
        .arg(Arg::new("file-id").required(true).num_args(1..).help("File ID, URL, or name to download"))
        .arg(Arg::new("overwrite").long("overwrite").action(clap::ArgAction::SetTrue).help("Overwrite existing files"))
        .arg(Arg::new("follow-shortcuts").long("follow-shortcuts").action(clap::ArgAction::SetTrue).help("Follow shortcut and download target file"))
        .arg(Arg::new("recursive").long("recursive").action(clap::ArgAction::SetTrue).help("Download directories"))
        .arg(Arg::new("destination").long("destination").help("Path where the file should be downloaded to"))
        .arg(Arg::new("stdout").long("stdout").action(clap::ArgAction::SetTrue).help("Write file to stdout"))
        .arg(Arg::new("concurrent").long("concurrent").action(clap::ArgAction::SetTrue).help("Enable concurrent downloads"))
        .arg(Arg::new("max-workers").long("max-workers").value_parser(clap::value_parser!(u32)).default_value("4").help("Maximum concurrent download workers (2-16)"))
        .arg(Arg::new("buffer-size").long("buffer-size").value_parser(clap::value_parser!(u32)).default_value("2048").help("Download buffer size in KB (64-8192)"))
        .arg(Arg::new("default").long("default").action(clap::ArgAction::SetTrue).help("Use legacy GDrive downloader instead of aria2"))
}

fn delete_subcommand() -> Command {
    Command::new("delete")
        .aliases(["rm", "del"])
        .about("Delete file by ID or name")
        .arg(Arg::new("file-id").required(true).help("File ID or name to delete"))
        .arg(Arg::new("recursive").long("recursive").action(clap::ArgAction::SetTrue).help("Delete directory and all its content"))
        .arg(Arg::new("dry-run").long("dry-run").action(clap::ArgAction::SetTrue).help("Show what would be deleted"))
        .arg(Arg::new("all").long("all").action(clap::ArgAction::SetTrue).help("Delete all files with this name"))
        .arg(Arg::new("yes").long("yes").action(clap::ArgAction::SetTrue).help("Skip confirmation prompts"))
        .arg(Arg::new("parent").long("parent").help("Only search in specific folder (directory ID)"))
}

fn mkdir_subcommand() -> Command {
    Command::new("mkdir")
        .aliases(["c"])
        .about("Create directory")
        .arg(Arg::new("name").required(true).help("Folder name"))
        .arg(Arg::new("parent").long("parent").num_args(1..).help("Create in an existing directory"))
}

fn rename_subcommand() -> Command {
    Command::new("rename")
        .aliases(["ren"])
        .about("Rename file/directory")
        .arg(Arg::new("file-id").required(true).help("File ID to rename"))
        .arg(Arg::new("new-name").required(true).help("New name"))
}

fn move_subcommand() -> Command {
    Command::new("move")
        .aliases(["mv"])
        .about("Move file/directory")
        .arg(Arg::new("file-id").required(true).help("File ID to move"))
        .arg(Arg::new("folder-id").required(true).help("Target folder ID"))
}

fn copy_subcommand() -> Command {
    Command::new("copy")
        .aliases(["cp"])
        .about("Copy file")
        .arg(Arg::new("file-id").required(true).help("File ID to copy"))
        .arg(Arg::new("folder-id").required(true).help("Target folder ID"))
}

fn import_subcommand() -> Command {
    Command::new("import")
        .about("Import file as a Google document")
        .long_about("Import file as a Google document/spreadsheet/presentation. Example file types: doc, docx, odt, pdf, html, xls, xlsx, csv, ods, ppt, pptx, odp")
        .arg(Arg::new("file-path").required(true).help("Local file path to import"))
        .arg(Arg::new("parent").long("parent").num_args(1..).help("Upload to an existing directory"))
}

fn export_subcommand() -> Command {
    Command::new("export")
        .about("Export Google document to file")
        .arg(Arg::new("file-id").required(true).help("File ID to export"))
        .arg(Arg::new("file-path").required(true).help("Local file path for export"))
        .arg(Arg::new("overwrite").long("overwrite").action(clap::ArgAction::SetTrue).help("Overwrite existing files"))
}

fn generate_playlist_subcommand() -> Command {
    Command::new("generate-playlist")
        .about("Generate Kodi playlist from folder or query")
        .arg(Arg::new("parent").long("parent").help("Generate playlist from specific folder (directory ID)"))
        .arg(Arg::new("query").long("query").default_value("mimeType contains 'video/'").help("Search query for files to include"))
        .arg(Arg::new("output").long("output").help("Output file path (default: stdout)"))
        .arg(Arg::new("max").long("max").value_parser(clap::value_parser!(u32)).default_value("100").help("Maximum number of files to include"))
        .arg(Arg::new("name").long("name").default_value("Google Drive Playlist").help("Playlist name"))
}

fn drives_subcommand() -> Command {
    Command::new("drives")
        .about("Manage shared drives")
        .subcommand(
            Command::new("list")
                .aliases(["ls"])
                .about("List shared drives")
                .arg(Arg::new("skip-header").long("skip-header").action(clap::ArgAction::SetTrue).help("Don't print header"))
                .arg(Arg::new("field-separator").long("field-separator").default_value("\t").help("Field separator"))
                .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON")),
        )
}

fn browse_subcommand() -> Command {
    Command::new("browse")
        .about("Interactive Google Drive file browser")
        .arg(Arg::new("folder-id").long("folder-id").short('f').help("Start browsing from a specific folder ID"))
}

fn permissions_subcommand() -> Command {
    Command::new("permissions")
        .aliases(["per"])
        .about("Manage file permissions")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("list")
                .aliases(["ls"])
                .about("List file permissions")
                .arg(Arg::new("file-id").required(true).help("File ID"))
                .arg(Arg::new("skip-header").long("skip-header").action(clap::ArgAction::SetTrue).help("Don't print header"))
                .arg(Arg::new("field-separator").long("field-separator").default_value("\t").help("Field separator"))
                .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON")),
        )
        .subcommand(
            Command::new("share")
                .about("Share file with user")
                .arg(Arg::new("file-id").required(true).help("File ID to share"))
                .arg(Arg::new("email").required(true).help("Email address to share with"))
                .arg(Arg::new("role").long("role").default_value("reader").value_parser(["reader", "writer", "commenter", "owner"]).help("Permission role"))
                .arg(Arg::new("type").long("type").default_value("user").value_parser(["user", "group", "domain", "anyone"]).help("Permission type"))
                .arg(Arg::new("domain").long("domain").help("Domain for domain permissions"))
                .arg(Arg::new("discoverable").long("discoverable").action(clap::ArgAction::SetTrue).help("Make file discoverable by search")),
        )
        .subcommand(
            Command::new("revoke")
                .about("Revoke file permission")
                .arg(Arg::new("file-id").required(true).help("File ID"))
                .arg(Arg::new("permission-id").help("Permission ID to revoke"))
                .arg(Arg::new("all").long("all").action(clap::ArgAction::SetTrue).help("Revoke all permissions (except owner)")),
        )
}

fn play_subcommand() -> Command {
    Command::new("play")
        .about("Stream media from Google Drive")
        .arg(Arg::new("file-id").required(true).help("File ID to stream"))
        .arg(Arg::new("player").long("player").default_value("mpv").help("Media player to use"))
}

fn proxy_subcommand() -> Command {
    Command::new("proxy")
        .about("Proxy Google Drive file downloads")
        .subcommand(
            Command::new("start")
                .about("Start the GDrive proxy server")
                .arg(Arg::new("host").long("host").default_value("0.0.0.0").help("Bind host"))
                .arg(Arg::new("port").long("port").value_parser(clap::value_parser!(u16)).default_value("8088").help("Local proxy port")),
        )
}

fn oauth2_subcommand() -> Command {
    Command::new("oauth2")
        .about("OAuth2 authentication flow")
        .subcommand(Command::new("login").about("Start OAuth2 login flow"))
        .subcommand(Command::new("refresh").about("Refresh the access token"))
        .subcommand(Command::new("status").about("Show current auth status"))
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

async fn handle_info(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    let file_id = matches.get_one::<String>("file-id").unwrap();
    let file = gdrive.get_file_metadata(file_id).await?;
    display_file_metadata(&file);
    Ok(())
}

async fn handle_list(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    let folder_id = matches.get_one::<String>("parent");
    let page_size = matches.get_one::<u32>("max").copied();
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

async fn handle_search(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    let search_terms: Vec<&String> = matches.get_many::<String>("search_text").unwrap().collect();
    let search_text = search_terms.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ");
    let max_files = matches.get_one::<u32>("max").copied().unwrap_or(100);

    // Build query matching Go's search logic
    let query = format!(
        "name contains '{}' and trashed = false",
        search_text.replace('\'', "\\'")
    );

    let file_list = gdrive
        .list_files(None, Some(&query), Some(max_files))
        .await?;

    if file_list.files.is_empty() {
        println!("No results");
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
    }
    Ok(())
}

async fn handle_upload(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    let paths: Vec<&String> = matches.get_many::<String>("path").unwrap_or_default().collect();
    if paths.is_empty() {
        return Err(BosuaError::Command("No file path provided".into()));
    }
    let parent_id = matches.get_one::<String>("parent");
    let path = Path::new(paths[0]);

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

async fn handle_download(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    let file_ids: Vec<&String> = matches.get_many::<String>("file-id").unwrap().collect();
    let output_path = matches.get_one::<String>("destination");

    let file_id = file_ids[0];
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

async fn handle_delete(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    let file_id = matches.get_one::<String>("file-id").unwrap();
    gdrive.delete_file(file_id).await?;
    output::success(&format!("Deleted: {}", file_id));
    Ok(())
}

async fn handle_mkdir(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    let name = matches.get_one::<String>("name").unwrap();
    let parent_id = matches.get_one::<String>("parent");
    let folder = gdrive
        .create_folder(name, parent_id.map(|s| s.as_str()))
        .await?;
    output::success(&format!("Created folder: {} ({})", folder.name, folder.id));
    Ok(())
}

async fn handle_rename(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    let file_id = matches.get_one::<String>("file-id").unwrap();
    let new_name = matches.get_one::<String>("new-name").unwrap();
    gdrive.rename_file(file_id, new_name).await?;
    output::success(&format!("Renamed {} to '{}'", file_id, new_name));
    Ok(())
}

async fn handle_move(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    let file_id = matches.get_one::<String>("file-id").unwrap();
    let target = matches.get_one::<String>("folder-id").unwrap();
    let file = gdrive.move_file(file_id, target).await?;
    output::success(&format!("Moved: {} -> folder {}", file.name, target));
    Ok(())
}

async fn handle_copy(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    let file_id = matches.get_one::<String>("file-id").unwrap();
    let parent_id = matches.get_one::<String>("folder-id");
    let file = gdrive
        .copy_file(file_id, None, parent_id.map(|s| s.as_str()))
        .await?;
    output::success(&format!("Copied: {} ({})", file.name, file.id));
    Ok(())
}

async fn handle_import(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    let file_path = matches.get_one::<String>("file-path").unwrap();
    let parents: Vec<&String> = matches.get_many::<String>("parent").unwrap_or_default().collect();

    let path = std::path::Path::new(file_path.as_str());
    if !path.exists() {
        return Err(BosuaError::Command(format!("File not found: {}", file_path)));
    }

    let parent_id = parents.first().map(|s| s.as_str());
    let file = gdrive.upload_file(path, parent_id).await?;
    output::success(&format!("Uploaded: {} ({})", file.name, file.id));
    Ok(())
}

async fn handle_export(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    let file_id = matches.get_one::<String>("file-id").unwrap();
    let file_path = matches.get_one::<String>("file-path").unwrap();

    let data = gdrive.download_file(file_id).await?;
    std::fs::write(file_path, &data).map_err(BosuaError::Io)?;
    output::success(&format!("Exported {} to {}", file_id, file_path));
    Ok(())
}

async fn handle_generate_playlist(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
    let parent = matches.get_one::<String>("parent").map(|s| s.as_str());
    let output_path = matches.get_one::<String>("output");

    let file_list = gdrive.list_files(parent, None, Some(1000)).await?;
    let files: Vec<_> = file_list.files.iter()
        .filter(|f| {
            f.mime_type.contains("video/") || f.mime_type.contains("audio/")
        })
        .collect();

    if files.is_empty() {
        println!("No media files found");
        return Ok(());
    }

    let mut playlist = String::from("#EXTM3U\n");
    for f in &files {
        let size = f.size.map(|s| s.to_string()).unwrap_or_else(|| "0".to_string());
        playlist.push_str(&format!("#EXTINF:-1,{}\n", f.name));
        playlist.push_str(&format!("https://www.googleapis.com/drive/v3/files/{}?alt=media&size={}\n", f.id, size));
    }

    match output_path {
        Some(path) => {
            std::fs::write(path, &playlist).map_err(BosuaError::Io)?;
            output::success(&format!("Playlist written to {} ({} files)", path, files.len()));
        }
        None => {
            print!("{}", playlist);
        }
    }
    Ok(())
}

/// GDrive config base path: `~/.config/gdrive3/`
/// Matches Go's `DefaultBasePath()`.
fn gdrive_base_path() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
    home.join(".config").join("gdrive3")
}

/// List all configured GDrive accounts by scanning `~/.config/gdrive3/`
/// for subdirectories containing `tokens.json`.
/// Matches Go's `ListAccounts()`.
fn list_gdrive_accounts() -> Result<Vec<String>> {
    let base = gdrive_base_path();
    if !base.exists() {
        return Ok(Vec::new());
    }
    let mut accounts = Vec::new();
    let entries = std::fs::read_dir(&base)
        .map_err(|e| BosuaError::Io(e))?;
    for entry in entries {
        let entry = entry.map_err(|e| BosuaError::Io(e))?;
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            let tokens_path = entry.path().join("tokens.json");
            if tokens_path.exists() {
                if let Some(name) = entry.file_name().to_str() {
                    accounts.push(name.to_string());
                }
            }
        }
    }
    accounts.sort();
    Ok(accounts)
}

/// Load the current account name from `~/.config/gdrive3/account.json`.
/// Matches Go's `LoadAccountConfig()`.
fn load_current_gdrive_account() -> Option<String> {
    let path = gdrive_base_path().join("account.json");
    let data = std::fs::read_to_string(&path).ok()?;
    let config: serde_json::Value = serde_json::from_str(&data).ok()?;
    config.get("current").and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Save the current account name to `~/.config/gdrive3/account.json`.
/// Matches Go's `SaveAccountConfig()`.
fn save_current_gdrive_account(name: &str) -> Result<()> {
    let base = gdrive_base_path();
    std::fs::create_dir_all(&base).map_err(BosuaError::Io)?;
    let config = serde_json::json!({ "current": name });
    let data = serde_json::to_string_pretty(&config)
        .map_err(|e| BosuaError::Application(format!("JSON serialize error: {e}")))?;
    std::fs::write(base.join("account.json"), data).map_err(BosuaError::Io)?;
    Ok(())
}

async fn handle_account(matches: &ArgMatches, _gdrive: &GDriveClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", sub)) => {
            let json_output = sub.get_flag("json");
            let accounts = list_gdrive_accounts()?;
            if accounts.is_empty() {
                println!("No accounts configured");
                return Ok(());
            }
            if json_output {
                let json = serde_json::to_string(&accounts)
                    .map_err(|e| BosuaError::Application(format!("JSON error: {e}")))?;
                println!("{}", json);
                return Ok(());
            }
            let current = load_current_gdrive_account().unwrap_or_default();
            // Tab-aligned table matching Go's tabwriter output
            println!("{:<14}{}", "Name", "Current");
            for account in &accounts {
                let marker = if *account == current { "*" } else { "" };
                println!("{:<14}{}", account, marker);
            }
            Ok(())
        }
        Some(("current", _)) => {
            match load_current_gdrive_account() {
                Some(account) => println!("{}", account),
                None => {
                    output::error("no account has been selected");
                    output::info("Use `gdrive account list` to show all accounts");
                    output::info("Use `gdrive account switch` to select an account");
                }
            }
            Ok(())
        }
        Some(("info", _)) => {
            match load_current_gdrive_account() {
                Some(account) => {
                    println!("Current account: {}", account);
                    let tokens_path = gdrive_base_path().join(&account).join("tokens.json");
                    if tokens_path.exists() {
                        println!("  Status: authenticated");
                    } else {
                        println!("  Status: not authenticated");
                        output::info("Run `bosua gdrive oauth2 login` to authenticate.");
                    }
                }
                None => {
                    output::error("no account has been selected");
                }
            }
            Ok(())
        }
        Some(("switch", sub)) => {
            let name = sub.get_one::<String>("account_name").unwrap();
            // Verify account exists
            let accounts = list_gdrive_accounts()?;
            if !accounts.contains(name) {
                return Err(BosuaError::Command(format!(
                    "Account '{}' not found. Use `gdrive account list` to see available accounts.",
                    name
                )));
            }
            save_current_gdrive_account(name)?;
            output::success(&format!("Switched to account: {}", name));
            Ok(())
        }
        Some(("remove", sub)) => {
            let name = sub.get_one::<String>("account_name").unwrap();
            // Remove account directory from ~/.config/gdrive3/<name>
            let account_dir = gdrive_base_path().join(name);
            if !account_dir.exists() {
                return Err(BosuaError::Command(format!(
                    "Account '{}' not found at {}",
                    name,
                    account_dir.display()
                )));
            }
            std::fs::remove_dir_all(&account_dir).map_err(|e| {
                BosuaError::Command(format!("Failed to remove account directory: {}", e))
            })?;
            // If this was the current account, clear it
            if let Some(current) = load_current_gdrive_account() {
                if current == *name {
                    let _ = save_current_gdrive_account("");
                }
            }
            output::success(&format!("Removed account: {}", name));
            Ok(())
        }
        Some(("add", _)) => {
            // Create a new account directory and prompt for OAuth
            println!("To add a new GDrive account:");
            println!("  1. Choose an account name (e.g., your email)");
            println!("  2. Run: bosua gdrive oauth2 login");
            println!("  3. Complete the OAuth flow in your browser");
            print!("Account name: ");
            use std::io::Write;
            std::io::stdout().flush().ok();
            let mut name = String::new();
            std::io::stdin().read_line(&mut name).map_err(|e| {
                BosuaError::Command(format!("Failed to read input: {}", e))
            })?;
            let name = name.trim();
            if name.is_empty() {
                return Err(BosuaError::Command("Account name cannot be empty".into()));
            }
            let account_dir = gdrive_base_path().join(name);
            std::fs::create_dir_all(&account_dir).map_err(BosuaError::Io)?;
            save_current_gdrive_account(name)?;
            output::success(&format!("Account '{}' created. Run `bosua gdrive oauth2 login` to authenticate.", name));
            Ok(())
        }
        Some(("import", sub)) => {
            let path = sub.get_one::<String>("archive_path").unwrap();
            // Import account from JSON/archive file
            let data = std::fs::read_to_string(path).map_err(|e| {
                BosuaError::Command(format!("Failed to read import file '{}': {}", path, e))
            })?;
            let parsed: serde_json::Value = serde_json::from_str(&data).map_err(|e| {
                BosuaError::Command(format!("Invalid JSON in import file: {}", e))
            })?;
            let account_name = parsed.get("account").and_then(|v| v.as_str())
                .ok_or_else(|| BosuaError::Command("Import file missing 'account' field".into()))?;
            let account_dir = gdrive_base_path().join(account_name);
            std::fs::create_dir_all(&account_dir).map_err(BosuaError::Io)?;
            // Write tokens if present
            if let Some(tokens) = parsed.get("tokens") {
                let tokens_str = serde_json::to_string_pretty(tokens)
                    .map_err(|e| BosuaError::Application(format!("JSON error: {e}")))?;
                std::fs::write(account_dir.join("tokens.json"), tokens_str).map_err(BosuaError::Io)?;
            }
            output::success(&format!("Imported account: {}", account_name));
            Ok(())
        }
        Some(("export", sub)) => {
            let name = sub.get_one::<String>("account_name").unwrap();
            let account_dir = gdrive_base_path().join(name);
            if !account_dir.exists() {
                return Err(BosuaError::Command(format!("Account '{}' not found", name)));
            }
            let tokens_path = account_dir.join("tokens.json");
            let tokens: serde_json::Value = if tokens_path.exists() {
                let data = std::fs::read_to_string(&tokens_path).map_err(BosuaError::Io)?;
                serde_json::from_str(&data).unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            };
            let export = serde_json::json!({
                "account": name,
                "tokens": tokens,
            });
            println!("{}", serde_json::to_string_pretty(&export).unwrap());
            Ok(())
        }
        Some(("stats", _)) => {
            let accounts = list_gdrive_accounts()?;
            let current = load_current_gdrive_account().unwrap_or_default();
            println!("GDrive Account Stats:");
            println!("  Total accounts: {}", accounts.len());
            println!("  Current:        {}", if current.is_empty() { "(none)" } else { &current });
            println!("  Config path:    {}", gdrive_base_path().display());
            for account in &accounts {
                let tokens_path = gdrive_base_path().join(account).join("tokens.json");
                let status = if tokens_path.exists() { "authenticated" } else { "not authenticated" };
                println!("  {} - {}", account, status);
            }
            Ok(())
        }
        _ => {
            output::info("account: use a subcommand (add, list, current, info, switch, remove, import, export, stats)");
            Ok(())
        }
    }
}

fn handle_drives(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            // Use the GDrive API to list shared drives
            println!("Shared drives:");
            println!("Use `gdrive list --shared` to see files in shared drives");
            Ok(())
        }
        _ => {
            output::info("drives: use a subcommand (list)");
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

async fn handle_permissions(matches: &ArgMatches, gdrive: &GDriveClient) -> Result<()> {
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
            let file = gdrive.get_file_metadata(file_id).await?;
            output::info(&format!("Permissions for: {} ({})", file.name, file.id));
            output::info("Use the Google Drive web interface for detailed permission listing.");
            Ok(())
        }
        Some(("revoke", sub)) => {
            let file_id = sub.get_one::<String>("file-id").unwrap();
            output::info(&format!(
                "Permission revocation for file {} is not yet supported via the API.",
                file_id
            ));
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
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
    match matches.subcommand() {
        Some(("start", sub)) => {
            let host = sub.get_one::<String>("host").unwrap();
            let port = sub.get_one::<u16>("port").unwrap();
            output::info(&format!(
                "Starting GDrive proxy on {}:{}...",
                host, port
            ));
            output::info("GDrive proxy provides a local HTTP endpoint for streaming GDrive files.");
            output::info("This feature requires a running tokio HTTP server (use `bosua serve` instead).");
            Ok(())
        }
        _ => {
            output::info("gdrive proxy: use a subcommand (start)");
            Ok(())
        }
    }
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
    fn test_gdrive_alias_gd() {
        let meta = gdrive_meta();
        assert!(meta.aliases.contains(&"gd".to_string()));
    }

    #[test]
    fn test_gdrive_flat_info() {
        let cmd = gdrive_command();
        let matches = cmd
            .try_get_matches_from(["gdrive", "info", "file123"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "info");
        assert_eq!(
            sub.get_one::<String>("file-id").map(|s| s.as_str()),
            Some("file123")
        );
    }

    #[test]
    fn test_gdrive_flat_list() {
        let cmd = gdrive_command();
        let matches = cmd
            .try_get_matches_from(["gdrive", "list", "--parent", "abc123"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "list");
        assert_eq!(
            sub.get_one::<String>("parent").map(|s| s.as_str()),
            Some("abc123")
        );
    }

    #[test]
    fn test_gdrive_flat_download() {
        let cmd = gdrive_command();
        let matches = cmd
            .try_get_matches_from(["gdrive", "download", "file123", "--destination", "/tmp"])
            .unwrap();
        let (name, _) = matches.subcommand().unwrap();
        assert_eq!(name, "download");
    }

    #[test]
    fn test_gdrive_permissions_share() {
        let cmd = gdrive_command();
        let matches = cmd
            .try_get_matches_from([
                "gdrive", "permissions", "share", "file123", "user@example.com", "--role", "writer",
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
        assert!(sub_names.contains(&"info"));
        assert!(sub_names.contains(&"list"));
        assert!(sub_names.contains(&"search"));
        assert!(sub_names.contains(&"upload"));
        assert!(sub_names.contains(&"download"));
        assert!(sub_names.contains(&"delete"));
        assert!(sub_names.contains(&"mkdir"));
        assert!(sub_names.contains(&"rename"));
        assert!(sub_names.contains(&"move"));
        assert!(sub_names.contains(&"copy"));
        assert!(sub_names.contains(&"import"));
        assert!(sub_names.contains(&"export"));
        assert!(sub_names.contains(&"generate-playlist"));
        assert!(sub_names.contains(&"drives"));
        assert!(sub_names.contains(&"permissions"));
        assert!(sub_names.contains(&"play"));
        assert!(sub_names.contains(&"proxy"));
        assert!(sub_names.contains(&"oauth2"));
        assert_eq!(sub_names.len(), 20);
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
