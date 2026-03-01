//! Proxy CLI command — Proxy management operations.
//!
//! Subcommands: check, create-config, get, off, on, shell, status, test, with-proxy.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::Result;

/// Build the `proxy` clap command.
pub fn proxy_command() -> Command {
    Command::new("proxy")
        .about("Proxy management operations")
        .aliases(["px"])
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(Arg::new("config").long("config").short('c').help("Path to proxy configuration file"))
        .subcommand(Command::new("check").about("Check proxy").aliases(["ch"]).arg(Arg::new("one").long("one").action(clap::ArgAction::SetTrue).help("Check one by one")))
        .subcommand(Command::new("create-config").about("Create a proxy configuration file"))
        .subcommand(Command::new("get").about("Get via proxy"))
        .subcommand(Command::new("off").about("Disable proxy"))
        .subcommand(Command::new("on").about("Enable proxy with configuration from file"))
        .subcommand(Command::new("shell").about("Generate shell commands for proxy management"))
        .subcommand(Command::new("status").about("Show current proxy status"))
        .subcommand(Command::new("test").about("Test proxy connection"))
        .subcommand(Command::new("with-proxy").about("Run a command with proxy temporarily enabled"))
}

/// Build the `CommandMeta` for registry registration.
pub fn proxy_meta() -> CommandMeta {
    CommandBuilder::from_clap(proxy_command())
        .category(CommandCategory::Network)
        .build()
}

/// Handle the `proxy` command.
pub async fn handle_proxy(matches: &ArgMatches) -> Result<()> {
    let config = matches.get_one::<String>("config");

    let (sub_name, sub_matches) = matches.subcommand().expect("subcommand_required is set");
    let mut args = vec!["proxy"];
    if let Some(c) = config {
        args.push("--config");
        args.push(c);
    }
    args.push(sub_name);
    // Forward --one flag for check
    if sub_name == "check" && sub_matches.get_flag("one") {
        args.push("--one");
    }
    super::delegate_to_go(&args).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_command_parses() {
        let cmd = proxy_command();
        let m = cmd.try_get_matches_from(["proxy", "status"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("status"));
    }

    #[test]
    fn test_proxy_alias_px() {
        let cmd = proxy_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"px"));
    }

    #[test]
    fn test_proxy_check_one() {
        let cmd = proxy_command();
        let m = cmd.try_get_matches_from(["proxy", "check", "--one"]).unwrap();
        let (name, sub) = m.subcommand().unwrap();
        assert_eq!(name, "check");
        assert!(sub.get_flag("one"));
    }

    #[test]
    fn test_proxy_config_flag() {
        let cmd = proxy_command();
        let m = cmd.try_get_matches_from(["proxy", "--config", "/tmp/proxy.conf", "status"]).unwrap();
        assert_eq!(m.get_one::<String>("config").map(|s| s.as_str()), Some("/tmp/proxy.conf"));
    }

    #[test]
    fn test_proxy_requires_subcommand() {
        let cmd = proxy_command();
        assert!(cmd.try_get_matches_from(["proxy"]).is_err());
    }

    #[test]
    fn test_proxy_meta() {
        let meta = proxy_meta();
        assert_eq!(meta.name, "proxy");
        assert_eq!(meta.category, CommandCategory::Network);
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = proxy_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        for name in &["check", "create-config", "get", "off", "on", "shell", "status", "test", "with-proxy"] {
            assert!(sub_names.contains(name), "missing: {}", name);
        }
        assert_eq!(sub_names.len(), 9);
    }
}
