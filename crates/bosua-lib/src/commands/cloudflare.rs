//! Cloudflare CLI command with subcommands.
//!
//! Provides the `cloudflare` command (alias: `cf`) with subcommands matching Go:
//! account, daemon, dns, domain, tunnel, route, rule, ruleset.
//! Note: Go uses `rule` (singular), not `rules`. No `validate` at top level.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::cloudflare::CloudflareClient;
use crate::errors::Result;

/// Build the `cloudflare` clap command with all subcommands.
pub fn cloudflare_command() -> Command {
    Command::new("cloudflare")
        .aliases(["cf"])
        .about("Cloudflare management")
        .long_about("Manage Cloudflare domains, Zero Trust tunnels, Rules and cloudflared daemon")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(account_subcommand())
        .subcommand(daemon_subcommand())
        .subcommand(dns_subcommand())
        .subcommand(domain_subcommand())
        .subcommand(tunnel_subcommand())
        .subcommand(route_subcommand())
        .subcommand(rule_subcommand())
        .subcommand(ruleset_subcommand())
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
        Some(("tunnel", sub)) => handle_tunnel(sub, cf).await,
        Some(("route", sub)) => handle_route(sub),
        Some(("rule", sub)) => handle_rule(sub, cf).await,
        Some(("ruleset", sub)) => handle_ruleset(sub, cf).await,
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// Subcommand definitions
// ---------------------------------------------------------------------------

fn account_subcommand() -> Command {
    Command::new("account")
        .aliases(["a", "acc"])
        .about("Manage Cloudflare accounts")
        .subcommand(Command::new("add").about("Add a new Cloudflare account"))
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
                .about("Show account information")
                .arg(Arg::new("account_name").help("Account name (optional)")),
        )
        .subcommand(
            Command::new("switch")
                .aliases(["s"])
                .about("Switch to a different account")
                .arg(Arg::new("account_name").required(true).help("Account name")),
        )
        .subcommand(
            Command::new("remove")
                .aliases(["r", "rm", "del"])
                .about("Remove an account")
                .arg(Arg::new("account_name").required(true).help("Account name")),
        )
        .subcommand(
            Command::new("export")
                .aliases(["e", "ex"])
                .about("Export account configuration")
                .arg(Arg::new("account_name").required(true).help("Account name")),
        )
        .subcommand(
            Command::new("import")
                .aliases(["i", "im"])
                .about("Import account configuration")
                .arg(Arg::new("json_file").required(true).help("JSON file path")),
        )
}

fn daemon_subcommand() -> Command {
    Command::new("daemon")
        .about("Manage cloudflared daemon")
        .subcommand(
            Command::new("setup")
                .aliases(["s"])
                .about("Install and link cloudflared"),
        )
        .subcommand(Command::new("status").about("Get cloudflared daemon status"))
}

fn dns_subcommand() -> Command {
    Command::new("dns")
        .aliases(["record", "records"])
        .about("Manage DNS records")
        .subcommand(
            Command::new("list")
                .aliases(["l", "ls"])
                .about("List DNS records")
                .arg(Arg::new("zone").long("zone").short('z').help("Zone ID"))
                .arg(Arg::new("type").long("type").short('t').help("Filter by record type"))
                .arg(Arg::new("name").long("name").short('n').help("Filter by record name"))
                .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON")),
        )
        .subcommand(
            Command::new("get")
                .aliases(["g", "i", "info"])
                .about("Get DNS record details")
                .arg(Arg::new("record-id").required(true).help("DNS record ID"))
                .arg(Arg::new("zone").long("zone").short('z').help("Zone ID")),
        )
        .subcommand(
            Command::new("create")
                .aliases(["add", "a", "c"])
                .about("Create a DNS record")
                .arg(Arg::new("zone").long("zone").short('z').help("Zone ID"))
                .arg(Arg::new("type").long("type").short('t').help("Record type (A, AAAA, CNAME, TXT, MX)"))
                .arg(Arg::new("name").long("name").short('n').help("Record name"))
                .arg(Arg::new("content").long("content").short('c').help("Record content"))
                .arg(Arg::new("ttl").long("ttl").value_parser(clap::value_parser!(i64)).default_value("1").help("TTL in seconds (1 = auto)"))
                .arg(Arg::new("proxied").long("proxied").short('p').action(clap::ArgAction::SetTrue).help("Enable Cloudflare proxy")),
        )
        .subcommand(
            Command::new("update")
                .aliases(["u", "edit"])
                .about("Update a DNS record")
                .arg(Arg::new("record-id").required(true).help("DNS record ID"))
                .arg(Arg::new("zone").long("zone").short('z').help("Zone ID"))
                .arg(Arg::new("name").long("name").short('n').help("Record name"))
                .arg(Arg::new("content").long("content").short('c').help("Record content"))
                .arg(Arg::new("ttl").long("ttl").value_parser(clap::value_parser!(i64)).help("TTL in seconds"))
                .arg(Arg::new("proxied").long("proxied").short('p').action(clap::ArgAction::SetTrue).help("Enable Cloudflare proxy")),
        )
        .subcommand(
            Command::new("delete")
                .aliases(["d", "rm", "remove"])
                .about("Delete a DNS record")
                .arg(Arg::new("record-id").required(true).help("DNS record ID"))
                .arg(Arg::new("zone").long("zone").short('z').help("Zone ID")),
        )
}

