//! CRX CLI command — Chrome extension utilities.
//!
//! Downloads Chrome extensions by ID from the Chrome Web Store.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};
use crate::http_client::HttpClient;

/// Chrome Web Store CRX download URL template.
const CRX_DOWNLOAD_URL: &str = "https://clients2.google.com/service/update2/crx?response=redirect&acceptformat=crx2,crx3&prodversion=120.0&x=id%3D{EXT_ID}%26installsource%3Dondemand%26uc";

/// Build the `crx` clap command.
pub fn crx_command() -> Command {
    Command::new("crx")
        .about("Chrome extension utilities")
        .arg(
            Arg::new("extension-id")
                .required(true)
                .help("Chrome extension ID"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output path for downloaded extension"),
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
    let ext_id = matches.get_one::<String>("extension-id").unwrap();
    let output = matches.get_one::<String>("output");

    // Validate extension ID format (32 lowercase letters)
    if ext_id.len() != 32 || !ext_id.chars().all(|c| c.is_ascii_lowercase()) {
        return Err(BosuaError::Command(format!(
            "Invalid extension ID '{}': expected 32 lowercase letters",
            ext_id
        )));
    }

    let url = CRX_DOWNLOAD_URL.replace("{EXT_ID}", ext_id);
    let output_path = output
        .map(|o| o.to_string())
        .unwrap_or_else(|| format!("{}.crx", ext_id));

    println!("Downloading extension: {}", ext_id);

    let client = http.get_client().await;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| BosuaError::Command(format!("Failed to download CRX: {}", e)))?;

    if !resp.status().is_success() {
        return Err(BosuaError::Command(format!(
            "Failed to download CRX: HTTP {}",
            resp.status()
        )));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| BosuaError::Command(format!("Failed to read CRX data: {}", e)))?;

    std::fs::write(&output_path, &bytes).map_err(|e| {
        BosuaError::Command(format!("Failed to write CRX to '{}': {}", output_path, e))
    })?;

    println!("Downloaded {} bytes to {}", bytes.len(), output_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crx_command_parses() {
        let cmd = crx_command();
        let matches = cmd
            .try_get_matches_from(["crx", "abcdefghijklmnopqrstuvwxyz"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("extension-id").map(|s| s.as_str()),
            Some("abcdefghijklmnopqrstuvwxyz"),
        );
    }

    #[test]
    fn test_crx_command_with_output() {
        let cmd = crx_command();
        let matches = cmd
            .try_get_matches_from(["crx", "ext123", "--output", "/tmp/ext.crx"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("output").map(|s| s.as_str()),
            Some("/tmp/ext.crx"),
        );
    }

    #[test]
    fn test_crx_requires_extension_id() {
        let cmd = crx_command();
        let result = cmd.try_get_matches_from(["crx"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_crx_meta() {
        let meta = crx_meta();
        assert_eq!(meta.name, "crx");
        assert_eq!(meta.category, CommandCategory::Utility);
        assert!(!meta.description.is_empty());
    }
}
