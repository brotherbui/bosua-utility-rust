//! Version CLI command — display application version.
//!
//! Supports `--json` flag for machine-readable output.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};

/// Build the `version` clap command.
pub fn version_command() -> Command {
    Command::new("version")
        .about("Show version and build information")
        .aliases(["v"])
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output version information as JSON"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn version_meta() -> CommandMeta {
    CommandBuilder::from_clap(version_command())
        .category(CommandCategory::Core)
        .build()
}

/// Handle the `version` command.
pub fn handle_version(matches: &ArgMatches) {
    let json = matches.get_flag("json");
    let version = env!("CARGO_PKG_VERSION");

    if json {
        println!(
            "{{\"version\":\"{}\",\"name\":\"bosua\"}}",
            version,
        );
    } else {
        println!("bosua version {}", version);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_command_parses() {
        let cmd = version_command();
        let matches = cmd.try_get_matches_from(["version"]).unwrap();
        assert!(!matches.get_flag("json"));
    }

    #[test]
    fn test_version_command_json_flag() {
        let cmd = version_command();
        let matches = cmd.try_get_matches_from(["version", "--json"]).unwrap();
        assert!(matches.get_flag("json"));
    }

    #[test]
    fn test_version_meta() {
        let meta = version_meta();
        assert_eq!(meta.name, "version");
        assert_eq!(meta.category, CommandCategory::Core);
        assert!(!meta.description.is_empty());
    }
}
