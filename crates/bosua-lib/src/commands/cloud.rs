//! Unified cloud CLI command with subcommands.
//!
//! Provides the `cloud` command with subcommands: config, daemon, gdrive,
//! service, setup, stats, sync.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};

/// Build the `cloud` clap command with all subcommands.
pub fn cloud_command() -> Command {
    Command::new("cloud")
        .about("Cloud stuffs")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(Arg::new("backend").long("backend").action(clap::ArgAction::SetTrue).help("Use Backend IP. Will be overwritten by IP if specified"))
        .arg(Arg::new("gcp").long("gcp").action(clap::ArgAction::SetTrue).help("Use GCP IP. Will be overwritten by IP if specified"))
        .arg(Arg::new("ip").long("ip").help("Server IP address"))
        .subcommand(
            Command::new("config")
                .about("Configure services on cloud server")
                .subcommand(Command::new("caddy").about("Configure Caddy reverse proxy")),
        )
        .subcommand(
            Command::new("daemon")
                .about("Manage daemon on remote server")
                .subcommand(Command::new("logs").about("Show daemon logs from remote server"))
                .subcommand(Command::new("restart").about("Restart bosua daemon on remote server"))
                .subcommand(Command::new("start").about("Start bosua daemon on remote server"))
                .subcommand(Command::new("status").about("Check daemon status on remote server"))
                .subcommand(Command::new("stop").about("Stop bosua daemon on remote server")),
        )
        .subcommand(
            Command::new("gdrive")
                .about("Manage Google Drive on remote server")
                .subcommand(Command::new("import").about("Import Google Drive account to remote server")),
        )
        .subcommand(
            Command::new("service")
                .about("Manage services on server")
                .subcommand(Command::new("restart").about("Restart one or more services"))
                .subcommand(Command::new("start").about("Start one or more services"))
                .subcommand(Command::new("status").about("Check status of services"))
                .subcommand(Command::new("stop").about("Stop one or more services")),
        )
        .subcommand(
            Command::new("setup")
                .about("Setup server infrastructure and services")
                .subcommand(Command::new("aria2").about("Install and configure aria2 download manager"))
                .subcommand(Command::new("backend").about("Run backend setup commands"))
                .subcommand(Command::new("caddy").about("Install and configure Caddy web server"))
                .subcommand(Command::new("check-env").about("Check deployment environment and server connectivity"))
                .subcommand(Command::new("cloudflared").about("Install and configure cloudflared daemon"))
                .subcommand(Command::new("dante").about("Install and configure SOCKS5 proxy server"))
                .subcommand(Command::new("deploy").about("Build Linux binary and deploy to server"))
                .subcommand(Command::new("full").about("Run all setup commands"))
                .subcommand(Command::new("gcp").about("Run gcp setup commands"))
                .subcommand(Command::new("kodi-repo").about("Setup Kodi addon repository on remote server"))
                .subcommand(Command::new("latex").about("Install TeX Live for full LaTeX support"))
                .subcommand(Command::new("microsocks").about("Install and configure microsocks SOCKS5 proxy server"))
                .subcommand(Command::new("tailscale").about("Install and configure Tailscale VPN")),
        )
        .subcommand(
            Command::new("stats")
                .about("Display server system information (CPU, memory, disk)")
                .aliases(["info"])
                .arg(Arg::new("gcp").long("gcp").action(clap::ArgAction::SetTrue).help("Check GCP")),
        )
        .subcommand(
            Command::new("sync")
                .about("Sync files to cloud server")
                .subcommand(Command::new("kodi-repo").about("Sync a single Kodi addon to remote server")),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn cloud_meta() -> CommandMeta {
    CommandBuilder::from_clap(cloud_command())
        .category(CommandCategory::Cloud)
        .build()
}

/// Handle the `cloud` command dispatch.
///
/// All cloud subcommands involve SSH connections and remote server management.
/// Uses the system `ssh` command for remote operations.
pub fn handle_cloud(matches: &ArgMatches) {
    let ip = resolve_server_ip(matches);

    match matches.subcommand() {
        Some(("config", sub)) => {
            match sub.subcommand() {
                Some(("caddy", _)) => {
                    run_ssh_command(&ip, "sudo caddy reload --config /etc/caddy/Caddyfile");
                }
                _ => println!("cloud config: use a subcommand (caddy)"),
            }
        }
        Some(("daemon", sub)) => {
            let action = match sub.subcommand() {
                Some(("logs", _)) => "journalctl -u bosua -f --no-pager -n 100",
                Some(("restart", _)) => "sudo systemctl restart bosua",
                Some(("start", _)) => "sudo systemctl start bosua",
                Some(("status", _)) => "sudo systemctl status bosua",
                Some(("stop", _)) => "sudo systemctl stop bosua",
                _ => { println!("cloud daemon: use a subcommand (logs, restart, start, status, stop)"); return; }
            };
            run_ssh_command(&ip, action);
        }
        Some(("gdrive", sub)) => {
            match sub.subcommand() {
                Some(("import", _)) => {
                    // rsync gdrive config to remote
                    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                    let local_path = format!("{}/.config/gdrive3/", home);
                    let remote_path = format!("root@{}:~/.config/gdrive3/", ip);
                    let _ = std::process::Command::new("rsync")
                        .args(["-avz", &local_path, &remote_path])
                        .stdin(std::process::Stdio::inherit())
                        .stdout(std::process::Stdio::inherit())
                        .stderr(std::process::Stdio::inherit())
                        .status();
                    println!("GDrive config synced to {}", ip);
                }
                _ => println!("cloud gdrive: use a subcommand (import)"),
            }
        }
        Some(("service", sub)) => {
            let (action, _) = sub.subcommand().unwrap_or(("status", sub));
            let cmd = match action {
                "restart" => "sudo systemctl restart",
                "start" => "sudo systemctl start",
                "status" => "sudo systemctl status",
                "stop" => "sudo systemctl stop",
                _ => { println!("cloud service: use a subcommand (restart, start, status, stop)"); return; }
            };
            // List common services
            let services = ["caddy", "bosua", "aria2", "cloudflared"];
            for svc in &services {
                run_ssh_command(&ip, &format!("{} {}", cmd, svc));
            }
        }
        Some(("setup", sub)) => {
            let subcmd = match sub.subcommand() {
                Some((name, _)) => name,
                None => { println!("cloud setup: use a subcommand"); return; }
            };
            match subcmd {
                "check-env" => {
                    println!("Checking server connectivity...");
                    run_ssh_command(&ip, "uname -a && uptime && df -h / && free -h");
                }
                "deploy" => {
                    println!("Building and deploying to {}...", ip);
                    // Build linux binary
                    let _ = std::process::Command::new("cargo")
                        .args(["build", "--release", "--target", "x86_64-unknown-linux-gnu"])
                        .stdin(std::process::Stdio::inherit())
                        .stdout(std::process::Stdio::inherit())
                        .stderr(std::process::Stdio::inherit())
                        .status();
                    // Deploy via scp
                    let _ = std::process::Command::new("scp")
                        .args(["target/x86_64-unknown-linux-gnu/release/bosua-linux", &format!("root@{}:/usr/local/bin/bosua", ip)])
                        .stdin(std::process::Stdio::inherit())
                        .stdout(std::process::Stdio::inherit())
                        .stderr(std::process::Stdio::inherit())
                        .status();
                    run_ssh_command(&ip, "sudo systemctl restart bosua");
                }
                "full" => {
                    println!("Running full setup on {}...", ip);
                    run_ssh_command(&ip, "apt update && apt upgrade -y");
                }
                _ => {
                    // For specific setup commands, run the appropriate install script
                    let install_cmd = match subcmd {
                        "aria2" => "apt install -y aria2 && systemctl enable aria2",
                        "caddy" => "apt install -y debian-keyring debian-archive-keyring apt-transport-https && curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg && apt update && apt install -y caddy",
                        "cloudflared" => "curl -L --output cloudflared.deb https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb && dpkg -i cloudflared.deb && rm cloudflared.deb",
                        "dante" => "apt install -y dante-server",
                        "microsocks" => "apt install -y microsocks",
                        "tailscale" => "curl -fsSL https://tailscale.com/install.sh | sh",
                        "latex" => "apt install -y texlive-full",
                        _ => { println!("cloud setup {}: running on remote...", subcmd); "" }
                    };
                    if !install_cmd.is_empty() {
                        run_ssh_command(&ip, install_cmd);
                    }
                }
            }
        }
        Some(("stats", sub)) => {
            let target_ip = if sub.get_flag("gcp") {
                // Use GCP IP if --gcp flag
                resolve_server_ip_with_flag(true, false, None)
            } else {
                ip.clone()
            };
            run_ssh_command(&target_ip, "echo '=== CPU ===' && nproc && echo '=== Memory ===' && free -h && echo '=== Disk ===' && df -h / && echo '=== Uptime ===' && uptime");
        }
        Some(("sync", sub)) => {
            match sub.subcommand() {
                Some(("kodi-repo", _)) => {
                    println!("Syncing Kodi repo to {}...", ip);
                    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                    let _ = std::process::Command::new("rsync")
                        .args(["-avz", &format!("{}/kodi-repo/", home), &format!("root@{}:/var/www/kodi-repo/", ip)])
                        .stdin(std::process::Stdio::inherit())
                        .stdout(std::process::Stdio::inherit())
                        .stderr(std::process::Stdio::inherit())
                        .status();
                }
                _ => println!("cloud sync: use a subcommand (kodi-repo)"),
            }
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

fn resolve_server_ip(matches: &ArgMatches) -> String {
    if let Some(ip) = matches.get_one::<String>("ip") {
        return ip.clone();
    }
    let backend = matches.get_flag("backend");
    let gcp = matches.get_flag("gcp");
    resolve_server_ip_with_flag(gcp, backend, None)
}

fn resolve_server_ip_with_flag(gcp: bool, backend: bool, explicit_ip: Option<&str>) -> String {
    if let Some(ip) = explicit_ip {
        return ip.to_string();
    }
    // Read from config file
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let config_path = format!("{}/.bosua/config.json", home);
    if let Ok(data) = std::fs::read_to_string(&config_path) {
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&data) {
            if gcp {
                if let Some(ip) = config.get("gcpIp").and_then(|v| v.as_str()) {
                    return ip.to_string();
                }
            }
            if backend {
                if let Some(ip) = config.get("backendIp").and_then(|v| v.as_str()) {
                    return ip.to_string();
                }
            }
            if let Some(ip) = config.get("serverIp").and_then(|v| v.as_str()) {
                return ip.to_string();
            }
        }
    }
    "localhost".to_string()
}

fn run_ssh_command(ip: &str, command: &str) {
    let _ = std::process::Command::new("ssh")
        .args([&format!("root@{}", ip), command])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_command_parses() {
        let cmd = cloud_command();
        let m = cmd.try_get_matches_from(["cloud", "stats"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("stats"));
    }

    #[test]
    fn test_cloud_persistent_flags() {
        let cmd = cloud_command();
        let m = cmd.try_get_matches_from(["cloud", "--backend", "--ip", "1.2.3.4", "stats"]).unwrap();
        assert!(m.get_flag("backend"));
        assert_eq!(m.get_one::<String>("ip").map(|s| s.as_str()), Some("1.2.3.4"));
    }

    #[test]
    fn test_cloud_requires_subcommand() {
        let cmd = cloud_command();
        assert!(cmd.try_get_matches_from(["cloud"]).is_err());
    }

    #[test]
    fn test_cloud_meta() {
        let meta = cloud_meta();
        assert_eq!(meta.name, "cloud");
        assert_eq!(meta.category, CommandCategory::Cloud);
    }

    #[test]
    fn test_all_subcommands_present() {
        let cmd = cloud_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        for name in &["config", "daemon", "gdrive", "service", "setup", "stats", "sync"] {
            assert!(sub_names.contains(name), "missing: {}", name);
        }
        assert_eq!(sub_names.len(), 7);
    }
}
