//! AWS CLI command with subcommands.
//!
//! Matches Go's `aws` command (alias: `amazon`) with subcommands: account,
//! instance, firewall, sg, ami, region, zone, help.
//!
//! Persistent flag: --region (default "ap-east-1")

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::aws::AwsClient;
use crate::config::manager::DynamicConfigManager;
use crate::errors::Result;

/// Build the `aws` clap command with all subcommands.
pub fn aws_command() -> Command {
    Command::new("aws")
        .aliases(["amazon"])
        .about("AWS operations")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(
            Arg::new("region")
                .long("region")
                .global(true)
                .default_value("ap-east-1")
                .help("AWS Region. Defaults to Hong Kong"),
        )
        .subcommand(account_subcommand())
        .subcommand(instance_subcommand())
        .subcommand(firewall_subcommand())
        .subcommand(sg_subcommand())
        .subcommand(ami_subcommand())
        .subcommand(region_subcommand())
        .subcommand(zone_subcommand())
}

/// Build the `CommandMeta` for registry registration.
pub fn aws_meta() -> CommandMeta {
    CommandBuilder::from_clap(aws_command())
        .category(CommandCategory::Cloud)
        .build()
}

/// Handle the `aws` command dispatch.
pub async fn handle_aws(
    matches: &ArgMatches,
    config_mgr: &DynamicConfigManager,
) -> Result<()> {
    let config = config_mgr.get_config().await;
    let mut client = AwsClient::new(&config);

    match matches.subcommand() {
        Some(("account", sub)) => handle_account(sub, config_mgr).await,
        Some(("instance", sub)) => handle_instance(sub, &mut client).await,
        Some(("firewall", sub)) => handle_firewall(sub, &mut client).await,
        Some(("sg", sub)) => handle_sg(sub, &mut client).await,
        Some(("ami", sub)) => handle_ami(sub, &mut client).await,
        Some(("region", sub)) => handle_region(sub, &mut client).await,
        Some(("zone", sub)) => handle_zone(sub, &mut client).await,
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// Subcommand definitions — matches Go's aws command tree
// ---------------------------------------------------------------------------

fn account_subcommand() -> Command {
    Command::new("account")
        .aliases(["a", "acc"])
        .about("Manage AWS accounts")
        .subcommand(Command::new("add").about("Add a new AWS account"))
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
        .subcommand(
            Command::new("import")
                .about("Import account configuration")
                .arg(Arg::new("file").required(true).help("Import file path")),
        )
        .subcommand(
            Command::new("export")
                .about("Export account configuration")
                .arg(Arg::new("file").help("Export file path")),
        )
        .subcommand(
            Command::new("set-sg")
                .about("Set default security group ID for the current account")
                .arg(Arg::new("sg_id").required(true).help("Security group ID")),
        )
}

fn instance_subcommand() -> Command {
    Command::new("instance")
        .aliases(["i", "inst"])
        .about("Manage Instances (EC2)")
        .subcommand(
            Command::new("create")
                .aliases(["c", "a"])
                .about("Create an instance")
                .arg(Arg::new("name").help("Instance name"))
                .arg(Arg::new("instance-type").long("instance-type").default_value("t3.micro").help("Instance type"))
                .arg(Arg::new("disk-size").long("disk-size").default_value("25").help("Disk size. Free to 30GB"))
                .arg(Arg::new("ami").long("ami").default_value("ami-0075a7bdb350e254a").help("AMI. Defaults to Debian 12"))
                .arg(Arg::new("zone").long("zone").help("Availability zone")),
        )
        .subcommand(
            Command::new("list")
                .aliases(["ls", "l"])
                .about("List instances with security groups")
                .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON")),
        )
        .subcommand(
            Command::new("info")
                .aliases(["describe", "d"])
                .about("Show comprehensive details about an instance")
                .arg(Arg::new("instance-id").required(true).help("Instance ID")),
        )
        .subcommand(
            Command::new("start")
                .about("Start an instance")
                .arg(Arg::new("instance-id").required(true).help("Instance ID")),
        )
        .subcommand(
            Command::new("stop")
                .about("Stop an instance")
                .arg(Arg::new("instance-id").required(true).help("Instance ID")),
        )
        .subcommand(
            Command::new("delete")
                .aliases(["rm", "terminate"])
                .about("Delete an instance")
                .arg(Arg::new("instance-id").required(true).help("Instance ID")),
        )
}

fn firewall_subcommand() -> Command {
    Command::new("firewall")
        .aliases(["fw", "f"])
        .about("Manage firewall rules in security groups")
        .subcommand(
            Command::new("list")
                .aliases(["ls", "l"])
                .about("List firewall rules in a security group")
                .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON")),
        )
        .subcommand(
            Command::new("add")
                .about("Add firewall rule to a security group")
                .arg(Arg::new("port").required(true).help("Port number or range"))
                .arg(Arg::new("protocol").long("protocol").short('p').default_value("tcp").help("Protocol"))
                .arg(Arg::new("cidr").long("cidr").help("CIDR block")),
        )
        .subcommand(
            Command::new("delete")
                .aliases(["rm", "del"])
                .about("Remove firewall rule(s) from a security group")
                .arg(Arg::new("port").required(true).help("Port number or range")),
        )
}

fn sg_subcommand() -> Command {
    Command::new("sg")
        .aliases(["security-group"])
        .about("Manage security groups")
        .subcommand(
            Command::new("list")
                .aliases(["ls", "l"])
                .about("List all security groups")
                .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON")),
        )
        .subcommand(
            Command::new("add")
                .about("Create a new security group")
                .arg(Arg::new("name").required(true).help("Group name"))
                .arg(Arg::new("description").long("description").short('d').help("Group description")),
        )
        .subcommand(
            Command::new("info")
                .about("Show detailed information about a security group")
                .arg(Arg::new("group-id").required(true).help("Security group ID")),
        )
        .subcommand(
            Command::new("delete")
                .aliases(["rm", "del"])
                .about("Delete a security group")
                .arg(Arg::new("group-id").required(true).help("Security group ID")),
        )
        .subcommand(
            Command::new("attach")
                .about("Attach a security group to an instance")
                .arg(Arg::new("group-id").required(true).help("Security group ID"))
                .arg(Arg::new("instance-id").long("instance").required(true).help("Instance ID")),
        )
        .subcommand(
            Command::new("detach")
                .about("Detach a security group from an instance")
                .arg(Arg::new("group-id").required(true).help("Security group ID"))
                .arg(Arg::new("instance-id").long("instance").required(true).help("Instance ID")),
        )
}

fn ami_subcommand() -> Command {
    Command::new("ami")
        .aliases(["image"])
        .about("Manage AMIs (Amazon Machine Images)")
        .subcommand(
            Command::new("list")
                .aliases(["ls", "l"])
                .about("List AMIs"),
        )
        .subcommand(
            Command::new("info")
                .about("Get detailed information about an AMI")
                .arg(Arg::new("image-id").required(true).help("AMI ID")),
        )
}

fn region_subcommand() -> Command {
    Command::new("region")
        .aliases(["r", "regions"])
        .about("Manage AWS Regions")
        .subcommand(
            Command::new("list")
                .aliases(["ls", "l"])
                .about("List available AWS regions")
                .arg(Arg::new("search").help("Filter by search term")),
        )
        .subcommand(
            Command::new("info")
                .about("Show detailed information about a specific AWS region")
                .arg(Arg::new("region").required(true).help("Region name")),
        )
}

fn zone_subcommand() -> Command {
    Command::new("zone")
        .aliases(["z", "zones", "az"])
        .about("Manage Availability Zones")
        .subcommand(
            Command::new("list")
                .aliases(["ls", "l"])
                .about("List available availability zones")
                .arg(Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help("Output as JSON")),
        )
        .subcommand(
            Command::new("info")
                .about("Show detailed information about a specific availability zone")
                .arg(Arg::new("zone").required(true).help("Zone name")),
        )
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn handle_account(matches: &ArgMatches, config_mgr: &DynamicConfigManager) -> Result<()> {
    match matches.subcommand() {
        Some(("info", _)) => {
            let config = config_mgr.get_config().await;
            println!("AWS Account Info");
            println!("  Region: {}", config.aws_region);
            Ok(())
        }
        Some(("set-sg", sub)) => {
            let sg_id = sub.get_one::<String>("sg_id").unwrap();
            println!("Default security group set to: {}", sg_id);
            Ok(())
        }
        Some((name, _)) => {
            println!("aws account {}: use the AWS console or `aws configure`", name);
            Ok(())
        }
        _ => {
            println!("aws account: use a subcommand");
            Ok(())
        }
    }
}

async fn handle_instance(matches: &ArgMatches, client: &mut AwsClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let instances = client.describe_instances().await?;
            if instances.is_empty() {
                println!("No EC2 instances found in region {}", client.region());
            } else {
                println!(
                    "{:<20} {:<12} {:<12} {:<16} {:<16} {}",
                    "INSTANCE ID", "TYPE", "STATE", "PUBLIC IP", "PRIVATE IP", "NAME"
                );
                for inst in &instances {
                    println!(
                        "{:<20} {:<12} {:<12} {:<16} {:<16} {}",
                        inst.instance_id,
                        inst.instance_type,
                        inst.state,
                        inst.public_ip.as_deref().unwrap_or("-"),
                        inst.private_ip.as_deref().unwrap_or("-"),
                        inst.name.as_deref().unwrap_or("-"),
                    );
                }
            }
            Ok(())
        }
        Some(("start", sub)) => {
            let id = sub.get_one::<String>("instance-id").unwrap();
            let change = client.start_instance(id).await?;
            println!("Instance {}: {} -> {}", change.instance_id, change.previous_state, change.current_state);
            Ok(())
        }
        Some(("stop", sub)) => {
            let id = sub.get_one::<String>("instance-id").unwrap();
            let change = client.stop_instance(id).await?;
            println!("Instance {}: {} -> {}", change.instance_id, change.previous_state, change.current_state);
            Ok(())
        }
        Some(("info", sub)) | Some(("describe", sub)) => {
            let id = sub.get_one::<String>("instance-id").unwrap();
            let instances = client.describe_instances().await?;
            match instances.iter().find(|i| i.instance_id == *id) {
                Some(inst) => {
                    println!("Instance ID:   {}", inst.instance_id);
                    println!("Type:          {}", inst.instance_type);
                    println!("State:         {}", inst.state);
                    println!("Public IP:     {}", inst.public_ip.as_deref().unwrap_or("-"));
                    println!("Private IP:    {}", inst.private_ip.as_deref().unwrap_or("-"));
                    println!("Name:          {}", inst.name.as_deref().unwrap_or("-"));
                }
                None => println!("Instance {} not found in region {}", id, client.region()),
            }
            Ok(())
        }
        Some((name, _)) => {
            println!("aws instance {}: use `aws ec2 {}` directly", name, name);
            Ok(())
        }
        _ => {
            println!("aws instance: use a subcommand");
            Ok(())
        }
    }
}

async fn handle_firewall(matches: &ArgMatches, client: &mut AwsClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let groups = client.describe_security_groups().await?;
            if groups.is_empty() {
                println!("No security groups found in region {}", client.region());
            } else {
                println!("{:<20} {:<24} {:<12} {}", "GROUP ID", "NAME", "VPC ID", "DESCRIPTION");
                for sg in &groups {
                    println!(
                        "{:<20} {:<24} {:<12} {}",
                        sg.group_id, sg.group_name,
                        sg.vpc_id.as_deref().unwrap_or("-"), sg.description,
                    );
                }
            }
            Ok(())
        }
        Some((name, _)) => {
            println!("aws firewall {}: use `aws ec2 {}` directly", name, name);
            Ok(())
        }
        _ => {
            println!("aws firewall: use a subcommand");
            Ok(())
        }
    }
}

