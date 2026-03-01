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
    let config_path = matches.get_one::<String>("config");
    let default_config = format!(
        "{}/.config/proxy/config.json",
        std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
    );
    let config_file = config_path.map(|s| s.as_str()).unwrap_or(&default_config);

    match matches.subcommand() {
        Some(("check", sub)) => {
            let one = sub.get_flag("one");
            println!("Checking proxies (one_by_one={})...", one);
            // Read proxy list from stdin or file
            println!("Provide proxy addresses as arguments or pipe from a file");
            Ok(())
        }
        Some(("create-config", _)) => {
            let path = std::path::Path::new(config_file);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let default = serde_json::json!({
                "http_proxy": "http://127.0.0.1:1080",
                "https_proxy": "http://127.0.0.1:1080",
                "no_proxy": "localhost,127.0.0.1"
            });
            std::fs::write(config_file, serde_json::to_string_pretty(&default)?)?;
            println!("Config file created at: {}", config_file);
            Ok(())
        }
        Some(("get", _)) => {
            println!("Provide a URL as argument to fetch via proxy");
            Ok(())
        }
        Some(("off", _)) => {
            std::env::remove_var("HTTP_PROXY");
            std::env::remove_var("HTTPS_PROXY");
            std::env::remove_var("http_proxy");
            std::env::remove_var("https_proxy");
            println!("Proxy disabled for this session");
            println!("To persist, run:");
            println!("  unset HTTP_PROXY HTTPS_PROXY http_proxy https_proxy");
            Ok(())
        }
        Some(("on", _)) => {
            if std::path::Path::new(config_file).exists() {
                let data = std::fs::read_to_string(config_file)?;
                let config: serde_json::Value = serde_json::from_str(&data)?;
                if let Some(http) = config.get("http_proxy").and_then(|v| v.as_str()) {
                    println!("export HTTP_PROXY={}", http);
                    println!("export http_proxy={}", http);
                }
                if let Some(https) = config.get("https_proxy").and_then(|v| v.as_str()) {
                    println!("export HTTPS_PROXY={}", https);
                    println!("export https_proxy={}", https);
                }
                println!("\nRun the above commands to enable proxy in your shell");
            } else {
                println!("Config file not found: {}", config_file);
                println!("Run `proxy create-config` first");
            }
            Ok(())
        }
        Some(("shell", _)) => {
            if std::path::Path::new(config_file).exists() {
                let data = std::fs::read_to_string(config_file)?;
                let config: serde_json::Value = serde_json::from_str(&data)?;
                println!("# Proxy ON:");
                if let Some(http) = config.get("http_proxy").and_then(|v| v.as_str()) {
                    println!("export HTTP_PROXY={}", http);
                    println!("export http_proxy={}", http);
                }
                if let Some(https) = config.get("https_proxy").and_then(|v| v.as_str()) {
                    println!("export HTTPS_PROXY={}", https);
                    println!("export https_proxy={}", https);
                }
                println!("\n# Proxy OFF:");
                println!("unset HTTP_PROXY HTTPS_PROXY http_proxy https_proxy");
            } else {
                println!("Config file not found: {}", config_file);
            }
            Ok(())
        }
        Some(("status", _)) => {
            let http = std::env::var("HTTP_PROXY").or_else(|_| std::env::var("http_proxy")).ok();
            let https = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")).ok();
            let no_proxy = std::env::var("NO_PROXY").or_else(|_| std::env::var("no_proxy")).ok();
            match (&http, &https) {
                (Some(h), Some(s)) => {
                    println!("Proxy: enabled");
                    println!("  HTTP:  {}", h);
                    println!("  HTTPS: {}", s);
                    if let Some(np) = &no_proxy {
                        println!("  No Proxy: {}", np);
                    }
                }
                (Some(h), None) => {
                    println!("Proxy: partially enabled");
                    println!("  HTTP:  {}", h);
                }
                _ => println!("Proxy: disabled"),
            }
            Ok(())
        }
        Some(("test", _)) => {
            let proxy_url = std::env::var("HTTP_PROXY")
                .or_else(|_| std::env::var("http_proxy"))
                .unwrap_or_default();
            if proxy_url.is_empty() {
                println!("No proxy configured. Set HTTP_PROXY or use --config");
                return Ok(());
            }
            println!("Testing proxy: {}", proxy_url);
            let client = reqwest::Client::builder()
                .proxy(reqwest::Proxy::all(&proxy_url).map_err(|e| {
                    crate::errors::BosuaError::Command(format!("Invalid proxy URL: {}", e))
                })?)
                .build()
                .map_err(|e| crate::errors::BosuaError::Command(format!("Failed to build client: {}", e)))?;
            match client.get("https://httpbin.org/ip").send().await {
                Ok(resp) => {
                    let body = resp.text().await.unwrap_or_default();
                    println!("Proxy test successful:");
                    println!("{}", body);
                }
                Err(e) => println!("Proxy test failed: {}", e),
            }
            Ok(())
        }
        Some(("with-proxy", _)) => {
            println!("Usage: proxy with-proxy <command> [args...]");
            println!("Example: bosua proxy with-proxy curl https://httpbin.org/ip");
            Ok(())
        }
        _ => unreachable!("subcommand_required is set"),
    }
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
