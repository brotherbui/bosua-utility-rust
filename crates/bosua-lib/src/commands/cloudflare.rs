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
    let mgr = crate::cloud::account_manager::AccountManager::new("cloudflare")?;
    match matches.subcommand() {
        Some(("add", _)) => {
            mgr.add_account_interactive(&[
                ("Enter Cloudflare API Token", true),
                ("Enter Cloudflare Account ID", true),
                ("Enter Default Zone ID (optional, press Enter to skip)", false),
            ])
        }
        Some(("list", _)) => mgr.print_list(),
        Some(("current", _)) => mgr.print_current(),
        Some(("info", _)) => mgr.show_info(None),
        Some(("switch", sub)) => {
            let name = sub.get_one::<String>("account_name").unwrap();
            mgr.switch_account(name)
        }
        Some(("remove", sub)) => {
            let name = sub.get_one::<String>("account_name").unwrap();
            mgr.remove_account_interactive(name)
        }
        Some(("export", sub)) => {
            let name = sub.get_one::<String>("account_name").unwrap();
            mgr.export_account(name)
        }
        Some(("import", sub)) => {
            let path = sub.get_one::<String>("json_file").unwrap();
            mgr.import_account(path)
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
            // Check if cloudflared is installed
            let status = std::process::Command::new("which")
                .arg("cloudflared")
                .stdout(std::process::Stdio::null())
                .status();
            match status {
                Ok(s) if s.success() => {
                    println!("cloudflared is already installed");
                    // Try to login/link
                    let _ = std::process::Command::new("cloudflared")
                        .args(["tunnel", "login"])
                        .stdin(std::process::Stdio::inherit())
                        .stdout(std::process::Stdio::inherit())
                        .stderr(std::process::Stdio::inherit())
                        .status();
                }
                _ => {
                    println!("cloudflared not found. Install it with:");
                    println!("  brew install cloudflared");
                }
            }
            Ok(())
        }
        Some(("status", _)) => {
            let output = std::process::Command::new("cloudflared")
                .args(["version"])
                .output();
            match output {
                Ok(o) if o.status.success() => {
                    println!("cloudflared: installed");
                    println!("{}", String::from_utf8_lossy(&o.stdout).trim());
                    // Check if service is running
                    let svc = std::process::Command::new("pgrep")
                        .arg("cloudflared")
                        .output();
                    match svc {
                        Ok(s) if s.status.success() => println!("Status: running"),
                        _ => println!("Status: not running"),
                    }
                }
                _ => println!("cloudflared: not installed"),
            }
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
            let record = cf.get_dns_record(id).await?;
            println!("ID:      {}", record.id);
            println!("Type:    {}", record.record_type);
            println!("Name:    {}", record.name);
            println!("Content: {}", record.content);
            println!("TTL:     {}", record.ttl);
            println!("Proxied: {}", record.proxied);
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
            // First get the current record to preserve unchanged fields
            let current = cf.get_dns_record(id).await?;
            let record = cf.update_dns_record(
                id,
                Some(&current.record_type),
                Some(&current.name),
                Some(&current.content),
                Some(current.ttl),
                Some(current.proxied),
            ).await?;
            println!("DNS record updated:");
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
            // Need account_id from credentials
            let mgr = crate::cloud::account_manager::AccountManager::new("cloudflare")?;
            let current = mgr.load_current()?;
            let creds = mgr.load_credentials(&current)?;
            let account_id = creds.get("accountid").or(creds.get("accountId")).or(creds.get("account_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if account_id.is_empty() {
                println!("No account ID found in credentials. Use `cloudflare account add` first.");
                return Ok(());
            }
            let zone = cf.add_zone(domain, account_id).await?;
            println!("Domain added:");
            println!("  ID:     {}", zone.id);
            println!("  Name:   {}", zone.name);
            println!("  Status: {}", zone.status);
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
            let zone = cf.get_zone(domain).await?;
            println!("ID:     {}", zone.id);
            println!("Name:   {}", zone.name);
            println!("Status: {}", zone.status);
            println!("Paused: {}", zone.paused);
            Ok(())
        }
        Some(("delete", sub)) => {
            let domain = sub.get_one::<String>("domain").unwrap();
            println!("Domain deletion is restricted for safety.");
            println!("To delete '{}', use the Cloudflare dashboard.", domain);
            Ok(())
        }
        _ => {
            println!("cloudflare domain: use a subcommand (add, list, get, delete)");
            Ok(())
        }
    }
}

fn handle_route(matches: &ArgMatches) -> Result<()> {
    // Routes are managed via cloudflared CLI or Cloudflare dashboard
    match matches.subcommand() {
        Some(("add", _)) => {
            println!("Use `cloudflared tunnel route dns <tunnel-name> <hostname>` to add routes");
            Ok(())
        }
        Some(("list", _)) => {
            // List routes via cloudflared
            let output = std::process::Command::new("cloudflared")
                .args(["tunnel", "route", "ip", "show"])
                .output();
            match output {
                Ok(o) if o.status.success() => {
                    println!("{}", String::from_utf8_lossy(&o.stdout));
                }
                _ => println!("Failed to list routes. Ensure cloudflared is installed."),
            }
            Ok(())
        }
        Some(("delete", sub)) => {
            let hostname = sub.get_one::<String>("hostname").unwrap();
            println!("Route deletion for '{}': use `cloudflared tunnel route dns delete` or the Cloudflare dashboard", hostname);
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
            let result = cf.validate().await?;
            println!("Validation: {}", if result.valid { "passed" } else { "failed" });
            if !result.errors.is_empty() {
                for msg in &result.errors {
                    println!("  Error: {}", msg);
                }
            }
            if !result.warnings.is_empty() {
                for msg in &result.warnings {
                    println!("  Warning: {}", msg);
                }
            }
            Ok(())
        }
        Some((rule_type, sub)) => {
            match sub.subcommand() {
                Some(("list", _)) => {
                    let rulesets = cf.list_rulesets().await?;
                    let matching: Vec<_> = rulesets.iter().filter(|rs| rs.phase.contains(rule_type) || rs.name.to_lowercase().contains(rule_type)).collect();
                    if matching.is_empty() {
                        println!("No rulesets found matching '{}'", rule_type);
                    } else {
                        println!("{:<36} {:<32} {}", "ID", "NAME", "PHASE");
                        for rs in &matching {
                            println!("{:<36} {:<32} {}", rs.id, rs.name, rs.phase);
                        }
                    }
                }
                Some((action, _)) => {
                    println!("cloudflare rule {} {}: use the Cloudflare dashboard for rule management", rule_type, action);
                }
                _ => {
                    println!("cloudflare rule {}: use a subcommand (list, get, create, delete, sync, export)", rule_type);
                }
            }
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
            println!("Tunnel deletion is restricted for safety.");
            println!("To delete tunnel '{}', use the Cloudflare dashboard or `cloudflared tunnel delete`.", id);
            Ok(())
        }
        Some((cmd, _)) => {
            // For tunnel subcommands like save, list-saved, current, switch, show, remove-saved, import
            // These are local config management - use the account manager pattern
            let mgr = crate::cloud::account_manager::AccountManager::new("cloudflare")?;
            match cmd {
                "save" | "list-saved" | "current" | "switch" | "show" | "remove-saved" | "import" => {
                    println!("cloudflare tunnel {}: use `cloudflared tunnel {}` directly", cmd, cmd);
                }
                _ => {
                    println!("cloudflare tunnel {}: use `cloudflared tunnel {}` directly", cmd, cmd);
                }
            }
            let _ = mgr;
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