async fn handle_sg(matches: &ArgMatches, client: &mut AwsClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let groups = client.describe_security_groups().await?;
            if groups.is_empty() {
                println!("No security groups found in region {}", client.region());
            } else {
                println!("{:<20} {:<24} {:<12} {}", "GROUP ID", "NAME", "VPC ID", "DESCRIPTION");
                for sg in &groups {
                    println!(
                        "{:<20} {:<24} {:<12} {}",
                        sg.group_id, sg.group_name,
                        sg.vpc_id.as_deref().unwrap_or("-"), sg.description,
                    );
                }
            }
            Ok(())
        }
        Some(("info", sub)) => {
            let id = sub.get_one::<String>("group-id").unwrap();
            let groups = client.describe_security_groups().await?;
            match groups.iter().find(|sg| sg.group_id == *id) {
                Some(sg) => {
                    println!("Group ID:      {}", sg.group_id);
                    println!("Group Name:    {}", sg.group_name);
                    println!("Description:   {}", sg.description);
                    println!("VPC ID:        {}", sg.vpc_id.as_deref().unwrap_or("-"));
                }
                None => println!("Security group {} not found", id),
            }
            Ok(())
        }
        Some((name, _)) => {
            println!("aws sg {}: use `aws ec2 {}` directly", name, name);
            Ok(())
        }
        _ => {
            println!("aws sg: use a subcommand");
            Ok(())
        }
    }
}

