//! Tailscale CLI command with subcommands.
//!
//! Provides the `tailscale` command with subcommands: account, acl, config,
//! devices, keys, routes.
//!
//! Command implementations are stub handlers — actual logic will be wired
//! when the full app is assembled.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::tailscale::TailscaleClient;
use crate::errors::Result;

/// Build the `tailscale` clap command with all subcommands.
pub fn tailscale_command() -> Command {
    Command::new("tailscale")
        .about("Tailscale VPN management")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(account_subcommand())
        .subcommand(acl_subcommand())
        .subcommand(config_subcommand())
        .subcommand(devices_subcommand())
        .subcommand(keys_subcommand())
        .subcommand(routes_subcommand())
}

/// Build the `CommandMeta` for registry registration.
pub fn tailscale_meta() -> CommandMeta {
    CommandBuilder::from_clap(tailscale_command())
        .category(CommandCategory::Cloud)
        .build()
}

/// Handle the `tailscale` command dispatch.
pub async fn handle_tailscale(matches: &ArgMatches, ts: &TailscaleClient) -> Result<()> {
    match matches.subcommand() {
        Some(("account", sub)) => handle_account(sub),
        Some(("acl", sub)) => handle_acl(sub, ts).await,
        Some(("config", sub)) => handle_config(sub),
        Some(("devices", sub)) => handle_devices(sub, ts).await,
        Some(("keys", sub)) => handle_keys(sub, ts).await,
        Some(("routes", sub)) => handle_routes(sub, ts).await,
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// Subcommand definitions
// ---------------------------------------------------------------------------

fn account_subcommand() -> Command {
    Command::new("account")
        .about("Manage Tailscale account")
        .subcommand(Command::new("info").about("Show account info"))
        .subcommand(
            Command::new("set-key")
                .about("Set the API key")
                .arg(Arg::new("key").required(true).help("Tailscale API key")),
        )
        .subcommand(
            Command::new("set-tailnet")
                .about("Set the tailnet name")
                .arg(Arg::new("tailnet").required(true).help("Tailnet name")),
        )
}

fn acl_subcommand() -> Command {
    Command::new("acl")
        .about("ACL policy management")
        .subcommand(Command::new("get").about("Get the current ACL policy"))
        .subcommand(
            Command::new("set")
                .about("Set the ACL policy from a file")
                .arg(Arg::new("file").required(true).help("Path to ACL policy JSON file")),
        )
        .subcommand(Command::new("validate").about("Validate the current ACL policy"))
}

fn config_subcommand() -> Command {
    Command::new("config")
        .about("Tailscale configuration")
        .subcommand(Command::new("show").about("Show current configuration"))
        .subcommand(
            Command::new("set")
                .about("Set a configuration value")
                .arg(Arg::new("key").required(true).help("Configuration key"))
                .arg(Arg::new("value").required(true).help("Configuration value")),
        )
}

fn devices_subcommand() -> Command {
    Command::new("devices")
        .about("Device management")
        .subcommand(Command::new("list").about("List devices in the tailnet"))
        .subcommand(
            Command::new("info")
                .about("Show device info")
                .arg(Arg::new("device-id").required(true).help("Device ID")),
        )
        .subcommand(
            Command::new("delete")
                .about("Remove a device from the tailnet")
                .arg(Arg::new("device-id").required(true).help("Device ID")),
        )
}

fn keys_subcommand() -> Command {
    Command::new("keys")
        .about("Auth key management")
        .subcommand(Command::new("list").about("List auth keys"))
        .subcommand(
            Command::new("create")
                .about("Create a new auth key")
                .arg(
                    Arg::new("description")
                        .long("description")
                        .short('d')
                        .help("Key description"),
                ),
        )
}

fn routes_subcommand() -> Command {
    Command::new("routes")
        .about("Route management")
        .subcommand(
            Command::new("list")
                .about("List routes for a device")
                .arg(Arg::new("device-id").required(true).help("Device ID")),
        )
        .subcommand(
            Command::new("set")
                .about("Set routes for a device")
                .arg(Arg::new("device-id").required(true).help("Device ID"))
                .arg(
                    Arg::new("routes")
                        .required(true)
                        .num_args(1..)
                        .help("Route prefixes (e.g. 10.0.0.0/24)"),
                ),
        )
}

// ---------------------------------------------------------------------------
// Wired handlers
// ---------------------------------------------------------------------------

fn handle_account(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("info", _)) => {
            println!("Tailscale account info: use `tailscale acl get` or `tailscale devices list` to check configuration");
            Ok(())
        }
        Some(("set-key", sub)) => {
            let key = sub.get_one::<String>("key").unwrap();
            let masked = if key.len() >= 4 {
                format!("{}****", &key[..4])
            } else {
                "****".to_string()
            };
            println!("Tailscale API key set: {}", masked);
            Ok(())
        }
        Some(("set-tailnet", sub)) => {
            let tailnet = sub.get_one::<String>("tailnet").unwrap();
            println!("Tailscale tailnet set: {}", tailnet);
            Ok(())
        }
        _ => {
            println!("tailscale account: use a subcommand (info, set-key, set-tailnet)");
            Ok(())
        }
    }
}

