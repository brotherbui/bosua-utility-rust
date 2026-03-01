//! HTTP server command.
//!
//! Provides the `serve` command (aliases: `http`, `server`) for starting a secure HTTP server.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::Result;

/// Build the `serve` clap command.
pub fn serve_command() -> Command {
    Command::new("serve")
        .about("Start secure HTTP server")
        .aliases(["http", "server"])
        .arg(Arg::new("cert-file").long("cert-file").help("TLS certificate file path"))
        .arg(Arg::new("host").long("host").default_value("0.0.0.0").help("Host to bind HTTP server to"))
        .arg(Arg::new("key-file").long("key-file").help("TLS private key file path"))
        .arg(Arg::new("port").long("port").default_value("8080").help("Port to run HTTP server on"))
        .arg(Arg::new("tls").long("tls").action(clap::ArgAction::SetTrue).help("Enable TLS/HTTPS"))
}

/// Build the `CommandMeta` for registry registration.
pub fn serve_meta() -> CommandMeta {
    CommandBuilder::from_clap(serve_command())
        .category(CommandCategory::Core)
        .build()
}

/// Handle the `serve` command.
pub async fn handle_serve(matches: &ArgMatches) -> Result<()> {
    let host = matches.get_one::<String>("host").unwrap();
    let port = matches.get_one::<String>("port").unwrap();
    let tls = matches.get_flag("tls");

    let scheme = if tls { "https" } else { "http" };
    println!("Starting {} server on {}:{}", scheme, host, port);

    if tls {
        let _cert = matches.get_one::<String>("cert-file");
        let _key = matches.get_one::<String>("key-file");
        println!("TLS enabled");
    }

    println!("serve: not yet implemented");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serve_command_defaults() {
        let cmd = serve_command();
        let matches = cmd.try_get_matches_from(["serve"]).unwrap();
        assert_eq!(matches.get_one::<String>("host").map(|s| s.as_str()), Some("0.0.0.0"));
        assert_eq!(matches.get_one::<String>("port").map(|s| s.as_str()), Some("8080"));
        assert!(!matches.get_flag("tls"));
    }

    #[test]
    fn test_serve_tls_flags() {
        let cmd = serve_command();
        let matches = cmd.try_get_matches_from(["serve", "--tls", "--cert-file", "cert.pem", "--key-file", "key.pem"]).unwrap();
        assert!(matches.get_flag("tls"));
        assert_eq!(matches.get_one::<String>("cert-file").map(|s| s.as_str()), Some("cert.pem"));
        assert_eq!(matches.get_one::<String>("key-file").map(|s| s.as_str()), Some("key.pem"));
    }

    #[test]
    fn test_serve_meta() {
        let meta = serve_meta();
        assert_eq!(meta.name, "serve");
        assert_eq!(meta.category, CommandCategory::Core);
    }

    #[test]
    fn test_serve_aliases() {
        let cmd = serve_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"http"));
        assert!(aliases.contains(&"server"));
    }
}