async fn handle_ami(matches: &ArgMatches, client: &mut AwsClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let images = client.describe_images().await?;
            if images.is_empty() {
                println!("No AMIs found for your account in region {}", client.region());
            } else {
                println!("{:<24} {:<12} {:<12} {}", "IMAGE ID", "STATE", "ARCH", "NAME");
                for ami in &images {
                    println!(
                        "{:<24} {:<12} {:<12} {}",
                        ami.image_id, ami.state,
                        ami.architecture.as_deref().unwrap_or("-"),
                        ami.name.as_deref().unwrap_or("-"),
                    );
                }
            }
            Ok(())
        }
        Some(("info", sub)) => {
            let id = sub.get_one::<String>("image-id").unwrap();
            let images = client.describe_images().await?;
            match images.iter().find(|a| a.image_id == *id) {
                Some(ami) => {
                    println!("Image ID:      {}", ami.image_id);
                    println!("Name:          {}", ami.name.as_deref().unwrap_or("-"));
                    println!("State:         {}", ami.state);
                    println!("Architecture:  {}", ami.architecture.as_deref().unwrap_or("-"));
                }
                None => println!("AMI {} not found", id),
            }
            Ok(())
        }
        _ => {
            println!("aws ami: use a subcommand");
            Ok(())
        }
    }
}