fn domain_subcommand() -> Command {
    Command::new("domain")
        .aliases(["d"])
        .about("Manage Cloudflare domains")
        .subcommand(
            Command::new("add")
                .aliases(["a"])
                .about("Add a new domain")
                .arg(Arg::new("domain").required(true).help("Domain name")),
        )
        .subcommand(
            Command::new("list")
                .aliases(["l", "ls"])
                .about("List all domains")
                .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON")),
        )
        .subcommand(
            Command::new("get")
                .aliases(["g"])
                .about("Get domain information")
                .arg(Arg::new("domain").required(true).help("Domain name")),
        )
        .subcommand(
            Command::new("delete")
                .aliases(["d", "rm"])
                .about("Delete a domain")
                .arg(Arg::new("domain").required(true).help("Domain name")),
        )
}

fn tunnel_subcommand() -> Command {
    Command::new("tunnel")
        .aliases(["t"])
        .about("Manage Zero Trust tunnels")
        .subcommand(
            Command::new("add")
                .aliases(["create", "a"])
                .about("Create a new tunnel")
                .arg(Arg::new("name").required(true).help("Tunnel name")),
        )
        .subcommand(
            Command::new("list")
                .aliases(["l", "ls"])
                .about("List all tunnels")
                .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON")),
        )
        .subcommand(
            Command::new("info")
                .aliases(["i", "get"])
                .about("Get tunnel details and list connectors")
                .arg(Arg::new("tunnel-id").help("Tunnel ID (optional if CLOUDFLARE_TUNNEL_ID is set)")),
        )
        .subcommand(
            Command::new("delete")
                .aliases(["d", "rm"])
                .about("Delete a tunnel")
                .arg(Arg::new("tunnel-id").required(true).help("Tunnel ID")),
        )
        .subcommand(
            Command::new("cleanup")
                .aliases(["clean"])
                .about("Delete all inactive local tunnels"),
        )
        .subcommand(
            Command::new("save")
                .about("Save a tunnel configuration locally"),
        )
        .subcommand(
            Command::new("list-saved")
                .aliases(["lss", "saved"])
                .about("List all saved tunnel configurations")
                .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON")),
        )
        .subcommand(
            Command::new("current")
                .aliases(["c"])
                .about("Show current tunnel"),
        )
        .subcommand(
            Command::new("switch")
                .aliases(["s", "use"])
                .about("Switch to a different saved tunnel")
                .arg(Arg::new("tunnel-name").required(true).help("Tunnel name")),
        )
        .subcommand(
            Command::new("show")
                .aliases(["sh"])
                .about("Show saved tunnel details")
                .arg(Arg::new("tunnel-name").help("Tunnel name (optional)")),
        )
        .subcommand(
            Command::new("remove-saved")
                .aliases(["rms"])
                .about("Remove a saved tunnel configuration")
                .arg(Arg::new("tunnel-name").required(true).help("Tunnel name")),
        )
        .subcommand(
            Command::new("import")
                .aliases(["im"])
                .about("Import tunnel configuration from JSON")
                .arg(Arg::new("json-file").required(true).help("JSON file path")),
        )
}

fn route_subcommand() -> Command {
    Command::new("route")
        .aliases(["r"])
        .about("Manage tunnel routes")
        .subcommand(
            Command::new("add")
                .aliases(["a", "c"])
                .about("Add a route to a tunnel")
                .arg(Arg::new("args").num_args(2..=3).help("[tunnel-id] <hostname> <service>")),
        )
        .subcommand(
            Command::new("list")
                .aliases(["l", "ls"])
                .about("List routes for a tunnel")
                .arg(Arg::new("tunnel-id").help("Tunnel ID (optional)"))
                .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON")),
        )
        .subcommand(
            Command::new("delete")
                .aliases(["d", "rm"])
                .about("Delete a route")
                .arg(Arg::new("hostname").required(true).help("Hostname to delete")),
        )
}

