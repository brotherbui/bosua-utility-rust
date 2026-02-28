//! Google Cloud SDK CLI command with subcommands.
//!
//! Provides the `gcloud` command with subcommands: account, compute,
//! firewall, region, zone, ami.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::config::dynamic::DynamicConfig;
use crate::errors::Result;
use crate::utils::run_external_tool;

/// Build the `gcloud` clap command with all subcommands.
pub fn gcloud_command() -> Command {
    Command::new("gcloud")
        .aliases(["gc"])
        .about("Google Cloud CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        // Persistent flags matching Go
        .arg(Arg::new("name").short('n').long("name").global(true).help("Name of the compute instance"))
        .arg(Arg::new("region").short('r').long("region").global(true).default_value("asia-east2-c").help("GCP region for resources. Defaults to asia-east2-c (Hong Kong)"))
        .arg(Arg::new("machine-type").short('t').long("machine-type").global(true).default_value("e2-medium").help("Machine type"))
        .arg(Arg::new("ami").short('a').long("ami").global(true).default_value("debian-12").help("Image name"))
        .arg(Arg::new("disk-size").short('d').long("disk-size").global(true).default_value("200").help("Disk size"))
        .subcommand(account_subcommand())
        .subcommand(instance_subcommand())
        .subcommand(firewall_subcommand())
        .subcommand(ami_subcommand())
        .subcommand(region_subcommand())
        .subcommand(zone_subcommand())
}

/// Build the `CommandMeta` for registry registration.
pub fn gcloud_meta() -> CommandMeta {
    CommandBuilder::from_clap(gcloud_command())
        .category(CommandCategory::Cloud)
        .build()
}

/// Handle the `gcloud` command dispatch.
pub async fn handle_gcloud(
    matches: &ArgMatches,
    config: &DynamicConfig,
) -> Result<()> {
    match matches.subcommand() {
        Some(("account", sub)) => handle_account(sub).await,
        Some(("instance", sub)) => handle_compute(sub, config).await,
        Some(("firewall", sub)) => handle_firewall(sub).await,
        Some(("ami", sub)) => handle_ami(sub).await,
        Some(("region", sub)) => handle_region(sub).await,
        Some(("zone", sub)) => handle_zone(sub).await,
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// Subcommand definitions
// ---------------------------------------------------------------------------

fn account_subcommand() -> Command {
    Command::new("account")
        .aliases(["a", "acc"])
        .about("Manage GCP service accounts")
        .subcommand(Command::new("add").about("Add a new GCP service account"))
        .subcommand(
            Command::new("list")
                .aliases(["ls"])
                .about("List all configured accounts")
                .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON")),
        )
        .subcommand(Command::new("current").aliases(["c"]).about("Show current account"))
        .subcommand(
            Command::new("info")
                .about("Show account information")
                .arg(Arg::new("account_name").help("Account name (optional)")),
        )
        .subcommand(
            Command::new("switch")
                .aliases(["s", "use"])
                .about("Switch to a different account")
                .arg(Arg::new("account_name").help("Account name")),
        )
        .subcommand(
            Command::new("remove")
                .aliases(["rm", "del"])
                .about("Remove an account")
                .arg(Arg::new("account_name").required(true).help("Account name")),
        )
        .subcommand(Command::new("import").about("Import account configuration").arg(Arg::new("file").required(true).help("Import file path")))
        .subcommand(Command::new("export").about("Export account configuration").arg(Arg::new("file").help("Export file path")))
}

fn instance_subcommand() -> Command {
    Command::new("instance")
        .aliases(["i", "inst"])
        .about("Manage Compute Engine Instances")
        .subcommand(
            Command::new("create")
                .aliases(["c", "a", "add"])
                .about("Create an instance"),
        )
        .subcommand(
            Command::new("list")
                .aliases(["ls", "l"])
                .about("List instances with firewall rules")
                .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON")),
        )
        .subcommand(
            Command::new("info")
                .aliases(["describe", "d"])
                .about("Show instance details")
                .arg(Arg::new("instance").required(true).help("Instance name")),
        )
        .subcommand(
            Command::new("start")
                .about("Start an instance")
                .arg(Arg::new("instance").required(true).help("Instance name")),
        )
        .subcommand(
            Command::new("stop")
                .about("Stop an instance")
                .arg(Arg::new("instance").required(true).help("Instance name")),
        )
        .subcommand(
            Command::new("delete")
                .aliases(["rm", "terminate"])
                .about("Delete an instance")
                .arg(Arg::new("instance").required(true).help("Instance name")),
        )
}

fn firewall_subcommand() -> Command {
    Command::new("firewall")
        .aliases(["fw", "f"])
        .about("Manage firewall rules")
        .subcommand(Command::new("list").aliases(["ls", "l"]).about("List firewall rules"))
        .subcommand(
            Command::new("create")
                .aliases(["c", "add"])
                .about("Create a firewall rule")
                .arg(Arg::new("name").required(true).help("Rule name"))
                .arg(Arg::new("allow").long("allow").help("Allowed protocols and ports (e.g. tcp:80,443)")),
        )
        .subcommand(
            Command::new("delete")
                .aliases(["rm", "del"])
                .about("Delete a firewall rule")
                .arg(Arg::new("name").required(true).help("Rule name")),
        )
}

fn region_subcommand() -> Command {
    Command::new("region")
        .about("Region operations")
        .subcommand(Command::new("list").about("List available regions"))
        .subcommand(
            Command::new("set")
                .about("Set the default region")
                .arg(Arg::new("region").required(true).help("Region name")),
        )
}

fn zone_subcommand() -> Command {
    Command::new("zone")
        .about("Zone operations")
        .subcommand(Command::new("list").about("List available zones"))
        .subcommand(
            Command::new("set")
                .about("Set the default zone")
                .arg(Arg::new("zone").required(true).help("Zone name")),
        )
}

fn ami_subcommand() -> Command {
    Command::new("ami")
        .about("Machine image operations")
        .subcommand(Command::new("list").about("List available machine images"))
        .subcommand(
            Command::new("describe")
                .about("Describe a machine image")
                .arg(Arg::new("image").required(true).help("Image name or ID")),
        )
}

// ---------------------------------------------------------------------------
// Handlers — delegate to the `gcloud` CLI tool
// ---------------------------------------------------------------------------

async fn handle_account(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let output = run_external_tool("gcloud", &["auth", "list"]).await?;
            println!("{}", output);
            Ok(())
        }
        Some(("current", _)) | Some(("info", _)) => {
            let output = run_external_tool("gcloud", &["config", "list", "account"]).await?;
            println!("{}", output);
            Ok(())
        }
        Some((name, _)) => {
            println!("gcloud account {}: not yet implemented", name);
            Ok(())
        }
        _ => {
            println!("gcloud account: use a subcommand");
            Ok(())
        }
    }
}

async fn handle_compute(matches: &ArgMatches, config: &DynamicConfig) -> Result<()> {
    let zone = &config.gcloud_region;
    match matches.subcommand() {
        Some(("list", _)) => {
            let output = run_external_tool(
                "gcloud",
                &["compute", "instances", "list", "--zone", zone, "--format=table(name,zone,status,networkInterfaces[0].accessConfigs[0].natIP)"],
            )
            .await?;
            println!("{}", output);
            Ok(())
        }
        Some(("start", sub)) => {
            let instance = sub.get_one::<String>("instance").unwrap();
            let output = run_external_tool(
                "gcloud",
                &["compute", "instances", "start", instance, "--zone", zone],
            )
            .await?;
            println!("{}", output);
            println!("Instance '{}' started", instance);
            Ok(())
        }
        Some(("stop", sub)) => {
            let instance = sub.get_one::<String>("instance").unwrap();
            let output = run_external_tool(
                "gcloud",
                &["compute", "instances", "stop", instance, "--zone", zone],
            )
            .await?;
            println!("{}", output);
            println!("Instance '{}' stopped", instance);
            Ok(())
        }
        Some(("describe", sub)) => {
            let instance = sub.get_one::<String>("instance").unwrap();
            let output = run_external_tool(
                "gcloud",
                &["compute", "instances", "describe", instance, "--zone", zone],
            )
            .await?;
            println!("{}", output);
            Ok(())
        }
        _ => {
            println!("gcloud compute: use a subcommand (list, start, stop, describe)");
            Ok(())
        }
    }
}

async fn handle_firewall(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let output = run_external_tool(
                "gcloud",
                &["compute", "firewall-rules", "list"],
            )
            .await?;
            println!("{}", output);
            Ok(())
        }
        Some(("create", sub)) => {
            let name = sub.get_one::<String>("name").unwrap();
            let mut args = vec!["compute", "firewall-rules", "create", name.as_str()];
            let allow_val;
            if let Some(allow) = sub.get_one::<String>("allow") {
                allow_val = format!("--allow={}", allow);
                args.push(&allow_val);
            }
            let output = run_external_tool("gcloud", &args).await?;
            println!("{}", output);
            println!("Firewall rule '{}' created", name);
            Ok(())
        }
        Some(("delete", sub)) => {
            let name = sub.get_one::<String>("name").unwrap();
            let output = run_external_tool(
                "gcloud",
                &["compute", "firewall-rules", "delete", name, "--quiet"],
            )
            .await?;
            println!("{}", output);
            println!("Firewall rule '{}' deleted", name);
            Ok(())
        }
        _ => {
            println!("gcloud firewall: use a subcommand (list, create, delete)");
            Ok(())
        }
    }
}

