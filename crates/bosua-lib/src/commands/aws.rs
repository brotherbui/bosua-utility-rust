//! AWS CLI command with subcommands.
//!
//! Provides the `aws` command with subcommands: account, ec2, firewall,
//! region, zone, sg, ami.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::cloud::aws::AwsClient;
use crate::config::manager::DynamicConfigManager;
use crate::errors::Result;

/// Build the `aws` clap command with all subcommands.
pub fn aws_command() -> Command {
    Command::new("aws")
        .about("AWS EC2 resource management")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(account_subcommand())
        .subcommand(ec2_subcommand())
        .subcommand(firewall_subcommand())
        .subcommand(region_subcommand())
        .subcommand(zone_subcommand())
        .subcommand(sg_subcommand())
        .subcommand(ami_subcommand())
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
        Some(("ec2", sub)) => handle_ec2(sub, &mut client).await,
        Some(("firewall", sub)) => handle_firewall(sub, &mut client).await,
        Some(("region", sub)) => handle_region(sub, &mut client).await,
        Some(("zone", sub)) => handle_zone(sub, &mut client).await,
        Some(("sg", sub)) => handle_sg(sub, &mut client).await,
        Some(("ami", sub)) => handle_ami(sub, &mut client).await,
        _ => unreachable!("subcommand_required is set"),
    }
}

// ---------------------------------------------------------------------------
// Subcommand definitions
// ---------------------------------------------------------------------------

fn account_subcommand() -> Command {
    Command::new("account")
        .about("Manage AWS accounts")
        .subcommand(Command::new("info").about("Show current account info"))
        .subcommand(
            Command::new("set-region")
                .about("Set the default AWS region")
                .arg(Arg::new("region").required(true).help("AWS region name")),
        )
}

fn ec2_subcommand() -> Command {
    Command::new("ec2")
        .about("EC2 instance management")
        .subcommand(Command::new("list").about("List EC2 instances"))
        .subcommand(
            Command::new("start")
                .about("Start an EC2 instance")
                .arg(Arg::new("instance-id").required(true).help("Instance ID")),
        )
        .subcommand(
            Command::new("stop")
                .about("Stop an EC2 instance")
                .arg(Arg::new("instance-id").required(true).help("Instance ID")),
        )
        .subcommand(
            Command::new("describe")
                .about("Describe an EC2 instance")
                .arg(Arg::new("instance-id").required(true).help("Instance ID")),
        )
}

fn firewall_subcommand() -> Command {
    Command::new("firewall")
        .about("Firewall (security group) management")
        .subcommand(Command::new("list").about("List security groups"))
        .subcommand(
            Command::new("describe")
                .about("Describe a security group")
                .arg(Arg::new("group-id").required(true).help("Security group ID")),
        )
}

fn region_subcommand() -> Command {
    Command::new("region")
        .about("AWS region operations")
        .subcommand(Command::new("list").about("List available regions"))
        .subcommand(
            Command::new("set")
                .about("Set the default region")
                .arg(Arg::new("region").required(true).help("Region name")),
        )
}

fn zone_subcommand() -> Command {
    Command::new("zone")
        .about("Availability zone operations")
        .subcommand(Command::new("list").about("List availability zones"))
}

fn sg_subcommand() -> Command {
    Command::new("sg")
        .about("Security group operations")
        .subcommand(Command::new("list").about("List security groups"))
        .subcommand(
            Command::new("describe")
                .about("Describe a security group")
                .arg(Arg::new("group-id").required(true).help("Security group ID")),
        )
        .subcommand(
            Command::new("create")
                .about("Create a security group")
                .arg(Arg::new("name").required(true).help("Group name"))
                .arg(
                    Arg::new("description")
                        .long("description")
                        .short('d')
                        .help("Group description"),
                ),
        )
}

fn ami_subcommand() -> Command {
    Command::new("ami")
        .about("AMI (Amazon Machine Image) operations")
        .subcommand(Command::new("list").about("List AMIs owned by the account"))
        .subcommand(
            Command::new("describe")
                .about("Describe an AMI")
                .arg(Arg::new("image-id").required(true).help("AMI ID")),
        )
}

// ---------------------------------------------------------------------------
// Wired handlers
// ---------------------------------------------------------------------------

async fn handle_account(matches: &ArgMatches, config_mgr: &DynamicConfigManager) -> Result<()> {
    match matches.subcommand() {
        Some(("info", _)) => {
            let config = config_mgr.get_config().await;
            println!("AWS Account Info");
            println!("  Region: {}", config.aws_region);
            Ok(())
        }
        Some(("set-region", sub)) => {
            let region = sub.get_one::<String>("region").unwrap();
            let mut updates = serde_json::Map::new();
            updates.insert(
                "aws_region".to_string(),
                serde_json::Value::String(region.clone()),
            );
            config_mgr.update_config(updates).await?;
            println!("AWS region set to: {}", region);
            Ok(())
        }
        _ => {
            println!("aws account: use a subcommand (info, set-region)");
            Ok(())
        }
    }
}