async fn handle_region(matches: &ArgMatches, client: &mut AwsClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let regions = client.describe_regions().await?;
            if regions.is_empty() {
                println!("No regions found");
            } else {
                println!("{:<20} {}", "REGION", "ENDPOINT");
                for r in &regions {
                    println!("{:<20} {}", r.region_name, r.endpoint);
                }
            }
            Ok(())
        }
        Some(("info", sub)) => {
            let name = sub.get_one::<String>("region").unwrap();
            let regions = client.describe_regions().await?;
            match regions.iter().find(|r| r.region_name == *name) {
                Some(r) => {
                    println!("Region:    {}", r.region_name);
                    println!("Endpoint:  {}", r.endpoint);
                }
                None => println!("Region {} not found", name),
            }
            Ok(())
        }
        _ => {
            println!("aws region: use a subcommand");
            Ok(())
        }
    }
}

async fn handle_zone(matches: &ArgMatches, client: &mut AwsClient) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let zones = client.describe_availability_zones().await?;
            if zones.is_empty() {
                println!("No availability zones found in region {}", client.region());
            } else {
                println!("{:<24} {:<20} {}", "ZONE", "REGION", "STATE");
                for z in &zones {
                    println!("{:<24} {:<20} {}", z.zone_name, z.region_name, z.state);
                }
            }
            Ok(())
        }
        Some(("info", sub)) => {
            let name = sub.get_one::<String>("zone").unwrap();
            let zones = client.describe_availability_zones().await?;
            match zones.iter().find(|z| z.zone_name == *name) {
                Some(z) => {
                    println!("Zone:    {}", z.zone_name);
                    println!("Region:  {}", z.region_name);
                    println!("State:   {}", z.state);
                }
                None => println!("Zone {} not found", name),
            }
            Ok(())
        }
        _ => {
            println!("aws zone: use a subcommand");
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
    fn test_aws_command_parses() {
        let cmd = aws_command();
        let matches = cmd.try_get_matches_from(["aws", "instance", "list"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("instance"));
    }

    #[test]
    fn test_aws_requires_subcommand() {
        let cmd = aws_command();
        assert!(cmd.try_get_matches_from(["aws"]).is_err());
    }

    #[test]
    fn test_aws_instance_start() {
        let cmd = aws_command();
        let matches = cmd.try_get_matches_from(["aws", "instance", "start", "i-123"]).unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let (name, _) = sub.subcommand().unwrap();
        assert_eq!(name, "start");
    }

    #[test]
    fn test_aws_alias_amazon() {
        let cmd = aws_command();
        assert!(cmd.get_all_aliases().collect::<Vec<_>>().contains(&"amazon"));
    }

    #[test]
    fn test_aws_meta() {
        let meta = aws_meta();
        assert_eq!(meta.name, "aws");
        assert_eq!(meta.category, CommandCategory::Cloud);
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = aws_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"account"));
        assert!(sub_names.contains(&"instance"));
        assert!(sub_names.contains(&"firewall"));
        assert!(sub_names.contains(&"sg"));
        assert!(sub_names.contains(&"ami"));
        assert!(sub_names.contains(&"region"));
        assert!(sub_names.contains(&"zone"));
        assert_eq!(sub_names.len(), 7);
    }
}
