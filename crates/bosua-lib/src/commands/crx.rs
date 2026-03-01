//! CRX CLI command — Chrome CRX extension operations.
//!
//! Subcommands: convert, download.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};
use crate::http_client::HttpClient;

/// Chrome Web Store CRX download URL template.
const CRX_DOWNLOAD_URL: &str = "https://clients2.google.com/service/update2/crx?response=redirect&acceptformat=crx2,crx3&prodversion=120.0&x=id%3D{EXT_ID}%26installsource%3Dondemand%26uc";

/// Build the `crx` clap command.
pub fn crx_command() -> Command {
    Command::new("crx")
        .about("Chrome CRX extension operations")
        .aliases(["chrome"])
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("convert")
                .about("Convert extension")
                .aliases(["cv", "c"])
                .arg(Arg::new("archive").long("archive").action(clap::ArgAction::SetTrue).help("Export xcode project to application"))
                .arg(Arg::new("install").long("install").action(clap::ArgAction::SetTrue).help("Install after export"))
                .arg(Arg::new("remove").long("remove").action(clap::ArgAction::SetTrue).help("Remove the source code after convert"))
                .arg(Arg::new("team").long("team").default_value("Z6322AZ9HJ").help("Developer Account Team ID")),
        )
        .subcommand(
            Command::new("download")
                .about("Download extension")
                .aliases(["dl", "d"])
                .arg(Arg::new("archive").long("archive").action(clap::ArgAction::SetTrue).help("Export xcode project to application"))
                .arg(Arg::new("convert").long("convert").action(clap::ArgAction::SetTrue).help("Convert after extract"))
                .arg(Arg::new("extract").long("extract").action(clap::ArgAction::SetTrue).help("Extract after download"))
                .arg(Arg::new("install").long("install").action(clap::ArgAction::SetTrue).help("Install after export"))
                .arg(Arg::new("team").long("team").default_value("Z6322AZ9HJ").help("Developer Account Team ID")),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn crx_meta() -> CommandMeta {
    CommandBuilder::from_clap(crx_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `crx` command.
pub async fn handle_crx(matches: &ArgMatches, http: &HttpClient) -> Result<()> {
    match matches.subcommand() {
        Some(("convert", sub)) => handle_convert(sub).await,
        Some(("download", sub)) => handle_download(sub, http).await,
        _ => unreachable!("subcommand_required is set"),
    }
}

async fn handle_convert(matches: &ArgMatches) -> Result<()> {
    let archive = matches.get_flag("archive");
    let install = matches.get_flag("install");
    let remove = matches.get_flag("remove");
    let team = matches.get_one::<String>("team").unwrap();

    // Find extracted extension directories in current directory
    let cwd = std::env::current_dir().map_err(BosuaError::Io)?;
    let mut ext_dirs: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&cwd) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("manifest.json").exists() {
                ext_dirs.push(path);
            }
        }
    }

    if ext_dirs.is_empty() {
        return Err(BosuaError::Command(
            "No extracted extension directories found (looking for directories with manifest.json)".into(),
        ));
    }

    for ext_dir in &ext_dirs {
        let ext_name = ext_dir.file_name().and_then(|n| n.to_str()).unwrap_or("extension");
        println!("Converting {} from CRX to Xcode project", ext_name);

        let xcode_proj_path = cwd.join("Xcodeproj");
        let mut cmd_args = vec![
            "xcrun", "safari-web-extension-converter",
            "--no-open", "--macos-only", "--copy-resources",
            "--project-location",
        ];
        let xcode_str = xcode_proj_path.to_string_lossy().to_string();
        cmd_args.push(&xcode_str);
        let ext_str = ext_dir.to_string_lossy().to_string();
        cmd_args.push(&ext_str);

        let status = tokio::process::Command::new(cmd_args[0])
            .args(&cmd_args[1..])
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .await
            .map_err(|e| BosuaError::Command(format!("Failed to run xcrun: {}", e)))?;

        if !status.success() {
            println!("Warning: conversion failed for {}", ext_name);
            continue;
        }

        if archive {
            let project_path = format!("{}/{}/{}.xcodeproj", xcode_str, ext_name, ext_name);
            let archive_path = format!("{}/{}.xcarchive", xcode_str, ext_name);
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let export_dir = format!("{}/{}-{}", cwd.display(), ext_name, timestamp);

            // xcodebuild archive
            let arch_status = tokio::process::Command::new("xcodebuild")
                .args([
                    "-project", &project_path,
                    "-scheme", ext_name,
                    "-archivePath", &archive_path,
                    "archive",
                    &format!("DEVELOPMENT_TEAM={}", team),
                ])
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()
                .await
                .map_err(|e| BosuaError::Command(format!("xcodebuild archive failed: {}", e)))?;

            if arch_status.success() {
                // Export
                let plist_path = cwd.join("ExportOptions.plist");
                if !plist_path.exists() {
                    let plist_content = format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>method</key><string>developer-id</string>
<key>teamID</key><string>{}</string>
</dict></plist>"#,
                        team
                    );
                    std::fs::write(&plist_path, plist_content).map_err(BosuaError::Io)?;
                }

                let _ = tokio::process::Command::new("xcodebuild")
                    .args([
                        "-exportArchive",
                        "-archivePath", &archive_path,
                        "-exportPath", &export_dir,
                        "-exportOptionsPlist", &plist_path.to_string_lossy(),
                    ])
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit())
                    .status()
                    .await;

                if install {
                    // Find .app in export dir and copy to /Applications
                    if let Ok(entries) = std::fs::read_dir(&export_dir) {
                        for entry in entries.flatten() {
                            if entry.path().extension().map(|e| e == "app").unwrap_or(false) {
                                let dest = format!("/Applications/{}", entry.file_name().to_string_lossy());
                                let _ = std::fs::rename(entry.path(), &dest);
                                println!("Installed to {}", dest);
                            }
                        }
                    }
                }
            }
            // Cleanup Xcode project
            let _ = std::fs::remove_dir_all(&xcode_proj_path);
        }

        if remove {
            let _ = std::fs::remove_dir_all(ext_dir);
        }
    }
    Ok(())
}