async fn handle_ec2(matches: &ArgMatches, client: &mut AwsClient) -> Result<()> {
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
            println!(
                "Instance {}: {} -> {}",
                change.instance_id, change.previous_state, change.current_state
            );
            Ok(())
        }
        Some(("stop", sub)) => {
            let id = sub.get_one::<String>("instance-id").unwrap();
            let change = client.stop_instance(id).await?;
            println!(
                "Instance {}: {} -> {}",
                change.instance_id, change.previous_state, change.current_state
            );
            Ok(())
        }
        Some(("describe", sub)) => {
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
                None => {
                    println!("Instance {} not found in region {}", id, client.region());
                }
            }
            Ok(())
        }
        _ => unreachable!("subcommand_required is set on ec2"),
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
                        sg.group_id,
                        sg.group_name,
                        sg.vpc_id.as_deref().unwrap_or("-"),
                        sg.description,
                    );
                }
            }
            Ok(())
        }
        Some(("describe", sub)) => {
            let id = sub.get_one::<String>("group-id").unwrap();
            let groups = client.describe_security_groups().await?;
            match groups.iter().find(|sg| sg.group_id == *id) {
                Some(sg) => {
                    println!("Group ID:      {}", sg.group_id);
                    println!("Group Name:    {}", sg.group_name);
                    println!("Description:   {}", sg.description);
                    println!("VPC ID:        {}", sg.vpc_id.as_deref().unwrap_or("-"));
                }
                None => {
                    println!("Security group {} not found", id);
                }
            }
            Ok(())
        }
        _ => {
            println!("aws firewall: use a subcommand (list, describe)");
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
        Some(("set", sub)) => {
            let _region = sub.get_one::<String>("region").unwrap();
            // Region setting is handled via config; this is a convenience alias
            println!("Use `config set aws_region {}` to change the default region", _region);
            Ok(())
        }
        _ => {
            println!("aws region: use a subcommand (list, set)");
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
        _ => {
            println!("aws zone: use a subcommand (list)");
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
                        sg.group_id,
                        sg.group_name,
                        sg.vpc_id.as_deref().unwrap_or("-"),
                        sg.description,
                    );
                }
            }
            Ok(())
        }
        Some(("describe", sub)) => {
            let id = sub.get_one::<String>("group-id").unwrap();
            let groups = client.describe_security_groups().await?;
            match groups.iter().find(|sg| sg.group_id == *id) {
                Some(sg) => {
                    println!("Group ID:      {}", sg.group_id);
                    println!("Group Name:    {}", sg.group_name);
                    println!("Description:   {}", sg.description);
                    println!("VPC ID:        {}", sg.vpc_id.as_deref().unwrap_or("-"));
                }
                None => {
                    println!("Security group {} not found", id);
                }
            }
            Ok(())
        }
        Some(("create", sub)) => {
            let name = sub.get_one::<String>("name").unwrap();
            let _desc = sub.get_one::<String>("description");
            // AwsClient doesn't expose create_security_group yet
            println!("aws sg create {}: not yet supported by AwsClient", name);
            Ok(())
        }
        _ => {
            println!("aws sg: use a subcommand (list, describe, create)");
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
                        ami.image_id,
                        ami.state,
                        ami.architecture.as_deref().unwrap_or("-"),
                        ami.name.as_deref().unwrap_or("-"),
                    );
                }
            }
            Ok(())
        }
        Some(("describe", sub)) => {
            let id = sub.get_one::<String>("image-id").unwrap();
            let images = client.describe_images().await?;
            match images.iter().find(|a| a.image_id == *id) {
                Some(ami) => {
                    println!("Image ID:      {}", ami.image_id);
                    println!("Name:          {}", ami.name.as_deref().unwrap_or("-"));
                    println!("State:         {}", ami.state);
                    println!("Architecture:  {}", ami.architecture.as_deref().unwrap_or("-"));
                }
                None => {
                    println!("AMI {} not found", id);
                }
            }
            Ok(())
        }
        _ => {
            println!("aws ami: use a subcommand (list, describe)");
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
        let matches = cmd.try_get_matches_from(["aws", "ec2", "list"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("ec2"));
    }

    #[test]
    fn test_aws_requires_subcommand() {
        let cmd = aws_command();
        let result = cmd.try_get_matches_from(["aws"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_aws_ec2_start() {
        let cmd = aws_command();
        let matches = cmd
            .try_get_matches_from(["aws", "ec2", "start", "i-1234567890abcdef0"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let (sub_name, sub_sub) = sub.subcommand().unwrap();
        assert_eq!(sub_name, "start");
        assert_eq!(
            sub_sub.get_one::<String>("instance-id").map(|s| s.as_str()),
            Some("i-1234567890abcdef0"),
        );
    }

    #[test]
    fn test_aws_sg_create() {
        let cmd = aws_command();
        let matches = cmd
            .try_get_matches_from(["aws", "sg", "create", "my-sg", "-d", "My security group"])
            .unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        let (_, create_sub) = sub.subcommand().unwrap();
        assert_eq!(
            create_sub.get_one::<String>("name").map(|s| s.as_str()),
            Some("my-sg"),
        );
        assert_eq!(
            create_sub.get_one::<String>("description").map(|s| s.as_str()),
            Some("My security group"),
        );
    }

    #[test]
    fn test_aws_meta() {
        let meta = aws_meta();
        assert_eq!(meta.name, "aws");
        assert_eq!(meta.category, CommandCategory::Cloud);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = aws_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(sub_names.contains(&"account"));
        assert!(sub_names.contains(&"ec2"));
        assert!(sub_names.contains(&"firewall"));
        assert!(sub_names.contains(&"region"));
        assert!(sub_names.contains(&"zone"));
        assert!(sub_names.contains(&"sg"));
        assert!(sub_names.contains(&"ami"));
        assert_eq!(sub_names.len(), 7);
    }
}
