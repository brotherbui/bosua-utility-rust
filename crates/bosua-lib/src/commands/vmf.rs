//! VMF CLI command — Manage VMF sources (Google Sheets) for movie searches.
//!
//! Subcommands: add, delete, disable, edit, enable, info, init, list, migrate.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};
use crate::output;

use rusqlite;

/// Build the `vmf` clap command.
pub fn vmf_command() -> Command {
    Command::new("vmf")
        .about("Manage VMF sources (Google Sheets) for movie searches. Supports list, add, delete, edit, and migrate operations.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("add").about("Add a new VMF source"))
        .subcommand(
            Command::new("delete")
                .about("Delete a VMF source by ID")
                .arg(Arg::new("id").required(true).help("VMF source ID")),
        )
        .subcommand(
            Command::new("disable")
                .about("Disable a VMF source")
                .arg(Arg::new("id").required(true).help("VMF source ID")),
        )
        .subcommand(
            Command::new("edit")
                .about("Edit an existing VMF source")
                .arg(Arg::new("id").required(true).help("VMF source ID")),
        )
        .subcommand(
            Command::new("enable")
                .about("Enable a VMF source")
                .arg(Arg::new("id").required(true).help("VMF source ID")),
        )
        .subcommand(
            Command::new("info")
                .about("Show detailed information about a VMF source")
                .arg(Arg::new("id").required(true).help("VMF source ID")),
        )
        .subcommand(Command::new("init").about("Initialize database with default VMF sources"))
        .subcommand(Command::new("list").about("List all VMF sources"))
        .subcommand(Command::new("migrate").about("Migrate sources from text file to database"))
}

/// Build the `CommandMeta` for registry registration.
pub fn vmf_meta() -> CommandMeta {
    CommandBuilder::from_clap(vmf_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `vmf` command.
pub fn handle_vmf(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("add", sub)) => vmf_add(sub),
        Some(("delete", sub)) => vmf_delete(sub),
        Some(("disable", sub)) => vmf_set_active(sub, false),
        Some(("edit", sub)) => vmf_edit(sub),
        Some(("enable", sub)) => vmf_set_active(sub, true),
        Some(("info", sub)) => vmf_info(sub),
        Some(("init", _)) => vmf_init(),
        Some(("list", _)) => vmf_list(),
        Some(("migrate", _)) => vmf_migrate(),
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// VMF SQLite helpers
// ---------------------------------------------------------------------------

/// Path to the VMF sources database: `~/.bosua/vmf-sources.db`
fn vmf_db_path() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
    home.join(".bosua").join("vmf-sources.db")
}

/// Open (and initialize) the VMF database.
fn vmf_open_db() -> Result<rusqlite::Connection> {
    let path = vmf_db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(BosuaError::Io)?;
    }
    let conn = rusqlite::Connection::open(&path)
        .map_err(|e| BosuaError::Application(format!("Failed to open VMF database: {}", e)))?;

    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;
         CREATE TABLE IF NOT EXISTS vmf_sources (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             author TEXT NOT NULL,
             note TEXT NOT NULL,
             sheet_url TEXT NOT NULL,
             sheet_id TEXT NOT NULL,
             is_active INTEGER NOT NULL DEFAULT 1,
             created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
             updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
         );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_vmf_sources_sheet_id ON vmf_sources(sheet_id);
         CREATE INDEX IF NOT EXISTS idx_vmf_sources_active ON vmf_sources(is_active);
         CREATE INDEX IF NOT EXISTS idx_vmf_sources_author ON vmf_sources(author);",
    )
    .map_err(|e| BosuaError::Application(format!("Failed to init VMF schema: {}", e)))?;

    Ok(conn)
}

/// Extract spreadsheet ID from a Google Sheets URL.
fn extract_spreadsheet_id(url: &str) -> Option<String> {
    // Match /spreadsheets/d/{ID}
    let marker = "/spreadsheets/d/";
    let start = url.find(marker)? + marker.len();
    let rest = &url[start..];
    let end = rest.find('/').unwrap_or(rest.len());
    let id = &rest[..end];
    if id.is_empty() { None } else { Some(id.to_string()) }
}

