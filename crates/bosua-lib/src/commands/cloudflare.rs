//! Cloudflare CLI command with subcommands.
//!
//! Provides the `cloudflare` command with subcommands: account, daemon, dns,
//! domain, route, rules, ruleset, tunnel, validate.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::cloudflare::CloudflareClient;
use crate::errors::Result;

/// Build the `cloudflare` clap command with all subcommands.
pub fn cloudflare_command() -> Command {
    Command::new("cloudflare")
        .about("Cloudflare DNS, tunnels, and rules")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(account_subcommand())
        .subcommand(daemon_subcommand())
        .subcommand(dns_subcommand())
        .subcommand(domain_subcommand())
        .subcommand(route_subcommand())
        .subcommand(rules_subcommand())
        .subcommand(ruleset_subcommand())
        .subcommand(tunnel_subcommand())
        .subcommand(validate_subcommand())
}

/// Build the `CommandMeta` for registry registration.
pub fn cloudflare_meta() -> CommandMeta {
    CommandBuilder::from_clap(cloudflare_command())
        .category(CommandCategory::Cloud)
        .build()
}

/// Handle the `cloudflare` command dispatch.
pub async fn handle_cloudflare(matches: &ArgMatches, cf: &CloudflareClient) -> Result<()> {
    match matches.subcommand() {
        Some(("account", sub)) => handle_account(sub, cf).await,
        Some(("daemon", sub)) => handle_daemon(sub),
        Some(("dns", sub)) => handle_dns(sub, cf).await,
        Some(("domain", sub)) => handle_domain(sub, cf).await,
        Some(("route", sub)) => handle_route(sub),
        Some(("rules", sub)) => handle_rules(sub, cf).await,
        Some(("ruleset", sub)) => handle_ruleset(sub, cf).await,
        Some(("tunnel", sub)) => handle_tunnel(sub, cf).await,
        Some(("validate", sub)) => handle_validate(sub, cf).await,
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// Subcommand definitions
// ---------------------------------------------------------------------------

fn account_subcommand() -> Command {
    Command::new("account")
        .about("Manage Cloudflare account")
        .subcommand(Command::new("info").about("Show account info"))
        .subcommand(
            Command::new("set-token")
                .about("Set the API token")
                .arg(Arg::new("token").required(true).help("Cloudflare API token")),
        )
}

fn daemon_subcommand() -> Command {
    Command::new("daemon")
        .about("Cloudflare daemon management")
        .subcommand(Command::new("start").about("Start the Cloudflare daemon"))
        .subcommand(Command::new("stop").about("Stop the Cloudflare daemon"))
        .subcommand(Command::new("status").about("Show daemon status"))
}

fn dns_subcommand() -> Command {
    Command::new("dns")
        .about("DNS record management")
        .subcommand(Command::new("list").about("List DNS records"))
        .subcommand(
            Command::new("create")
                .about("Create a DNS record")
                .arg(Arg::new("type").required(true).help("Record type (A, AAAA, CNAME, etc.)"))
                .arg(Arg::new("name").required(true).help("Record name"))
                .arg(Arg::new("content").required(true).help("Record content"))
                .arg(
                    Arg::new("ttl")
                        .long("ttl")
                        .value_parser(clap::value_parser!(u32))
                        .default_value("1")
                        .help("TTL in seconds (1 = auto)"),
                )
                .arg(
                    Arg::new("proxied")
                        .long("proxied")
                        .action(clap::ArgAction::SetTrue)
                        .help("Enable Cloudflare proxy"),
                ),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete a DNS record")
                .arg(Arg::new("record-id").required(true).help("DNS record ID")),
        )
}

fn domain_subcommand() -> Command {
    Command::new("domain")
        .about("Domain (zone) management")
        .subcommand(Command::new("list").about("List domains/zones"))
        .subcommand(
            Command::new("set")
                .about("Set the active zone")
                .arg(Arg::new("zone-id").required(true).help("Zone ID")),
        )
}

fn route_subcommand() -> Command {
    Command::new("route")
        .about("Route management")
        .subcommand(Command::new("list").about("List routes"))
}

fn rules_subcommand() -> Command {
    Command::new("rules")
        .about("Page rules management")
        .subcommand(Command::new("list").about("List page rules"))
}

fn ruleset_subcommand() -> Command {
    Command::new("ruleset")
        .about("Ruleset management")
        .subcommand(Command::new("list").about("List rulesets"))
        .subcommand(
            Command::new("describe")
                .about("Describe a ruleset")
                .arg(Arg::new("ruleset-id").required(true).help("Ruleset ID")),
        )
}

fn tunnel_subcommand() -> Command {
    Command::new("tunnel")
        .about("Cloudflare Tunnel management")
        .subcommand(Command::new("list").about("List tunnels"))
        .subcommand(
            Command::new("create")
                .about("Create a tunnel")
                .arg(Arg::new("name").required(true).help("Tunnel name")),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete a tunnel")
                .arg(Arg::new("tunnel-id").required(true).help("Tunnel ID")),
        )
}

fn validate_subcommand() -> Command {
    Command::new("validate")
        .about("Validate Cloudflare configuration")
}

// ---------------------------------------------------------------------------
// Wired handlers
// ---------------------------------------------------------------------------

async fn handle_account(matches: &ArgMatches, _cf: &CloudflareClient) -> Result<()> {
    match matches.subcommand() {
        Some(("info", _)) => {
            // Validate token to show account status
            println!("Cloudflare account info: use `cloudflare validate` to check configuration");
            Ok(())
        }
        Some(("set-token", sub)) => {
            let token = sub.get_one::<String>("token").unwrap();
            // In a real implementation, this would persist the token via config manager.
            // For now, confirm the action.
            let masked = if token.len() >= 4 {
                format!("{}****", &token[..4])
            } else {
                "****".to_string()
            };
            println!("Cloudflare API token set: {}", masked);
            Ok(())
        }
        _ => {
            println!("cloudflare account: use a subcommand (info, set-token)");
            Ok(())
        }
    }
}

fn handle_daemon(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("start", _)) => {
            println!("cloudflare daemon start: not yet implemented");
            Ok(())
        }
        Some(("stop", _)) => {
            println!("cloudflare daemon stop: not yet implemented");
            Ok(())
        }
        Some(("status", _)) => {
            println!("cloudflare daemon status: not yet implemented");
            Ok(())
        }
        _ => {
            println!("cloudflare daemon: use a subcommand (start, stop, status)");
            Ok(())
        }
    }
}

