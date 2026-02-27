pub mod registry;

pub use registry::{CommandBuilder, CommandCategory, CommandMeta, CommandRegistry, RegistryStats};

/// Creates the root clap Command with global `--verbose` and `--json` flags.
///
/// The `--verbose` / `-v` flag enables detailed output across all subcommands.
/// When combined with `--json`, verbose output is suppressed to keep JSON clean.
pub fn create_root_command() -> clap::Command {
    clap::Command::new("bosua")
        .about("Bosua CLI toolkit")
        .arg(
            clap::Arg::new("verbose")
                .short('v')
                .long("verbose")
                .global(true)
                .action(clap::ArgAction::SetTrue)
                .help("Enable verbose output"),
        )
        .arg(
            clap::Arg::new("json")
                .long("json")
                .global(true)
                .action(clap::ArgAction::SetTrue)
                .help("Output in JSON format"),
        )
}

/// Returns whether verbose mode is active based on parsed matches.
///
/// Verbose is suppressed when `--json` is also set, to keep JSON output clean.
pub fn is_verbose(matches: &clap::ArgMatches) -> bool {
    let verbose = matches.get_flag("verbose");
    let json = matches.get_flag("json");
    verbose && !json
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_command_has_verbose_flag() {
        let cmd = create_root_command();
        let matches = cmd.try_get_matches_from(["bosua", "--verbose"]).unwrap();
        assert!(matches.get_flag("verbose"));
    }

    #[test]
    fn test_root_command_has_json_flag() {
        let cmd = create_root_command();
        let matches = cmd.try_get_matches_from(["bosua", "--json"]).unwrap();
        assert!(matches.get_flag("json"));
    }

    #[test]
    fn test_verbose_suppressed_with_json() {
        let cmd = create_root_command();
        let matches = cmd
            .try_get_matches_from(["bosua", "--verbose", "--json"])
            .unwrap();
        assert!(!is_verbose(&matches));
    }

    #[test]
    fn test_verbose_active_without_json() {
        let cmd = create_root_command();
        let matches = cmd
            .try_get_matches_from(["bosua", "--verbose"])
            .unwrap();
        assert!(is_verbose(&matches));
    }

    #[test]
    fn test_short_verbose_flag() {
        let cmd = create_root_command();
        let matches = cmd.try_get_matches_from(["bosua", "-v"]).unwrap();
        assert!(is_verbose(&matches));
    }

    #[test]
    fn test_no_flags_not_verbose() {
        let cmd = create_root_command();
        let matches = cmd.try_get_matches_from(["bosua"]).unwrap();
        assert!(!is_verbose(&matches));
    }
}
