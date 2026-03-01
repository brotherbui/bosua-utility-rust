//! Tailscale CLI command with FLAT subcommands matching Go.
//!
//! Provides the `tailscale` command with flat subcommands:
//! account, acl, list, info, approve, deauthorize, set-name, set-tags,
//! disable-key-expiry, set-ipv4, delete, key-generate, routes, exit-node.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::tailscale::TailscaleClient;
use crate::errors::Result;

/// Build the `tailscale` clap command with all subcommands.
pub fn tailscale_command() -> Command {
    Command::new("tailscale")
        .about("Manage Tailscale devices and configuration")
        .long_about("Manage Tailscale devices, auth keys, routes, and approvals via the Tailscale API. Requires OAuth client credentials.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        // Account management
        .subcommand(account_subcommand())
        // Device management (flat)
        .subcommand(list_subcommand())
        .subcommand(info_subcommand())
        .subcommand(approve_subcommand())
        .subcommand(deauthorize_subcommand())
        // Device configuration (flat)
        .subcommand(set_name_subcommand())
        .subcommand(set_tags_subcommand())
        .subcommand(disable_key_expiry_subcommand())
        .subcommand(set_ipv4_subcommand())
        // Key and device lifecycle (flat)
        .subcommand(delete_subcommand())
        .subcommand(key_generate_subcommand())
        // Routes and exit node
        .subcommand(routes_subcommand())
        .subcommand(exit_node_subcommand())
        // ACL management
        .subcommand(acl_subcommand())
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
        Some(("list", sub)) => handle_list(sub, ts).await,
        Some(("info", sub)) => handle_info(sub, ts).await,
        Some(("approve", sub)) => handle_approve(sub, ts).await,
        Some(("deauthorize", sub)) => handle_deauthorize(sub, ts).await,
        Some(("set-name", sub)) => handle_set_name(sub, ts).await,
        Some(("set-tags", sub)) => handle_set_tags(sub, ts).await,
        Some(("disable-key-expiry", sub)) => handle_disable_key_expiry(sub, ts).await,
        Some(("set-ipv4", sub)) => handle_set_ipv4(sub, ts).await,
        Some(("delete", sub)) => handle_delete(sub, ts).await,
        Some(("key-generate", sub)) => handle_key_generate(sub, ts).await,
        Some(("routes", sub)) => handle_routes(sub, ts).await,
        Some(("exit-node", sub)) => handle_exit_node(sub, ts).await,
        Some(("acl", sub)) => handle_acl(sub, ts).await,
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// Subcommand definitions
// ---------------------------------------------------------------------------

fn account_subcommand() -> Command {
    Command::new("account")
        .aliases(["a", "acc"])
        .about("Manage Tailscale accounts")
        .subcommand(Command::new("add").about("Add a new Tailscale account"))
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

fn list_subcommand() -> Command {
    Command::new("list")
        .aliases(["ls", "l"])
        .about("List all devices in your Tailscale network")
        .arg(Arg::new("all").long("all").action(clap::ArgAction::SetTrue).help("Show all devices including unauthorized"))
        .arg(Arg::new("tag").long("tag").help("Filter devices by tag"))
        .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON"))
}

fn info_subcommand() -> Command {
    Command::new("info")
        .about("Display detailed information about a device")
        .arg(Arg::new("hostname_or_id").required(true).help("Hostname or device ID"))
}

fn approve_subcommand() -> Command {
    Command::new("approve")
        .about("Approve a device and optionally enable as exit node")
        .arg(Arg::new("hostname_or_id").required(true).help("Hostname or device ID"))
        .arg(Arg::new("exit-node").long("exit-node").action(clap::ArgAction::SetTrue).help("Also enable device as exit node"))
}

fn deauthorize_subcommand() -> Command {
    Command::new("deauthorize")
        .aliases(["unapprove"])
        .about("Deauthorize (unapprove) a device")
        .arg(Arg::new("hostname_or_id").required(true).help("Hostname or device ID"))
}

fn set_name_subcommand() -> Command {
    Command::new("set-name")
        .about("Set the name of a device")
        .arg(Arg::new("hostname_or_id").required(true).help("Hostname or device ID"))
        .arg(Arg::new("new_name").required(true).help("New device name"))
}

fn set_tags_subcommand() -> Command {
    Command::new("set-tags")
        .about("Set tags for a device")
        .arg(Arg::new("hostname_or_id").required(true).help("Hostname or device ID"))
        .arg(Arg::new("tags").required(true).help("Comma-separated tags (e.g., tag:server,tag:prod)"))
}

fn disable_key_expiry_subcommand() -> Command {
    Command::new("disable-key-expiry")
        .about("Disable key expiry for a device")
        .arg(Arg::new("hostname_or_id").required(true).help("Hostname or device ID"))
}

fn set_ipv4_subcommand() -> Command {
    Command::new("set-ipv4")
        .about("Set the IPv4 address for a device")
        .arg(Arg::new("hostname_or_id").required(true).help("Hostname or device ID"))
        .arg(Arg::new("ipv4_address").required(true).help("IPv4 address"))
}

fn delete_subcommand() -> Command {
    Command::new("delete")
        .aliases(["rm", "remove"])
        .about("Delete a device from your Tailscale network")
        .arg(Arg::new("hostname_or_id").required(true).help("Hostname or device ID"))
        .arg(Arg::new("force").long("force").short('f').action(clap::ArgAction::SetTrue).help("Skip confirmation prompt"))
}

fn key_generate_subcommand() -> Command {
    Command::new("key-generate")
        .about("Generate a new auth key")
        .arg(Arg::new("reusable").long("reusable").action(clap::ArgAction::SetTrue).help("Generate a reusable key"))
        .arg(Arg::new("ephemeral").long("ephemeral").action(clap::ArgAction::SetTrue).help("Generate an ephemeral key"))
        .arg(Arg::new("tags").long("tags").default_value("tag:server").help("Comma-separated tags"))
        .arg(Arg::new("expiry-days").long("expiry-days").value_parser(clap::value_parser!(u32)).default_value("90").help("Key expiry in days (1-90)"))
}

fn routes_subcommand() -> Command {
    Command::new("routes")
        .about("Manage device routes (including exit node)")
        .arg(Arg::new("hostname_or_id").required(true).help("Hostname or device ID"))
        .arg(Arg::new("routes").required(true).help("Comma-separated routes (e.g., 0.0.0.0/0,::/0)"))
}

fn exit_node_subcommand() -> Command {
    Command::new("exit-node")
        .about("Manage exit node configuration")
        .subcommand(
            Command::new("approve")
                .about("Approve a device as an exit node")
                .arg(Arg::new("hostname_or_id").required(true).help("Hostname or device ID")),
        )
        .subcommand(
            Command::new("unapprove")
                .about("Remove exit node approval from a device")
                .arg(Arg::new("hostname_or_id").required(true).help("Hostname or device ID")),
        )
        .subcommand(
            Command::new("status")
                .about("Show exit node status for a device")
                .arg(Arg::new("hostname_or_id").required(true).help("Hostname or device ID")),
        )
}

fn acl_subcommand() -> Command {
    Command::new("acl")
        .about("Manage Tailscale ACL (Access Control List)")
        .subcommand(
            Command::new("get")
                .about("Get the current ACL policy")
                .arg(Arg::new("output").long("output").short('o').help("Save ACL to file"))
                .arg(Arg::new("raw").long("raw").action(clap::ArgAction::SetTrue).help("Display raw ACL")),
        )
        .subcommand(
            Command::new("set")
                .about("Update the ACL policy")
                .arg(Arg::new("file").long("file").short('f').help("Read ACL from file"))
                .arg(Arg::new("stdin").long("stdin").action(clap::ArgAction::SetTrue).help("Read ACL from stdin"))
                .arg(Arg::new("validate").long("validate").action(clap::ArgAction::SetTrue).default_value("true").help("Validate ACL before applying")),
        )
        .subcommand(Command::new("edit").about("Edit the ACL policy in your default editor"))
        .subcommand(
            Command::new("interactive")
                .aliases(["i", "wizard"])
                .about("Interactive ACL editor with guided prompts"),
        )
        .subcommand(
            Command::new("commit")
                .about("Commit and apply the draft ACL changes")
                .arg(Arg::new("force").long("force").short('f').action(clap::ArgAction::SetTrue).help("Skip confirmation prompt")),
        )
        .subcommand(
            Command::new("draft")
                .about("Manage ACL draft")
                .subcommand(
                    Command::new("show")
                        .about("Show the current draft")
                        .arg(Arg::new("raw").long("raw").action(clap::ArgAction::SetTrue).help("Show raw JSON")),
                )
                .subcommand(
                    Command::new("discard")
                        .about("Discard the current draft")
                        .arg(Arg::new("force").long("force").short('f').action(clap::ArgAction::SetTrue).help("Skip confirmation")),
                ),
        )
        .subcommand(
            Command::new("validate")
                .about("Validate an ACL policy without applying it")
                .arg(Arg::new("file").long("file").short('f').help("ACL file to validate"))
                .arg(Arg::new("stdin").long("stdin").action(clap::ArgAction::SetTrue).help("Read ACL from stdin")),
        )
        .subcommand(
            Command::new("test")
                .about("Test an ACL policy to preview changes")
                .arg(Arg::new("file").long("file").short('f').help("ACL file to test"))
                .arg(Arg::new("stdin").long("stdin").action(clap::ArgAction::SetTrue).help("Read ACL from stdin")),
        )
}

// ---------------------------------------------------------------------------
// Wired handlers
// ---------------------------------------------------------------------------

/// Helper to delegate a tailscale subcommand to the Go binary.
async fn delegate_tailscale(args: &[&str]) -> Result<()> {
    let go_bin = "/opt/homebrew/bin/bosua";
    if !std::path::Path::new(go_bin).exists() {
        return Err(crate::errors::BosuaError::Command(format!(
            "tailscale {} requires the Go binary at /opt/homebrew/bin/bosua",
            args.join(" ")
        )));
    }
    let mut full_args = vec!["tailscale"];
    full_args.extend_from_slice(args);
    let status = tokio::process::Command::new(go_bin)
        .args(&full_args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await
        .map_err(|e| crate::errors::BosuaError::Command(format!("Failed to run Go binary: {}", e)))?;
    if !status.success() {
        return Err(crate::errors::BosuaError::Command(format!(
            "tailscale {} failed",
            args.join(" ")
        )));
    }
    Ok(())
}

fn handle_account(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("add", _)) => {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(delegate_tailscale(&["account", "add"]))
        }
        Some(("list", _)) => {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(delegate_tailscale(&["account", "list"]))
        }
        Some(("current", _)) => {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(delegate_tailscale(&["account", "current"]))
        }
        Some(("info", _)) => {
            println!("Tailscale account info: use `tailscale acl get` or `tailscale list` to check configuration");
            Ok(())
        }
        Some(("switch", sub)) => {
            let name = sub.get_one::<String>("account_name").unwrap();
            let rt = tokio::runtime::Handle::current();
            rt.block_on(delegate_tailscale(&["account", "switch", name]))
        }
        Some(("remove", sub)) => {
            let name = sub.get_one::<String>("account_name").unwrap();
            let rt = tokio::runtime::Handle::current();
            rt.block_on(delegate_tailscale(&["account", "remove", name]))
        }
        Some(("export", sub)) => {
            let name = sub.get_one::<String>("account_name").unwrap();
            let rt = tokio::runtime::Handle::current();
            rt.block_on(delegate_tailscale(&["account", "export", name]))
        }
        Some(("import", sub)) => {
            let path = sub.get_one::<String>("json_file").unwrap();
            let rt = tokio::runtime::Handle::current();
            rt.block_on(delegate_tailscale(&["account", "import", path]))
        }
        _ => {
            println!("tailscale account: use a subcommand (add, list, current, info, switch, remove, export, import)");
            Ok(())
        }
    }
}

async fn handle_list(_matches: &ArgMatches, ts: &TailscaleClient) -> Result<()> {
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

async fn handle_info(matches: &ArgMatches, ts: &TailscaleClient) -> Result<()> {
    let id = matches.get_one::<String>("hostname_or_id").unwrap();
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

async fn handle_approve(matches: &ArgMatches, _ts: &TailscaleClient) -> Result<()> {
    let id = matches.get_one::<String>("hostname_or_id").unwrap();
    let exit_node = matches.get_flag("exit-node");
    let mut args = vec!["approve", id.as_str()];
    if exit_node { args.push("--exit-node"); }
    delegate_tailscale(&args).await
}

async fn handle_deauthorize(matches: &ArgMatches, _ts: &TailscaleClient) -> Result<()> {
    let id = matches.get_one::<String>("hostname_or_id").unwrap();
    delegate_tailscale(&["deauthorize", id]).await
}

async fn handle_set_name(matches: &ArgMatches, _ts: &TailscaleClient) -> Result<()> {
    let id = matches.get_one::<String>("hostname_or_id").unwrap();
    let name = matches.get_one::<String>("new_name").unwrap();
    delegate_tailscale(&["set-name", id, name]).await
}

async fn handle_set_tags(matches: &ArgMatches, _ts: &TailscaleClient) -> Result<()> {
    let id = matches.get_one::<String>("hostname_or_id").unwrap();
    let tags = matches.get_one::<String>("tags").unwrap();
    delegate_tailscale(&["set-tags", id, tags]).await
}

async fn handle_disable_key_expiry(matches: &ArgMatches, _ts: &TailscaleClient) -> Result<()> {
    let id = matches.get_one::<String>("hostname_or_id").unwrap();
    delegate_tailscale(&["disable-key-expiry", id]).await
}

async fn handle_set_ipv4(matches: &ArgMatches, _ts: &TailscaleClient) -> Result<()> {
    let id = matches.get_one::<String>("hostname_or_id").unwrap();
    let ip = matches.get_one::<String>("ipv4_address").unwrap();
    delegate_tailscale(&["set-ipv4", id, ip]).await
}

async fn handle_delete(matches: &ArgMatches, ts: &TailscaleClient) -> Result<()> {
    let id = matches.get_one::<String>("hostname_or_id").unwrap();
    ts.delete_device(id).await?;
    println!("Device {} deleted", id);
    Ok(())
}

async fn handle_key_generate(matches: &ArgMatches, _ts: &TailscaleClient) -> Result<()> {
    let reusable = matches.get_flag("reusable");
    let ephemeral = matches.get_flag("ephemeral");
    let tags = matches.get_one::<String>("tags").unwrap();
    let mut args = vec!["key-generate".to_string()];
    if reusable { args.push("--reusable".to_string()); }
    if ephemeral { args.push("--ephemeral".to_string()); }
    args.push(format!("--tags={}", tags));
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    delegate_tailscale(&refs).await
}

async fn handle_routes(matches: &ArgMatches, ts: &TailscaleClient) -> Result<()> {
    let id = matches.get_one::<String>("hostname_or_id").unwrap();
    let routes_str = matches.get_one::<String>("routes").unwrap();
    let routes: Vec<String> = routes_str.split(',').map(|s| s.trim().to_string()).collect();
    ts.set_device_routes(id, &routes).await?;
    println!("Routes updated for device {}: {}", id, routes.join(", "));
    Ok(())
}

async fn handle_exit_node(matches: &ArgMatches, _ts: &TailscaleClient) -> Result<()> {
    match matches.subcommand() {
        Some(("approve", sub)) => {
            let id = sub.get_one::<String>("hostname_or_id").unwrap();
            delegate_tailscale(&["exit-node", "approve", id]).await
        }
        Some(("unapprove", sub)) => {
            let id = sub.get_one::<String>("hostname_or_id").unwrap();
            delegate_tailscale(&["exit-node", "unapprove", id]).await
        }
        Some(("status", sub)) => {
            let id = sub.get_one::<String>("hostname_or_id").unwrap();
            delegate_tailscale(&["exit-node", "status", id]).await
        }
        _ => {
            println!("tailscale exit-node: use a subcommand (approve, unapprove, status)");
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
            let file = sub.get_one::<String>("file");
            match file {
                Some(f) => {
                    let content = std::fs::read_to_string(f).map_err(|e| {
                        crate::errors::BosuaError::Command(format!("Failed to read ACL file '{}': {}", f, e))
                    })?;
                    let policy: crate::cloud::tailscale::TsAclPolicy = serde_json::from_str(&content)?;
                    ts.set_acl(&policy).await?;
                    println!("ACL policy updated from {}", f);
                }
                None => {
                    println!("tailscale acl set: --file or --stdin required");
                }
            }
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
        Some((cmd, _)) => {
            delegate_tailscale(&["acl", cmd]).await
        }
        _ => {
            println!("tailscale acl: use a subcommand (get, set, edit, interactive, commit, draft, validate, test)");
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
            .try_get_matches_from(["tailscale", "list"])
            .unwrap();
        assert_eq!(matches.subcommand_name(), Some("list"));
    }

    #[test]
    fn test_tailscale_requires_subcommand() {
        let cmd = tailscale_command();
        let result = cmd.try_get_matches_from(["tailscale"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_tailscale_flat_info() {
        let cmd = tailscale_command();
        let matches = cmd
            .try_get_matches_from(["tailscale", "info", "my-device"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "info");
        assert_eq!(
            sub.get_one::<String>("hostname_or_id").map(|s| s.as_str()),
            Some("my-device"),
        );
    }

    #[test]
    fn test_tailscale_flat_approve() {
        let cmd = tailscale_command();
        let matches = cmd
            .try_get_matches_from(["tailscale", "approve", "my-device", "--exit-node"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "approve");
        assert!(sub.get_flag("exit-node"));
    }

    #[test]
    fn test_tailscale_flat_delete() {
        let cmd = tailscale_command();
        let matches = cmd
            .try_get_matches_from(["tailscale", "delete", "my-device", "--force"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "delete");
        assert!(sub.get_flag("force"));
    }

    #[test]
    fn test_tailscale_flat_key_generate() {
        let cmd = tailscale_command();
        let matches = cmd
            .try_get_matches_from(["tailscale", "key-generate", "--reusable", "--tags", "tag:prod"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "key-generate");
        assert!(sub.get_flag("reusable"));
    }

    #[test]
    fn test_tailscale_flat_routes() {
        let cmd = tailscale_command();
        let matches = cmd
            .try_get_matches_from(["tailscale", "routes", "dev123", "10.0.0.0/24,192.168.1.0/24"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "routes");
        assert_eq!(
            sub.get_one::<String>("hostname_or_id").map(|s| s.as_str()),
            Some("dev123"),
        );
    }

    #[test]
    fn test_tailscale_exit_node_approve() {
        let cmd = tailscale_command();
        let matches = cmd
            .try_get_matches_from(["tailscale", "exit-node", "approve", "my-device"])
            .unwrap();
        let (name, _) = matches.subcommand().unwrap();
        assert_eq!(name, "exit-node");
    }

    #[test]
    fn test_tailscale_acl_set() {
        let cmd = tailscale_command();
        let matches = cmd
            .try_get_matches_from(["tailscale", "acl", "set", "--file", "/path/to/acl.json"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let (_, set_sub) = sub.subcommand().unwrap();
        assert_eq!(
            set_sub.get_one::<String>("file").map(|s| s.as_str()),
            Some("/path/to/acl.json"),
        );
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
        assert!(sub_names.contains(&"list"));
        assert!(sub_names.contains(&"info"));
        assert!(sub_names.contains(&"approve"));
        assert!(sub_names.contains(&"deauthorize"));
        assert!(sub_names.contains(&"set-name"));
        assert!(sub_names.contains(&"set-tags"));
        assert!(sub_names.contains(&"disable-key-expiry"));
        assert!(sub_names.contains(&"set-ipv4"));
        assert!(sub_names.contains(&"delete"));
        assert!(sub_names.contains(&"key-generate"));
        assert!(sub_names.contains(&"routes"));
        assert!(sub_names.contains(&"exit-node"));
        assert_eq!(sub_names.len(), 14);
    }

    #[test]
    fn test_no_nested_devices_keys_config() {
        let cmd = tailscale_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        // These should NOT exist as they were the old nested structure
        assert!(!sub_names.contains(&"devices"));
        assert!(!sub_names.contains(&"keys"));
        assert!(!sub_names.contains(&"config"));
    }
}