async fn handle_download(matches: &ArgMatches, http: &HttpClient) -> Result<()> {
    let extract = matches.get_flag("extract");
    let convert = matches.get_flag("convert");
    let archive = matches.get_flag("archive");
    let install = matches.get_flag("install");
    let team = matches.get_one::<String>("team").unwrap();

    // Read extension URL/ID from stdin or trailing args
    print!("Enter Chrome Web Store URL or extension ID: ");
    use std::io::Write;
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).map_err(|e| {
        BosuaError::Command(format!("Failed to read input: {}", e))
    })?;
    let input = input.trim();
    if input.is_empty() {
        return Err(BosuaError::Command("No extension URL or ID provided".into()));
    }

    // Extract extension ID and name from URL
    let (ext_name, ext_id) = if input.contains('/') {
        // It's a URL — extract ID and name
        let parts: Vec<&str> = input.split('/').collect();
        if parts.len() >= 6 {
            (parts[4].to_string(), parts[5].split('?').next().unwrap_or(parts[5]).to_string())
        } else {
            (input.to_string(), input.to_string())
        }
    } else {
        (input.to_string(), input.to_string())
    };

    let download_url = CRX_DOWNLOAD_URL.replace("{EXT_ID}", &ext_id);
    let output_path = format!("{}.crx", ext_name);

    println!("Downloading extension: {}", ext_name);
    let client = http.get_client().await;
    let resp = client.get(&download_url).send().await
        .map_err(|e| BosuaError::Command(format!("Failed to download CRX: {}", e)))?;

    if !resp.status().is_success() {
        return Err(BosuaError::Command(format!(
            "Download failed with status: {}",
            resp.status()
        )));
    }

    let bytes = resp.bytes().await
        .map_err(|e| BosuaError::Command(format!("Failed to read CRX data: {}", e)))?;
    std::fs::write(&output_path, &bytes).map_err(BosuaError::Io)?;
    println!("Downloaded: {} ({} bytes)", output_path, bytes.len());

    if extract || convert {
        // Extract CRX (it's a ZIP with a CRX header)
        let extract_dir = ext_name.clone();
        println!("Extracting to {}/", extract_dir);

        std::fs::create_dir_all(&extract_dir).map_err(BosuaError::Io)?;
        let unzip_status = tokio::process::Command::new("unzip")
            .args(["-o", &output_path, "-d", &extract_dir])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .map_err(|e| BosuaError::Command(format!("Failed to run unzip: {}", e)))?;

        if !unzip_status.success() {
            return Err(BosuaError::Command("Failed to extract CRX file".into()));
        }
        println!("Extracted to {}/", extract_dir);

        if convert {
            // Use xcrun to convert
            println!("Converting to Safari extension...");
            let xcode_proj = "Xcodeproj";
            let status = tokio::process::Command::new("xcrun")
                .args([
                    "safari-web-extension-converter",
                    "--no-open", "--macos-only", "--copy-resources",
                    "--project-location", xcode_proj,
                    &extract_dir,
                ])
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()
                .await
                .map_err(|e| BosuaError::Command(format!("xcrun failed: {}", e)))?;

            if status.success() && archive {
                let project_path = format!("{}/{}/{}.xcodeproj", xcode_proj, ext_name, ext_name);
                let archive_path = format!("{}/{}.xcarchive", xcode_proj, ext_name);
                let _ = tokio::process::Command::new("xcodebuild")
                    .args([
                        "-project", &project_path,
                        "-scheme", &ext_name,
                        "-archivePath", &archive_path,
                        "archive",
                        &format!("DEVELOPMENT_TEAM={}", team),
                    ])
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit())
                    .status()
                    .await;

                if install {
                    println!("Install step: check {}/", xcode_proj);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crx_command_parses() {
        let cmd = crx_command();
        let m = cmd.try_get_matches_from(["crx", "download"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("download"));
    }

    #[test]
    fn test_crx_alias_chrome() {
        let cmd = crx_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"chrome"));
    }

    #[test]
    fn test_crx_convert_flags() {
        let cmd = crx_command();
        let m = cmd.try_get_matches_from(["crx", "convert", "--archive", "--install", "--remove"]).unwrap();
        let (name, sub) = m.subcommand().unwrap();
        assert_eq!(name, "convert");
        assert!(sub.get_flag("archive"));
        assert!(sub.get_flag("install"));
        assert!(sub.get_flag("remove"));
    }

    #[test]
    fn test_crx_download_flags() {
        let cmd = crx_command();
        let m = cmd.try_get_matches_from(["crx", "download", "--extract", "--convert"]).unwrap();
        let (name, sub) = m.subcommand().unwrap();
        assert_eq!(name, "download");
        assert!(sub.get_flag("extract"));
        assert!(sub.get_flag("convert"));
    }

    #[test]
    fn test_crx_requires_subcommand() {
        let cmd = crx_command();
        assert!(cmd.try_get_matches_from(["crx"]).is_err());
    }

    #[test]
    fn test_crx_meta() {
        let meta = crx_meta();
        assert_eq!(meta.name, "crx");
        assert_eq!(meta.category, CommandCategory::Utility);
    }
}