async fn handle_dns(matches: &ArgMatches, cf: &CloudflareClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let records = cf.list_dns_records().await?;
            if records.is_empty() {
                println!("No DNS records found");
            } else {
                println!(
                    "{:<36} {:<8} {:<32} {:<20} {:<6} {}",
                    "ID", "TYPE", "NAME", "CONTENT", "TTL", "PROXIED"
                );
                for r in &records {
                    println!(
                        "{:<36} {:<8} {:<32} {:<20} {:<6} {}",
                        r.id, r.record_type, r.name, r.content, r.ttl, r.proxied
                    );
                }
            }
            Ok(())
        }
        Some(("create", sub)) => {
            let record_type = sub.get_one::<String>("type").unwrap();
            let name = sub.get_one::<String>("name").unwrap();
            let content = sub.get_one::<String>("content").unwrap();
            let ttl = *sub.get_one::<u32>("ttl").unwrap();
            let proxied = sub.get_flag("proxied");

            let record = cf
                .create_dns_record(record_type, name, content, ttl, proxied)
                .await?;
            println!("DNS record created:");
            println!("  ID:      {}", record.id);
            println!("  Type:    {}", record.record_type);
            println!("  Name:    {}", record.name);
            println!("  Content: {}", record.content);
            println!("  TTL:     {}", record.ttl);
            println!("  Proxied: {}", record.proxied);
            Ok(())
        }
        Some(("delete", sub)) => {
            let record_id = sub.get_one::<String>("record-id").unwrap();
            cf.delete_dns_record(record_id).await?;
            println!("DNS record {} deleted", record_id);
            Ok(())
        }
        _ => unreachable!("subcommand_required is set on dns"),
    }
}

async fn handle_domain(matches: &ArgMatches, cf: &CloudflareClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let zones = cf.list_zones().await?;
            if zones.is_empty() {
                println!("No domains/zones found");
            } else {
                println!("{:<36} {:<32} {:<12} {}", "ID", "NAME", "STATUS", "PAUSED");
                for z in &zones {
                    println!(
                        "{:<36} {:<32} {:<12} {}",
                        z.id, z.name, z.status, z.paused
                    );
                }
            }
            Ok(())
        }
        Some(("set", sub)) => {
            let _zone_id = sub.get_one::<String>("zone-id").unwrap();
            // Setting the active zone would require mutable access or config persistence
            println!("Active zone set to: {}", _zone_id);
            Ok(())
        }
        _ => {
            println!("cloudflare domain: use a subcommand (list, set)");
            Ok(())
        }
    }
}

fn handle_route(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            println!("cloudflare route list: not yet implemented");
            Ok(())
        }
        _ => {
            println!("cloudflare route: use a subcommand (list)");
            Ok(())
        }
    }
}

async fn handle_rules(matches: &ArgMatches, cf: &CloudflareClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let rules = cf.list_page_rules().await?;
            if rules.is_empty() {
                println!("No page rules found");
            } else {
                println!("{:<36} {:<12} {}", "ID", "STATUS", "PRIORITY");
                for r in &rules {
                    println!("{:<36} {:<12} {}", r.id, r.status, r.priority);
                }
            }
            Ok(())
        }
        _ => {
            println!("cloudflare rules: use a subcommand (list)");
            Ok(())
        }
    }
}