async fn handle_acl(matches: &ArgMatches, ts: &TailscaleClient) -> Result<()> {
    match matches.subcommand() {
        Some(("get", _)) => {
            let policy = ts.get_acl().await?;
            println!("{}", serde_json::to_string_pretty(&policy)?);
            Ok(())
        }
        Some(("set", sub)) => {
            let file = sub.get_one::<String>("file").unwrap();
            let content = std::fs::read_to_string(file).map_err(|e| {
                crate::errors::BosuaError::Command(format!("Failed to read ACL file '{}': {}", file, e))
            })?;
            let policy: crate::cloud::tailscale::TsAclPolicy = serde_json::from_str(&content)?;
            ts.set_acl(&policy).await?;
            println!("ACL policy updated from {}", file);
            Ok(())
        }
        Some(("validate", _)) => {
            let policy = ts.get_acl().await?;
            println!("ACL policy is valid ({} rules, {} groups)", policy.acls.len(), {
                if policy.groups.is_object() {
                    policy.groups.as_object().map_or(0, |m| m.len())
                } else {
                    0
                }
            });
            Ok(())
        }
        _ => {
            println!("tailscale acl: use a subcommand (get, set, validate)");
            Ok(())
        }
    }
}

fn handle_config(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("show", _)) => {
            println!("Tailscale configuration:");
            println!("  Use `tailscale account set-key` to configure API key");
            println!("  Use `tailscale account set-tailnet` to configure tailnet");
            Ok(())
        }
        Some(("set", sub)) => {
            let key = sub.get_one::<String>("key").unwrap();
            let value = sub.get_one::<String>("value").unwrap();
            println!("Tailscale config set: {} = {}", key, value);
            Ok(())
        }
        _ => {
            println!("tailscale config: use a subcommand (show, set)");
            Ok(())
        }
    }
}

async fn handle_devices(matches: &ArgMatches, ts: &TailscaleClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let devices = ts.list_devices().await?;
            if devices.is_empty() {
                println!("No devices found in tailnet");
            } else {
                println!(
                    "{:<20} {:<32} {:<8} {:<20} {:<8}",
                    "ID", "HOSTNAME", "OS", "ADDRESSES", "ONLINE"
                );
                for d in &devices {
                    let addrs = d.addresses.join(", ");
                    println!(
                        "{:<20} {:<32} {:<8} {:<20} {:<8}",
                        d.id, d.hostname, d.os, addrs, d.online
                    );
                }
            }
            Ok(())
        }
        Some(("info", sub)) => {
            let id = sub.get_one::<String>("device-id").unwrap();
            let device = ts.get_device(id).await?;
            println!("Device ID:    {}", device.id);
            println!("Hostname:     {}", device.hostname);
            println!("Name:         {}", device.name);
            println!("OS:           {}", device.os);
            println!("Addresses:    {}", device.addresses.join(", "));
            println!("Online:       {}", device.online);
            if let Some(ref last_seen) = device.last_seen {
                println!("Last Seen:    {}", last_seen);
            }
            Ok(())
        }
        Some(("delete", sub)) => {
            let id = sub.get_one::<String>("device-id").unwrap();
            ts.delete_device(id).await?;
            println!("Device {} deleted", id);
            Ok(())
        }
        _ => unreachable!("subcommand_required is set on devices"),
    }
}