fn vmf_list() -> Result<()> {
    let conn = vmf_open_db()?;
    let mut stmt = conn
        .prepare("SELECT id, author, note, sheet_url, sheet_id, is_active, created_at, updated_at FROM vmf_sources ORDER BY id ASC")
        .map_err(|e| BosuaError::Application(format!("Query error: {}", e)))?;

    let sources: Vec<(i64, String, String, String, String, bool, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get::<_, i32>(5)? == 1,
                row.get::<_, String>(6).unwrap_or_default(),
                row.get::<_, String>(7).unwrap_or_default(),
            ))
        })
        .map_err(|e| BosuaError::Application(format!("Query error: {}", e)))?
        .filter_map(|r| r.ok())
        .collect();

    if sources.is_empty() {
        output::warning("No VMF sources found. Use 'bosua vmf migrate' to import from text file or 'bosua vmf add' to add a new source.");
        return Ok(());
    }

    output::info("VMF Sources:");
    println!("{}", "=".repeat(80));
    println!("{:<5} {:<20} {:<30} {:<10}", "ID", "Author", "Note", "Status");
    println!("{}", "-".repeat(80));

    for (id, author, note, _url, _sheet_id, is_active, _created, _updated) in &sources {
        let author_display = if author.len() > 20 { format!("{}...", &author[..17]) } else { author.clone() };
        let note_display = if note.len() > 30 { format!("{}...", &note[..27]) } else { note.clone() };
        let status = if *is_active { "active" } else { "inactive" };
        println!("{:<5} {:<20} {:<30} {}", id, author_display, note_display, status);
    }

    println!("{}", "=".repeat(80));
    println!("Total: {} sources", sources.len());
    println!("\nDatabase location: {}", vmf_db_path().display());
    Ok(())
}

fn vmf_add(matches: &ArgMatches) -> Result<()> {
    // Go expects: vmf add <author> <note> <sheet_url_or_id>
    // But our CLI doesn't have positional args for add. Delegate to Go.
    let go_bin = "/opt/homebrew/bin/bosua";
    if !std::path::Path::new(go_bin).exists() {
        return Err(BosuaError::Command(
            "vmf add: requires positional args (author, note, sheet_url_or_id). Use Go binary.".into(),
        ));
    }
    let status = std::process::Command::new(go_bin)
        .args(["vmf", "add"])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| BosuaError::Command(format!("Failed to run Go binary: {}", e)))?;
    if !status.success() {
        return Err(BosuaError::Command("vmf add failed".into()));
    }
    Ok(())
}

fn vmf_delete(matches: &ArgMatches) -> Result<()> {
    let id_str = matches.get_one::<String>("id").unwrap();
    let id: i64 = id_str.parse().map_err(|_| BosuaError::Command("Invalid ID. Must be a number.".into()))?;

    let conn = vmf_open_db()?;

    // Get source info first
    let source: Option<(String, String)> = conn
        .query_row(
            "SELECT author, note FROM vmf_sources WHERE id = ?",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    let affected = conn
        .execute("DELETE FROM vmf_sources WHERE id = ?", [id])
        .map_err(|e| BosuaError::Application(format!("Failed to delete: {}", e)))?;

    if affected == 0 {
        output::error(&format!("Source not found: id {}", id));
    } else if let Some((author, note)) = source {
        output::success(&format!("Successfully deleted VMF source: {} - {} (ID: {})", author, note, id));
    } else {
        output::success(&format!("Successfully deleted VMF source ID {}", id));
    }
    Ok(())
}

fn vmf_set_active(matches: &ArgMatches, active: bool) -> Result<()> {
    let id_str = matches.get_one::<String>("id").unwrap();
    let id: i64 = id_str.parse().map_err(|_| BosuaError::Command("Invalid ID. Must be a number.".into()))?;

    let conn = vmf_open_db()?;
    let active_int: i32 = if active { 1 } else { 0 };
    let affected = conn
        .execute(
            "UPDATE vmf_sources SET is_active = ?, updated_at = datetime('now') WHERE id = ?",
            rusqlite::params![active_int, id],
        )
        .map_err(|e| BosuaError::Application(format!("Failed to update: {}", e)))?;

    if affected == 0 {
        output::error(&format!("Source not found: id {}", id));
    } else {
        let action = if active { "enabled" } else { "disabled" };
        output::success(&format!("Successfully {} VMF source ID {}", action, id));
    }
    Ok(())
}

fn vmf_edit(matches: &ArgMatches) -> Result<()> {
    let id_str = matches.get_one::<String>("id").unwrap();
    // Go expects: vmf edit <id> <author> <note> <sheet_url_or_id>
    // Delegate to Go binary for the full interactive edit
    let go_bin = "/opt/homebrew/bin/bosua";
    if !std::path::Path::new(go_bin).exists() {
        return Err(BosuaError::Command(
            "vmf edit: requires positional args. Use Go binary.".into(),
        ));
    }
    let status = std::process::Command::new(go_bin)
        .args(["vmf", "edit", id_str])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| BosuaError::Command(format!("Failed to run Go binary: {}", e)))?;
    if !status.success() {
        return Err(BosuaError::Command("vmf edit failed".into()));
    }
    Ok(())
}