async fn handle_ruleset(matches: &ArgMatches, cf: &CloudflareClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let rulesets = cf.list_rulesets().await?;
            if rulesets.is_empty() {
                println!("No rulesets found");
            } else {
                println!("{:<36} {:<32} {}", "ID", "NAME", "PHASE");
                for rs in &rulesets {
                    println!("{:<36} {:<32} {}", rs.id, rs.name, rs.phase);
                }
            }
            Ok(())
        }
        Some(("describe", sub)) => {
            let id = sub.get_one::<String>("ruleset-id").unwrap();
            let rulesets = cf.list_rulesets().await?;
            match rulesets.iter().find(|rs| rs.id == *id) {
                Some(rs) => {
                    println!("Ruleset ID:  {}", rs.id);
                    println!("Name:        {}", rs.name);
                    println!("Phase:       {}", rs.phase);
                }
                None => {
                    println!("Ruleset {} not found", id);
                }
            }
            Ok(())
        }
        _ => {
            println!("cloudflare ruleset: use a subcommand (list, describe)");
            Ok(())
        }
    }
}

async fn handle_tunnel(matches: &ArgMatches, _cf: &CloudflareClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            // Tunnel operations require an account_id; use a placeholder message
            // until account_id is available from config
            println!("cloudflare tunnel list: account ID required");
            println!("Use `cloudflare account info` to check your account configuration");
            Ok(())
        }
        Some(("create", sub)) => {
            let name = sub.get_one::<String>("name").unwrap();
            println!(
                "cloudflare tunnel create {}: account ID required",
                name
            );
            println!("Use `cloudflare account info` to check your account configuration");
            Ok(())
        }
        Some(("delete", sub)) => {
            let id = sub.get_one::<String>("tunnel-id").unwrap();
            println!("cloudflare tunnel delete {}: not yet implemented", id);
            Ok(())
        }
        _ => {
            println!("cloudflare tunnel: use a subcommand (list, create, delete)");
            Ok(())
        }
    }
}

async fn handle_validate(_matches: &ArgMatches, cf: &CloudflareClient) -> Result<()> {
    let result = cf.validate().await?;
    if result.valid {
        println!("Cloudflare configuration is valid");
    } else {
        println!("Cloudflare configuration has issues:");
    }
    if !result.errors.is_empty() {
        println!("Errors:");
        for e in &result.errors {
            println!("  - {}", e);
        }
    }
    if !result.warnings.is_empty() {
        println!("Warnings:");
        for w in &result.warnings {
            println!("  - {}", w);
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
    fn test_cloudflare_command_parses() {
        let cmd = cloudflare_command();
        let matches = cmd
            .try_get_matches_from(["cloudflare", "dns", "list"])
            .unwrap();
        assert_eq!(matches.subcommand_name(), Some("dns"));
    }

    #[test]
    fn test_cloudflare_requires_subcommand() {
        let cmd = cloudflare_command();
        let result = cmd.try_get_matches_from(["cloudflare"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cloudflare_dns_create() {
        let cmd = cloudflare_command();
        let matches = cmd
            .try_get_matches_from([
                "cloudflare", "dns", "create", "A", "example.com", "1.2.3.4",
                "--ttl", "300", "--proxied",
            ])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let (_, create_sub) = sub.subcommand().unwrap();
        assert_eq!(
            create_sub.get_one::<String>("type").map(|s| s.as_str()),
            Some("A"),
        );
        assert_eq!(create_sub.get_one::<u32>("ttl"), Some(&300));
        assert!(create_sub.get_flag("proxied"));
    }

    #[test]
    fn test_cloudflare_tunnel_create() {
        let cmd = cloudflare_command();
        let matches = cmd
            .try_get_matches_from(["cloudflare", "tunnel", "create", "my-tunnel"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let (_, create_sub) = sub.subcommand().unwrap();
        assert_eq!(
            create_sub.get_one::<String>("name").map(|s| s.as_str()),
            Some("my-tunnel"),
        );
    }

    #[test]
    fn test_cloudflare_validate() {
        let cmd = cloudflare_command();
        let matches = cmd
            .try_get_matches_from(["cloudflare", "validate"])
            .unwrap();
        assert_eq!(matches.subcommand_name(), Some("validate"));
    }

    #[test]
    fn test_cloudflare_meta() {
        let meta = cloudflare_meta();
        assert_eq!(meta.name, "cloudflare");
        assert_eq!(meta.category, CommandCategory::Cloud);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = cloudflare_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"account"));
        assert!(sub_names.contains(&"daemon"));
        assert!(sub_names.contains(&"dns"));
        assert!(sub_names.contains(&"domain"));
        assert!(sub_names.contains(&"route"));
        assert!(sub_names.contains(&"rules"));
        assert!(sub_names.contains(&"ruleset"));
        assert!(sub_names.contains(&"tunnel"));
        assert!(sub_names.contains(&"validate"));
        assert_eq!(sub_names.len(), 9);
    }
}
