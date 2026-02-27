//! Luhn CLI command — Luhn algorithm operations for checksum validation.
//!
//! Matches Go's `luhn` command with subcommands: check, generate.
//! Aliases: c, conf.

use clap::{ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};

/// Build the `luhn` clap command with subcommands.
pub fn checksum_command() -> Command {
    Command::new("luhn")
        .aliases(["c", "conf"])
        .about("Luhn algorithm operations for checksum validation")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("check")
                .aliases(["c", "chk"])
                .about("Check Luhn valid number")
                .arg(
                    clap::Arg::new("numbers")
                        .num_args(1..)
                        .required(true)
                        .help("Numbers to validate"),
                ),
        )
        .subcommand(
            Command::new("generate")
                .aliases(["g", "gen"])
                .about("Generate Luhn valid number")
                .arg(
                    clap::Arg::new("prefix")
                        .required(true)
                        .help("Prefix for number generation"),
                ),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn checksum_meta() -> CommandMeta {
    CommandBuilder::from_clap(checksum_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `luhn` command dispatch.
/// Validate a number using the Luhn algorithm.
///
/// Strips non-digit characters, then doubles every second digit from the right,
/// sums all digits, and checks if the total is divisible by 10.
pub fn luhn_check(number: &str) -> bool {
    let digits: Vec<u32> = number
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c.to_digit(10).unwrap())
        .collect();

    if digits.is_empty() {
        return false;
    }

    let mut sum = 0u32;
    for (i, &d) in digits.iter().rev().enumerate() {
        if i % 2 == 1 {
            let doubled = d * 2;
            sum += if doubled > 9 { doubled - 9 } else { doubled };
        } else {
            sum += d;
        }
    }
    sum % 10 == 0
}

/// Generate a Luhn-valid number from a prefix.
///
/// Appends a check digit to the prefix so the resulting number passes the Luhn check.
pub fn luhn_generate(prefix: &str) -> std::result::Result<String, String> {
    let digits: Vec<u32> = prefix
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c.to_digit(10).unwrap())
        .collect();

    if digits.is_empty() {
        return Err("Prefix must contain at least one digit".to_string());
    }

    // Append 0 as placeholder check digit
    let mut with_check: Vec<u32> = digits.clone();
    with_check.push(0);

    // Calculate Luhn sum with the placeholder
    let mut sum = 0u32;
    for (i, &d) in with_check.iter().rev().enumerate() {
        if i % 2 == 1 {
            let doubled = d * 2;
            sum += if doubled > 9 { doubled - 9 } else { doubled };
        } else {
            sum += d;
        }
    }

    let check_digit = (10 - (sum % 10)) % 10;

    // Build the result string preserving original prefix formatting
    let prefix_clean: String = digits.iter().map(|d| std::char::from_digit(*d, 10).unwrap()).collect();
    Ok(format!("{}{}", prefix_clean, check_digit))
}

/// Handle the `luhn` command dispatch.
pub fn handle_checksum(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("check", sub)) => {
            let numbers: Vec<&String> = sub.get_many::<String>("numbers").unwrap().collect();
            for num in numbers {
                let valid = luhn_check(num);
                println!("{}: {}", num, if valid { "valid" } else { "invalid" });
            }
            Ok(())
        }
        Some(("generate", sub)) => {
            let prefix = sub.get_one::<String>("prefix").unwrap();
            match luhn_generate(prefix) {
                Ok(result) => {
                    println!("{}", result);
                    Ok(())
                }
                Err(e) => Err(BosuaError::Command(e)),
            }
        }
        _ => unreachable!("subcommand_required is set"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_luhn_command_parses_check() {
        let cmd = checksum_command();
        let matches = cmd.try_get_matches_from(["luhn", "check", "12345"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("check"));
    }

    #[test]
    fn test_luhn_command_parses_generate() {
        let cmd = checksum_command();
        let matches = cmd
            .try_get_matches_from(["luhn", "generate", "4111"])
            .unwrap();
        assert_eq!(matches.subcommand_name(), Some("generate"));
    }

    #[test]
    fn test_luhn_requires_subcommand() {
        let cmd = checksum_command();
        let result = cmd.try_get_matches_from(["luhn"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_luhn_meta() {
        let meta = checksum_meta();
        assert_eq!(meta.name, "luhn");
        assert_eq!(meta.category, CommandCategory::Utility);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_luhn_check_valid_numbers() {
        // Well-known Luhn-valid numbers
        assert!(luhn_check("0"));
        assert!(luhn_check("79927398713"));
        assert!(luhn_check("4539578763621486"));
    }

    #[test]
    fn test_luhn_check_invalid_numbers() {
        assert!(!luhn_check("1234567890"));
        assert!(!luhn_check("79927398710"));
    }

    #[test]
    fn test_luhn_check_empty_and_non_digit() {
        assert!(!luhn_check(""));
        assert!(!luhn_check("abcdef"));
    }

    #[test]
    fn test_luhn_check_strips_non_digits() {
        // "7992-7398-713" should be treated as "79927398713" which is valid
        assert!(luhn_check("7992-7398-713"));
    }

    #[test]
    fn test_luhn_generate_produces_valid_number() {
        let result = luhn_generate("7992739871").unwrap();
        assert!(luhn_check(&result));
    }

    #[test]
    fn test_luhn_generate_single_digit_prefix() {
        let result = luhn_generate("4").unwrap();
        assert!(luhn_check(&result));
        assert!(result.starts_with('4'));
    }

    #[test]
    fn test_luhn_generate_empty_prefix_errors() {
        assert!(luhn_generate("").is_err());
    }

    #[test]
    fn test_luhn_generate_roundtrip() {
        for prefix in &["1", "41", "411", "4111", "123456789012345"] {
            let generated = luhn_generate(prefix).unwrap();
            assert!(
                luhn_check(&generated),
                "Generated number {} from prefix {} should be valid",
                generated,
                prefix
            );
        }
    }
}