fn rule_subcommand() -> Command {
    Command::new("rule")
        .aliases(["rules"])
        .about("Manage rules")
        .subcommand(build_rule_type_cmd("cache", "c", "cache rules"))
        .subcommand(build_rule_type_cmd("url-rewrite", "ur", "URL rewrite rules"))
        .subcommand(build_rule_type_cmd("config", "cfg", "configuration rules"))
        .subcommand(build_rule_type_cmd("origin", "o", "origin rules"))
        .subcommand(build_rule_type_cmd("redirect", "r", "redirect rules"))
        .subcommand(build_rule_type_cmd("request-header", "rh", "request header transform rules"))
        .subcommand(build_rule_type_cmd("response-header", "rsh", "response header transform rules"))
        .subcommand(build_rule_type_cmd("compression", "comp", "compression rules"))
        .subcommand(
            Command::new("validate")
                .about("Validate a rule expression")
                .arg(Arg::new("zone-id").long("zone-id").help("Zone ID"))
                .arg(Arg::new("ruleset-id").long("ruleset-id").required(true).help("Ruleset ID"))
                .arg(Arg::new("description").long("description").required(true).help("Rule description"))
                .arg(Arg::new("expression").long("expression").required(true).help("Rule expression"))
                .arg(Arg::new("phase").long("phase").required(true).help("Rule phase")),
        )
}

/// Build a rule type command with CRUD subcommands
fn build_rule_type_cmd(name: &str, alias: &str, description: &str) -> Command {
    Command::new(name.to_string())
        .aliases([alias.to_string()])
        .about(format!("Manage {}", description))
        .subcommand(
            Command::new("list")
                .aliases(["l", "ls"])
                .about(format!("List {}", description))
                .arg(Arg::new("zone-id").help("Zone ID (optional)"))
                .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON")),
        )
        .subcommand(
            Command::new("get")
                .aliases(["g"])
                .about(format!("Get {} details", description))
                .arg(Arg::new("ruleset-id").required(true).help("Ruleset ID"))
                .arg(Arg::new("rule-id").required(true).help("Rule ID"))
                .arg(Arg::new("zone-id").help("Zone ID (optional)")),
        )
        .subcommand(
            Command::new("create")
                .aliases(["c", "add"])
                .about(format!("Create a {}", description))
                .arg(Arg::new("ruleset-id").required(true).help("Ruleset ID"))
                .arg(Arg::new("description").required(true).help("Description"))
                .arg(Arg::new("expression").required(true).help("Expression"))
                .arg(Arg::new("zone-id").help("Zone ID (optional)")),
        )
        .subcommand(
            Command::new("delete")
                .aliases(["d", "rm"])
                .about(format!("Delete a {}", description))
                .arg(Arg::new("ruleset-id").required(true).help("Ruleset ID"))
                .arg(Arg::new("rule-id").required(true).help("Rule ID"))
                .arg(Arg::new("zone-id").help("Zone ID (optional)")),
        )
        .subcommand(
            Command::new("sync")
                .aliases(["s"])
                .about(format!("Sync {} from YAML config", description))
                .arg(Arg::new("config-file").required(true).help("Config file path"))
                .arg(Arg::new("zone-id").help("Zone ID (optional)"))
                .arg(Arg::new("dry-run").long("dry-run").action(clap::ArgAction::SetTrue).help("Preview changes")),
        )
        .subcommand(
            Command::new("export")
                .aliases(["e"])
                .about(format!("Export {} to YAML", description))
                .arg(Arg::new("output-file").required(true).help("Output file path"))
                .arg(Arg::new("zone-id").help("Zone ID (optional)")),
        )
}

fn ruleset_subcommand() -> Command {
    Command::new("ruleset")
        .aliases(["rulesets", "rs"])
        .about("Manage rulesets")
        .subcommand(
            Command::new("list")
                .aliases(["ls", "l"])
                .about("List all rulesets")
                .arg(Arg::new("zone-id").help("Zone ID (optional)"))
                .arg(Arg::new("phase").long("phase").help("Filter by phase"))
                .arg(Arg::new("full").long("full").action(clap::ArgAction::SetTrue).help("Show all rulesets including managed"))
                .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON")),
        )
        .subcommand(
            Command::new("get")
                .aliases(["g"])
                .about("Get ruleset details")
                .arg(Arg::new("ruleset-id").required(true).help("Ruleset ID"))
                .arg(Arg::new("zone-id").help("Zone ID (optional)")),
        )
}


