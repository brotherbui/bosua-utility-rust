//! Info CLI command — display system and application information.
//!
//! Supports `--json` flag for machine-readable output.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};

/// Build the `info` clap command.
///
/// In the Go version, the Linux/GCP variant registers this as `config`
/// with aliases `conf, info`. For the macOS variant, `AddCommonCmd()`
/// registers `config` with aliases `i, c, conf, info`.
/// We keep the function name as `info_command` but the clap name matches Go.
pub fn info_command() -> Command {
    Command::new("config")
        .aliases(["conf", "info"])
        .about("Show system and configuration information")
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output information as JSON"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn info_meta() -> CommandMeta {
    CommandBuilder::from_clap(info_command())
        .category(CommandCategory::Core)
        .build()
}

/// Handle the `info` command.
pub fn handle_info(matches: &ArgMatches) {
    let json = matches.get_flag("json");

    if json {
        println!("{{\"os\":\"{}\",\"arch\":\"{}\"}}", std::env::consts::OS, std::env::consts::ARCH);
    } else {
        println!("OS:   {}", std::env::consts::OS);
        println!("Arch: {}", std::env::consts::ARCH);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_info_command_parses() {
        let cmd = info_command();
        let matches = cmd.try_get_matches_from(["config"]).unwrap();
        assert!(!matches.get_flag("json"));
    }

    #[test]
    fn test_info_command_json_flag() {
        let cmd = info_command();
        let matches = cmd.try_get_matches_from(["config", "--json"]).unwrap();
        assert!(matches.get_flag("json"));
    }

    #[test]
    fn test_info_meta() {
        let meta = info_meta();
        assert_eq!(meta.name, "config");
        assert_eq!(meta.category, CommandCategory::Core);
        assert!(!meta.description.is_empty());
    }
}
