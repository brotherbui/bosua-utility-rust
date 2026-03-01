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

fn vmf_add(_matches: &ArgMatches) -> Result<()> {
    // Interactive add: prompt for author, note, sheet URL
    use std::io::Write;
    let stdin = std::io::stdin();

    print!("Author: ");
    std::io::stdout().flush().ok();
    let mut author = String::new();
    stdin.read_line(&mut author).map_err(|e| BosuaError::Command(format!("Read error: {}", e)))?;
    let author = author.trim().to_string();

    print!("Note: ");
    std::io::stdout().flush().ok();
    let mut note = String::new();
    stdin.read_line(&mut note).map_err(|e| BosuaError::Command(format!("Read error: {}", e)))?;
    let note = note.trim().to_string();

    print!("Sheet URL or ID: ");
    std::io::stdout().flush().ok();
    let mut url_input = String::new();
    stdin.read_line(&mut url_input).map_err(|e| BosuaError::Command(format!("Read error: {}", e)))?;
    let url_input = url_input.trim().to_string();

    if author.is_empty() || note.is_empty() || url_input.is_empty() {
        return Err(BosuaError::Command("All fields are required".into()));
    }

    let sheet_id = extract_spreadsheet_id(&url_input)
        .unwrap_or_else(|| url_input.clone());
    let sheet_url = if url_input.contains("docs.google.com") {
        url_input.clone()
    } else {
        format!("https://docs.google.com/spreadsheets/d/{}/edit", sheet_id)
    };

    let conn = vmf_open_db()?;
    conn.execute(
        "INSERT INTO vmf_sources (author, note, sheet_url, sheet_id) VALUES (?, ?, ?, ?)",
        rusqlite::params![author, note, sheet_url, sheet_id],
    ).map_err(|e| BosuaError::Application(format!("Failed to add source: {}", e)))?;

    output::success(&format!("Added VMF source: {} - {}", author, note));
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
    let id: i64 = id_str.parse().map_err(|_| BosuaError::Command("Invalid ID. Must be a number.".into()))?;

    let conn = vmf_open_db()?;

    // Get current values
    let current = conn.query_row(
        "SELECT author, note, sheet_url, sheet_id FROM vmf_sources WHERE id = ?",
        [id],
        |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        )),
    ).map_err(|_| BosuaError::Command(format!("Source not found: id {}", id)))?;

    use std::io::Write;
    let stdin = std::io::stdin();

    print!("Author [{}]: ", current.0);
    std::io::stdout().flush().ok();
    let mut author = String::new();
    stdin.read_line(&mut author).map_err(|e| BosuaError::Command(format!("Read error: {}", e)))?;
    let author = if author.trim().is_empty() { current.0 } else { author.trim().to_string() };

    print!("Note [{}]: ", current.1);
    std::io::stdout().flush().ok();
    let mut note = String::new();
    stdin.read_line(&mut note).map_err(|e| BosuaError::Command(format!("Read error: {}", e)))?;
    let note = if note.trim().is_empty() { current.1 } else { note.trim().to_string() };

    print!("Sheet URL [{}]: ", current.2);
    std::io::stdout().flush().ok();
    let mut url_input = String::new();
    stdin.read_line(&mut url_input).map_err(|e| BosuaError::Command(format!("Read error: {}", e)))?;
    let (sheet_url, sheet_id) = if url_input.trim().is_empty() {
        (current.2, current.3)
    } else {
        let url = url_input.trim().to_string();
        let sid = extract_spreadsheet_id(&url).unwrap_or_else(|| url.clone());
        let surl = if url.contains("docs.google.com") { url } else {
            format!("https://docs.google.com/spreadsheets/d/{}/edit", sid)
        };
        (surl, sid)
    };

    conn.execute(
        "UPDATE vmf_sources SET author = ?, note = ?, sheet_url = ?, sheet_id = ?, updated_at = datetime('now') WHERE id = ?",
        rusqlite::params![author, note, sheet_url, sheet_id, id],
    ).map_err(|e| BosuaError::Application(format!("Failed to update: {}", e)))?;

    output::success(&format!("Updated VMF source ID {}", id));
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
        output::info(&format!("Database already has {} sources. No default sources added.", count));
        return Ok(());
    }

    // Embedded default sources matching Go's GetDefaultSources()
    let defaults = [
        ("zinzuno", "Phim bộ TQ", "https://docs.google.com/spreadsheets/d/1Ejw4-XAT9EUuieKmLMtKwBGCtFcxsFkjfYRpqlrocfg/edit", "1Ejw4-XAT9EUuieKmLMtKwBGCtFcxsFkjfYRpqlrocfg"),
        ("zinzuno", "Phim bộ HQ", "https://docs.google.com/spreadsheets/d/1AMNbCL4LBxC3yO5t5O_olMLRYqfEuV5ODcPj4kx0ZRk/edit", "1AMNbCL4LBxC3yO5t5O_olMLRYqfEuV5ODcPj4kx0ZRk"),
        ("zinzuno", "Phim bộ TVB", "https://docs.google.com/spreadsheets/d/1sRtL12Z-nJ3oktb14zy-qu9I_rXjR8LxQljoLwaxmF0/edit", "1sRtL12Z-nJ3oktb14zy-qu9I_rXjR8LxQljoLwaxmF0"),
        ("zinzuno", "Phim bộ Khác", "https://docs.google.com/spreadsheets/d/1PiVZWdvshhjMn3cd2QRB5hkd92ZqawnlsqGG1fD6WhQ/edit", "1PiVZWdvshhjMn3cd2QRB5hkd92ZqawnlsqGG1fD6WhQ"),
        ("zinzuno", "Phim lẻ 2025", "https://docs.google.com/spreadsheets/d/1AUso14EWNjs4Fzs-Gu_W4Cjy4f4BCmHapmyppHxQupo/edit", "1AUso14EWNjs4Fzs-Gu_W4Cjy4f4BCmHapmyppHxQupo"),
        ("zinzuno", "Phim lẻ 2024", "https://docs.google.com/spreadsheets/d/1D3UoGVSJwKp11fpZU9TQjIORFMViF1TFu-BQdGqmB3o/edit", "1D3UoGVSJwKp11fpZU9TQjIORFMViF1TFu-BQdGqmB3o"),
        ("zinzuno", "Hoạt Hình", "https://docs.google.com/spreadsheets/d/1_Gw_dZbrr6NF9mBi23BIues1F36je5uVidRiFHpgixU/edit", "1_Gw_dZbrr6NF9mBi23BIues1F36je5uVidRiFHpgixU"),
        ("LinhHuynh", "Phim bộ", "https://docs.google.com/spreadsheets/d/1z0kZmoa0roZ1wocW3VZc8kwUSbRYFSC7KSP8pbMQt50/edit", "1z0kZmoa0roZ1wocW3VZc8kwUSbRYFSC7KSP8pbMQt50"),
        ("LinhHuynh", "Phim lẻ", "https://docs.google.com/spreadsheets/d/15gzhKzSTX-B-xO2vukcV-z-XzYSamlsSoCAyakmVt40/edit", "15gzhKzSTX-B-xO2vukcV-z-XzYSamlsSoCAyakmVt40"),
        ("LinhHuynh", "Hoạt Hình", "https://docs.google.com/spreadsheets/d/1vGVWjdEjHowW5I_M42kItPYquOX6kiIX/edit", "1vGVWjdEjHowW5I_M42kItPYquOX6kiIX"),
    ];

    let mut added = 0;
    for (author, note, url, sheet_id) in &defaults {
        let result = conn.execute(
            "INSERT OR IGNORE INTO vmf_sources (author, note, sheet_url, sheet_id) VALUES (?, ?, ?, ?)",
            rusqlite::params![author, note, url, sheet_id],
        );
        if let Ok(n) = result {
            if n > 0 { added += 1; }
        }
    }

    output::success(&format!("Initialized database with {} default sources", added));
    Ok(())
}

