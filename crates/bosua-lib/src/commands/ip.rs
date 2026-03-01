//! IP CLI command — IP address utilities.
//!
//! Subcommands: local, public, geo.

use clap::{ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::BosuaError;
use crate::http_client::HttpClient;

/// Build the `ip` clap command.
pub fn ip_command() -> Command {
    Command::new("ip")
        .about("IP stuffs")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("info").about("Get current public IP info"))
}

/// Build the `CommandMeta` for registry registration.
pub fn ip_meta() -> CommandMeta {
    CommandBuilder::from_clap(ip_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `ip` command.
/// Handle the `ip` command.
pub async fn handle_ip(matches: &ArgMatches, http: &HttpClient) -> Result<(), BosuaError> {
    match matches.subcommand() {
        Some(("info", _)) => {
            let client = http.get_client().await;
            let resp = client
                .get("https://ipinfo.io/json")
                .send()
                .await
                .map_err(|e| BosuaError::Command(format!("Failed to query IP info: {}", e)))?;
            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| BosuaError::Command(format!("Failed to parse response: {}", e)))?;

            if let Some(ip) = body.get("ip").and_then(|v| v.as_str()) {
                println!("IP:       {}", ip);
            }
            if let Some(city) = body.get("city").and_then(|v| v.as_str()) {
                println!("City:     {}", city);
            }
            if let Some(region) = body.get("region").and_then(|v| v.as_str()) {
                println!("Region:   {}", region);
            }
            if let Some(country) = body.get("country").and_then(|v| v.as_str()) {
                println!("Country:  {}", country);
            }
            if let Some(loc) = body.get("loc").and_then(|v| v.as_str()) {
                println!("Location: {}", loc);
            }
            if let Some(org) = body.get("org").and_then(|v| v.as_str()) {
                println!("Org:      {}", org);
            }
            if let Some(timezone) = body.get("timezone").and_then(|v| v.as_str()) {
                println!("Timezone: {}", timezone);
            }
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_command_parses_info() {
        let cmd = ip_command();
        let m = cmd.try_get_matches_from(["ip", "info"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("info"));
    }

    #[test]
    fn test_ip_requires_subcommand() {
        let cmd = ip_command();
        assert!(cmd.try_get_matches_from(["ip"]).is_err());
    }

    #[test]
    fn test_ip_meta() {
        let meta = ip_meta();
        assert_eq!(meta.name, "ip");
        assert_eq!(meta.category, CommandCategory::Utility);
    }
}