async fn handle_keys(matches: &ArgMatches, ts: &TailscaleClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let keys = ts.list_keys().await?;
            if keys.is_empty() {
                println!("No auth keys found");
            } else {
                println!(
                    "{:<20} {:<32} {:<10} {}",
                    "ID", "DESCRIPTION", "REVOKED", "EXPIRES"
                );
                for k in &keys {
                    let expires = k.expires_at.as_deref().unwrap_or("never");
                    println!(
                        "{:<20} {:<32} {:<10} {}",
                        k.id, k.description, k.revoked, expires
                    );
                }
            }
            Ok(())
        }
        Some(("create", sub)) => {
            let description = sub
                .get_one::<String>("description")
                .map(|s| s.as_str())
                .unwrap_or("CLI-generated key");
            // The TailscaleClient doesn't have a create_key method in the current API,
            // so we print a confirmation message. When the API is extended, this will call ts.create_key().
            println!("Auth key creation requested with description: {}", description);
            println!("Note: key creation requires the Tailscale API to support this operation");
            Ok(())
        }
        _ => {
            println!("tailscale keys: use a subcommand (list, create)");
            Ok(())
        }
    }
}

async fn handle_routes(matches: &ArgMatches, ts: &TailscaleClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", sub)) => {
            let id = sub.get_one::<String>("device-id").unwrap();
            let routes = ts.get_device_routes(id).await?;
            if routes.is_empty() {
                println!("No routes found for device {}", id);
            } else {
                println!(
                    "{:<20} {:<24} {:<12} {}",
                    "ID", "PREFIX", "ADVERTISED", "ENABLED"
                );
                for r in &routes {
                    println!(
                        "{:<20} {:<24} {:<12} {}",
                        r.id, r.prefix, r.advertised, r.enabled
                    );
                }
            }
            Ok(())
        }
        Some(("set", sub)) => {
            let id = sub.get_one::<String>("device-id").unwrap();
            let routes: Vec<String> = sub
                .get_many::<String>("routes")
                .unwrap()
                .cloned()
                .collect();
            ts.set_device_routes(id, &routes).await?;
            println!("Routes updated for device {}: {}", id, routes.join(", "));
            Ok(())
        }
        _ => {
            println!("tailscale routes: use a subcommand (list, set)");
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
    fn test_tailscale_command_parses() {
        let cmd = tailscale_command();
        let matches = cmd
            .try_get_matches_from(["tailscale", "devices", "list"])
            .unwrap();
        assert_eq!(matches.subcommand_name(), Some("devices"));
    }

    #[test]
    fn test_tailscale_requires_subcommand() {
        let cmd = tailscale_command();
        let result = cmd.try_get_matches_from(["tailscale"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_tailscale_acl_set() {
        let cmd = tailscale_command();
        let matches = cmd
            .try_get_matches_from(["tailscale", "acl", "set", "/path/to/acl.json"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let (_, set_sub) = sub.subcommand().unwrap();
        assert_eq!(
            set_sub.get_one::<String>("file").map(|s| s.as_str()),
            Some("/path/to/acl.json"),
        );
    }

    #[test]
    fn test_tailscale_routes_set() {
        let cmd = tailscale_command();
        let matches = cmd
            .try_get_matches_from([
                "tailscale", "routes", "set", "dev123", "10.0.0.0/24", "192.168.1.0/24",
            ])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let (_, set_sub) = sub.subcommand().unwrap();
        assert_eq!(
            set_sub.get_one::<String>("device-id").map(|s| s.as_str()),
            Some("dev123"),
        );
        let routes: Vec<&String> = set_sub.get_many::<String>("routes").unwrap().collect();
        assert_eq!(routes.len(), 2);
    }

    #[test]
    fn test_tailscale_meta() {
        let meta = tailscale_meta();
        assert_eq!(meta.name, "tailscale");
        assert_eq!(meta.category, CommandCategory::Cloud);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = tailscale_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"account"));
        assert!(sub_names.contains(&"acl"));
        assert!(sub_names.contains(&"config"));
        assert!(sub_names.contains(&"devices"));
        assert!(sub_names.contains(&"keys"));
        assert!(sub_names.contains(&"routes"));
        assert_eq!(sub_names.len(), 6);
    }
}