fn vmf_migrate() -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let text_file = format!("{}/bosua-vmf-sources.txt", home);

    if !std::path::Path::new(&text_file).exists() {
        return Err(BosuaError::Command(format!("Text file not found: {}", text_file)));
    }

    let content = std::fs::read_to_string(&text_file).map_err(BosuaError::Io)?;
    let conn = vmf_open_db()?;

    let mut imported = 0;
    let mut skipped = 0;

    for (line_num, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse CSV: Author,Description,Link
        let parts: Vec<&str> = line.splitn(3, ',').collect();
        let (author, note, link) = if parts.len() >= 3 {
            (parts[0].trim(), parts[1].trim(), parts[2].trim())
        } else {
            ("Unknown", "Imported from text file", line)
        };

        let sheet_id = match extract_spreadsheet_id(link) {
            Some(id) => id,
            None => {
                println!("  Line {}: Invalid Google Sheets URL: {}", line_num + 1, link);
                skipped += 1;
                continue;
            }
        };

        let result = conn.execute(
            "INSERT OR IGNORE INTO vmf_sources (author, note, sheet_url, sheet_id) VALUES (?, ?, ?, ?)",
            rusqlite::params![author, note, link, sheet_id],
        );

        match result {
            Ok(n) if n > 0 => imported += 1,
            Ok(_) => skipped += 1,
            Err(e) => {
                println!("  Line {}: Failed to add: {}", line_num + 1, e);
                skipped += 1;
            }
        }
    }

    output::success(&format!("Migration complete: {} imported, {} skipped", imported, skipped));
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
