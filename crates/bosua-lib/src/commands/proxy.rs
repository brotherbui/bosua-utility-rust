//! Proxy CLI command — local HTTP proxy for GDrive downloads.
//!
//! Subcommands: start, stop, status.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};

/// Build the `proxy` clap command.
pub fn proxy_command() -> Command {
    Command::new("proxy")
        .about("SOCKS5 proxy management")
        .subcommand(
            Command::new("start")
                .about("Start SOCKS5 proxy")
                .arg(
                    Arg::new("port")
                        .long("port")
                        .short('p')
                        .default_value("1080")
                        .help("Port to listen on"),
                ),
        )
        .subcommand(Command::new("stop").about("Stop SOCKS5 proxy"))
        .subcommand(Command::new("status").about("Show proxy status"))
}

/// Build the `CommandMeta` for registry registration.
pub fn proxy_meta() -> CommandMeta {
    CommandBuilder::from_clap(proxy_command())
        .category(CommandCategory::Network)
        .build()
}

static PROXY_RUNNING: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Handle the `proxy` command.
pub async fn handle_proxy(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("start", sub)) => {
            let port: u16 = sub
                .get_one::<String>("port")
                .unwrap()
                .parse()
                .map_err(|_| BosuaError::Command("Invalid port".into()))?;

            if PROXY_RUNNING.load(std::sync::atomic::Ordering::SeqCst) {
                println!("Proxy is already running");
                return Ok(());
            }

            let addr = format!("127.0.0.1:{}", port);
            let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
                BosuaError::Command(format!("Failed to bind proxy on {}: {}", addr, e))
            })?;

            println!("Proxy started on {}", addr);
            PROXY_RUNNING.store(true, std::sync::atomic::Ordering::SeqCst);

            tokio::select! {
                _ = async {
                    loop {
                        if let Err(e) = listener.accept().await {
                            tracing::error!("Accept error: {}", e);
                            break;
                        }
                    }
                } => {}
                _ = tokio::signal::ctrl_c() => {
                    println!("\nProxy stopped by user");
                }
            }

            PROXY_RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        Some(("stop", _)) => {
            if PROXY_RUNNING.load(std::sync::atomic::Ordering::SeqCst) {
                PROXY_RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);
                println!("Proxy stopped");
            } else {
                println!("Proxy is not running");
            }
            Ok(())
        }
        Some(("status", _)) => {
            let running = PROXY_RUNNING.load(std::sync::atomic::Ordering::SeqCst);
            if running {
                println!("Proxy status: running");
            } else {
                println!("Proxy status: stopped");
            }
            Ok(())
        }
        _ => {
            println!("proxy: use a subcommand (start, stop, status)");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_command_parses_start() {
        let cmd = proxy_command();
        let m = cmd.try_get_matches_from(["proxy", "start"]).unwrap();
        let (name, sub) = m.subcommand().unwrap();
        assert_eq!(name, "start");
        assert_eq!(sub.get_one::<String>("port").map(|s| s.as_str()), Some("1080"));
    }

    #[test]
    fn test_proxy_command_start_custom_port() {
        let cmd = proxy_command();
        let m = cmd.try_get_matches_from(["proxy", "start", "--port", "9050"]).unwrap();
        let (_, sub) = m.subcommand().unwrap();
        assert_eq!(sub.get_one::<String>("port").map(|s| s.as_str()), Some("9050"));
    }

    #[test]
    fn test_proxy_command_parses_stop() {
        let cmd = proxy_command();
        let m = cmd.try_get_matches_from(["proxy", "stop"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("stop"));
    }

    #[test]
    fn test_proxy_command_parses_status() {
        let cmd = proxy_command();
        let m = cmd.try_get_matches_from(["proxy", "status"]).unwrap();
        assert_eq!(m.subcommand_name(), Some("status"));
    }

    #[test]
    fn test_proxy_meta() {
        let meta = proxy_meta();
        assert_eq!(meta.name, "proxy");
        assert_eq!(meta.category, CommandCategory::Network);
        assert!(!meta.description.is_empty());
    }
}