async fn handle_region(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let output = run_external_tool(
                "gcloud",
                &["compute", "regions", "list"],
            )
            .await?;
            println!("{}", output);
            Ok(())
        }
        Some(("set", sub)) => {
            let region = sub.get_one::<String>("region").unwrap();
            let output = run_external_tool(
                "gcloud",
                &["config", "set", "compute/region", region],
            )
            .await?;
            println!("{}", output);
            println!("Default region set to: {}", region);
            Ok(())
        }
        _ => {
            println!("gcloud region: use a subcommand (list, set)");
            Ok(())
        }
    }
}

async fn handle_zone(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let output = run_external_tool(
                "gcloud",
                &["compute", "zones", "list"],
            )
            .await?;
            println!("{}", output);
            Ok(())
        }
        Some(("set", sub)) => {
            let zone = sub.get_one::<String>("zone").unwrap();
            let output = run_external_tool(
                "gcloud",
                &["config", "set", "compute/zone", zone],
            )
            .await?;
            println!("{}", output);
            println!("Default zone set to: {}", zone);
            Ok(())
        }
        _ => {
            println!("gcloud zone: use a subcommand (list, set)");
            Ok(())
        }
    }
}

async fn handle_ami(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let output = run_external_tool(
                "gcloud",
                &["compute", "images", "list", "--no-standard-images"],
            )
            .await?;
            println!("{}", output);
            Ok(())
        }
        Some(("describe", sub)) => {
            let image = sub.get_one::<String>("image").unwrap();
            let output = run_external_tool(
                "gcloud",
                &["compute", "images", "describe", image],
            )
            .await?;
            println!("{}", output);
            Ok(())
        }
        _ => {
            println!("gcloud ami: use a subcommand (list, describe)");
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
    fn test_gcloud_command_parses() {
        let cmd = gcloud_command();
        let matches = cmd.try_get_matches_from(["gcloud", "region", "list"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("region"));
    }

    #[test]
    fn test_gcloud_requires_subcommand() {
        let cmd = gcloud_command();
        let result = cmd.try_get_matches_from(["gcloud"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_gcloud_compute_start() {
        let cmd = gcloud_command();
        let matches = cmd
            .try_get_matches_from(["gcloud", "instance", "start", "my-instance"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "instance");
        let (sub_name, _) = sub.subcommand().unwrap();
        assert_eq!(sub_name, "start");
    }

    #[test]
    fn test_gcloud_firewall_create() {
        let cmd = gcloud_command();
        let matches = cmd
            .try_get_matches_from(["gcloud", "firewall", "create", "allow-http", "--allow", "tcp:80"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let (_, create_sub) = sub.subcommand().unwrap();
        assert_eq!(
            create_sub.get_one::<String>("name").map(|s| s.as_str()),
            Some("allow-http"),
        );
        assert_eq!(
            create_sub.get_one::<String>("allow").map(|s| s.as_str()),
            Some("tcp:80"),
        );
    }

    #[test]
    fn test_gcloud_meta() {
        let meta = gcloud_meta();
        assert_eq!(meta.name, "gcloud");
        assert_eq!(meta.category, CommandCategory::Cloud);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = gcloud_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"account"));
        assert!(sub_names.contains(&"instance"));
        assert!(sub_names.contains(&"firewall"));
        assert!(sub_names.contains(&"ami"));
        assert!(sub_names.contains(&"region"));
        assert!(sub_names.contains(&"zone"));
        assert_eq!(sub_names.len(), 6);
    }
}
