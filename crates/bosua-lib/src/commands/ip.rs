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
        .about("IP address utilities")
        .subcommand(Command::new("local").about("Show local IP address"))
        .subcommand(Command::new("public").about("Show public IP address"))
        .subcommand(Command::new("geo").about("Geolocate an IP address"))
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
        Some(("local", _)) => {
            // Use UDP socket trick to detect local IP (no actual data is sent)
            let socket = std::net::UdpSocket::bind("0.0.0.0:0")
                .map_err(|e| BosuaError::Command(format!("Failed to create socket: {}", e)))?;
            socket
                .connect("8.8.8.8:80")
                .map_err(|e| BosuaError::Command(format!("Failed to detect local IP: {}", e)))?;
            let local_addr = socket
                .local_addr()
                .map_err(|e| BosuaError::Command(format!("Failed to get local address: {}", e)))?;
            println!("{}", local_addr.ip());
            Ok(())
        }
        Some(("public", _)) => {
            let client = http.get_client().await;
            let resp = client
                .get("https://api.ipify.org")
                .send()
                .await
                .map_err(|e| BosuaError::Command(format!("Failed to query public IP: {}", e)))?;
            let ip = resp
                .text()
                .await
                .map_err(|e| BosuaError::Command(format!("Failed to read response: {}", e)))?;
            println!("{}", ip.trim());
            Ok(())
        }
        Some(("geo", _)) => {
            let client = http.get_client().await;
            let resp = client
                .get("https://ipinfo.io/json")
                .send()
                .await
                .map_err(|e| BosuaError::Command(format!("Failed to query geolocation: {}", e)))?;
            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| BosuaError::Command(format!("Failed to parse geolocation: {}", e)))?;

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
        _ => {
            println!("ip: use a subcommand (local, public, geo)");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_command_parses_local() {
        let cmd = ip_command();
        let matches = cmd.try_get_matches_from(["ip", "local"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("local"));
    }

    #[test]
    fn test_ip_command_parses_public() {
        let cmd = ip_command();
        let matches = cmd.try_get_matches_from(["ip", "public"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("public"));
    }

    #[test]
    fn test_ip_command_parses_geo() {
        let cmd = ip_command();
        let matches = cmd.try_get_matches_from(["ip", "geo"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("geo"));
    }

    #[test]
    fn test_ip_meta() {
        let meta = ip_meta();
        assert_eq!(meta.name, "ip");
        assert_eq!(meta.category, CommandCategory::Utility);
        assert!(!meta.description.is_empty());
    }
}