fn vmf_info(matches: &ArgMatches) -> Result<()> {
    let id_str = matches.get_one::<String>("id").unwrap();
    let id: i64 = id_str.parse().map_err(|_| BosuaError::Command("Invalid ID. Must be a number.".into()))?;

    let conn = vmf_open_db()?;
    let result = conn.query_row(
        "SELECT id, author, note, sheet_url, sheet_id, is_active, created_at, updated_at FROM vmf_sources WHERE id = ?",
        [id],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i32>(5)? == 1,
                row.get::<_, String>(6).unwrap_or_default(),
                row.get::<_, String>(7).unwrap_or_default(),
            ))
        },
    );

    match result {
        Ok((id, author, note, sheet_url, sheet_id, is_active, created, updated)) => {
            let status = if is_active { "Active" } else { "Inactive" };
            output::info("VMF Source Details:");
            println!("{}", "=".repeat(80));
            println!("ID:         {}", id);
            println!("Author:     {}", author);
            println!("Note:       {}", note);
            println!("Sheet URL:  {}", sheet_url);
            println!("Sheet ID:   {}", sheet_id);
            println!("Status:     {}", status);
            println!("Created:    {}", created);
            println!("Updated:    {}", updated);
            println!("{}", "=".repeat(80));
        }
        Err(_) => {
            output::error(&format!("Source not found: id {}", id));
        }
    }
    Ok(())
}

fn vmf_init() -> Result<()> {
    let conn = vmf_open_db()?;
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM vmf_sources", [], |row| row.get(0))
        .unwrap_or(0);

    if count > 0 {
        output::info("Database already has sources. No default sources added.");
        return Ok(());
    }

    // Delegate to Go binary which has embedded default sources
    let go_bin = "/opt/homebrew/bin/bosua";
    if !std::path::Path::new(go_bin).exists() {
        return Err(BosuaError::Command(
            "vmf init: default sources are embedded in Go binary. Install Go binary first.".into(),
        ));
    }
    let status = std::process::Command::new(go_bin)
        .args(["vmf", "init"])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| BosuaError::Command(format!("Failed to run Go binary: {}", e)))?;
    if !status.success() {
        return Err(BosuaError::Command("vmf init failed".into()));
    }
    Ok(())
}

fn vmf_migrate() -> Result<()> {
    // Delegate to Go binary which has the text file migration logic
    let go_bin = "/opt/homebrew/bin/bosua";
    if !std::path::Path::new(go_bin).exists() {
        return Err(BosuaError::Command(
            "vmf migrate requires the Go binary at /opt/homebrew/bin/bosua".into(),
        ));
    }
    let status = std::process::Command::new(go_bin)
        .args(["vmf", "migrate"])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| BosuaError::Command(format!("Failed to run Go binary: {}", e)))?;
    if !status.success() {
        return Err(BosuaError::Command("vmf migrate failed".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vmf_command_parses() {
        let cmd = vmf_command();
        let m = cmd.try_get_matches_from(["vmf", "list"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("list"));
    }

    #[test]
    fn test_vmf_requires_subcommand() {
        let cmd = vmf_command();
        assert!(cmd.try_get_matches_from(["vmf"]).is_err());
    }

    #[test]
    fn test_vmf_delete() {
        let cmd = vmf_command();
        let m = cmd.try_get_matches_from(["vmf", "delete", "42"]).unwrap();
        let (name, sub) = m.subcommand().unwrap();
        assert_eq!(name, "delete");
        assert_eq!(sub.get_one::<String>("id").map(|s| s.as_str()), Some("42"));
    }

    #[test]
    fn test_vmf_meta() {
        let meta = vmf_meta();
        assert_eq!(meta.name, "vmf");
        assert_eq!(meta.category, CommandCategory::Utility);
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = vmf_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        for name in &["add", "delete", "disable", "edit", "enable", "info", "init", "list", "migrate"] {
            assert!(sub_names.contains(name), "missing subcommand: {}", name);
        }
        assert_eq!(sub_names.len(), 9);
    }
}