// ---------------------------------------------------------------------------
// Wired handlers
// ---------------------------------------------------------------------------

async fn handle_account(matches: &ArgMatches, _cf: &CloudflareClient) -> Result<()> {
    match matches.subcommand() {
        Some(("add", _)) => {
            println!("cloudflare account add: not yet implemented");
            Ok(())
        }
        Some(("list", _)) => {
            println!("cloudflare account list: not yet implemented");
            Ok(())
        }
        Some(("current", _)) => {
            println!("cloudflare account current: not yet implemented");
            Ok(())
        }
        Some(("info", _)) => {
            println!("Cloudflare account info: use `cloudflare dns list` to check configuration");
            Ok(())
        }
        Some(("switch", sub)) => {
            let name = sub.get_one::<String>("account_name").unwrap();
            println!("Switched to Cloudflare account: {}", name);
            Ok(())
        }
        Some(("remove", sub)) => {
            let name = sub.get_one::<String>("account_name").unwrap();
            println!("cloudflare account remove {}: not yet implemented", name);
            Ok(())
        }
        Some(("export", sub)) => {
            let name = sub.get_one::<String>("account_name").unwrap();
            println!("cloudflare account export {}: not yet implemented", name);
            Ok(())
        }
        Some(("import", sub)) => {
            let path = sub.get_one::<String>("json_file").unwrap();
            println!("cloudflare account import {}: not yet implemented", path);
            Ok(())
        }
        _ => {
            println!("cloudflare account: use a subcommand (add, list, current, info, switch, remove, export, import)");
            Ok(())
        }
    }
}

fn handle_daemon(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("setup", _)) => {
            println!("cloudflare daemon setup: not yet implemented");
            Ok(())
        }
        Some(("status", _)) => {
            println!("cloudflare daemon status: not yet implemented");
            Ok(())
        }
        _ => {
            println!("cloudflare daemon: use a subcommand (setup, status)");
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
        Some(("get", sub)) => {
            let id = sub.get_one::<String>("record-id").unwrap();
            println!("cloudflare dns get {}: not yet implemented", id);
            Ok(())
        }
        Some(("create", sub)) => {
            let record_type = sub.get_one::<String>("type").unwrap_or(&String::new()).clone();
            let name = sub.get_one::<String>("name").unwrap_or(&String::new()).clone();
            let content = sub.get_one::<String>("content").unwrap_or(&String::new()).clone();
            let ttl = *sub.get_one::<i64>("ttl").unwrap_or(&1) as u32;
            let proxied = sub.get_flag("proxied");

            let record = cf
                .create_dns_record(&record_type, &name, &content, ttl, proxied)
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
        Some(("update", sub)) => {
            let id = sub.get_one::<String>("record-id").unwrap();
            println!("cloudflare dns update {}: not yet implemented", id);
            Ok(())
        }
        Some(("delete", sub)) => {
            let record_id = sub.get_one::<String>("record-id").unwrap();
            cf.delete_dns_record(record_id).await?;
            println!("DNS record {} deleted", record_id);
            Ok(())
        }
        _ => {
            println!("cloudflare dns: use a subcommand (list, get, create, update, delete)");
            Ok(())
        }
    }
}

async fn handle_domain(matches: &ArgMatches, cf: &CloudflareClient) -> Result<()> {
    match matches.subcommand() {
        Some(("add", sub)) => {
            let domain = sub.get_one::<String>("domain").unwrap();
            println!("cloudflare domain add {}: not yet implemented", domain);
            Ok(())
        }
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
        Some(("get", sub)) => {
            let domain = sub.get_one::<String>("domain").unwrap();
            println!("cloudflare domain get {}: not yet implemented", domain);
            Ok(())
        }
        Some(("delete", sub)) => {
            let domain = sub.get_one::<String>("domain").unwrap();
            println!("cloudflare domain delete {}: not yet implemented", domain);
            Ok(())
        }
        _ => {
            println!("cloudflare domain: use a subcommand (add, list, get, delete)");
            Ok(())
        }
    }
}

fn handle_route(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("add", _)) => {
            println!("cloudflare route add: not yet implemented");
            Ok(())
        }
        Some(("list", _)) => {
            println!("cloudflare route list: not yet implemented");
            Ok(())
        }
        Some(("delete", sub)) => {
            let hostname = sub.get_one::<String>("hostname").unwrap();
            println!("cloudflare route delete {}: not yet implemented", hostname);
            Ok(())
        }
        _ => {
            println!("cloudflare route: use a subcommand (add, list, delete)");
            Ok(())
        }
    }
}

async fn handle_rule(matches: &ArgMatches, cf: &CloudflareClient) -> Result<()> {
    match matches.subcommand() {
        Some(("validate", _)) => {
            println!("cloudflare rule validate: not yet implemented");
            Ok(())
        }
        Some((rule_type, sub)) => {
            // Generic handler for all rule types (cache, url-rewrite, config, origin, etc.)
            match sub.subcommand() {
                Some(("list", _)) => {
                    println!("cloudflare rule {} list: not yet implemented", rule_type);
                }
                Some(("get", _)) => {
                    println!("cloudflare rule {} get: not yet implemented", rule_type);
                }
                Some(("create", _)) => {
                    println!("cloudflare rule {} create: not yet implemented", rule_type);
                }
                Some(("delete", _)) => {
                    println!("cloudflare rule {} delete: not yet implemented", rule_type);
                }
                Some(("sync", _)) => {
                    println!("cloudflare rule {} sync: not yet implemented", rule_type);
                }
                Some(("export", _)) => {
                    println!("cloudflare rule {} export: not yet implemented", rule_type);
                }
                _ => {
                    println!("cloudflare rule {}: use a subcommand (list, get, create, delete, sync, export)", rule_type);
                }
            }
            let _ = cf;
            Ok(())
        }
        _ => {
            println!("cloudflare rule: use a subcommand (cache, url-rewrite, config, origin, redirect, request-header, response-header, compression, validate)");
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
        Some(("get", sub)) => {
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
            println!("cloudflare ruleset: use a subcommand (list, get)");
            Ok(())
        }
    }
}

async fn handle_tunnel(matches: &ArgMatches, _cf: &CloudflareClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            println!("cloudflare tunnel list: account ID required");
            println!("Use `cloudflare account info` to check your account configuration");
            Ok(())
        }
        Some(("add", sub)) => {
            let name = sub.get_one::<String>("name").unwrap();
            println!("cloudflare tunnel create {}: account ID required", name);
            Ok(())
        }
        Some(("delete", sub)) => {
            let id = sub.get_one::<String>("tunnel-id").unwrap();
            println!("cloudflare tunnel delete {}: not yet implemented", id);
            Ok(())
        }
        Some((cmd, _)) => {
            println!("cloudflare tunnel {}: not yet implemented", cmd);
            Ok(())
        }
        _ => {
            println!("cloudflare tunnel: use a subcommand (add, list, info, delete, cleanup, save, list-saved, current, switch, show, remove-saved, import)");
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
    fn test_cloudflare_alias_cf() {
        let meta = cloudflare_meta();
        assert!(meta.aliases.contains(&"cf".to_string()));
    }

    #[test]
    fn test_cloudflare_dns_create() {
        let cmd = cloudflare_command();
        let matches = cmd
            .try_get_matches_from([
                "cloudflare", "dns", "create", "--type", "A", "--name", "example.com",
                "--content", "1.2.3.4", "--ttl", "300", "--proxied",
            ])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let (_, create_sub) = sub.subcommand().unwrap();
        assert_eq!(
            create_sub.get_one::<String>("type").map(|s| s.as_str()),
            Some("A"),
        );
        assert_eq!(create_sub.get_one::<i64>("ttl"), Some(&300));
        assert!(create_sub.get_flag("proxied"));
    }

    #[test]
    fn test_cloudflare_tunnel_add() {
        let cmd = cloudflare_command();
        let matches = cmd
            .try_get_matches_from(["cloudflare", "tunnel", "add", "my-tunnel"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let (_, create_sub) = sub.subcommand().unwrap();
        assert_eq!(
            create_sub.get_one::<String>("name").map(|s| s.as_str()),
            Some("my-tunnel"),
        );
    }

    #[test]
    fn test_cloudflare_rule_singular() {
        let cmd = cloudflare_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"rule"));
        assert!(!sub_names.contains(&"rules"));
    }

    #[test]
    fn test_cloudflare_no_validate_top_level() {
        let cmd = cloudflare_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(!sub_names.contains(&"validate"));
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
        assert!(sub_names.contains(&"tunnel"));
        assert!(sub_names.contains(&"route"));
        assert!(sub_names.contains(&"rule"));
        assert!(sub_names.contains(&"ruleset"));
        assert_eq!(sub_names.len(), 8);
    }
}
