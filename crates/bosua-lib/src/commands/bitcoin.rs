//! Bitcoin CLI command — Bitcoin address and Luhn validation.
//!
//! Subcommands: validate, luhn.

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::Result;

/// Build the `bitcoin` clap command.
pub fn bitcoin_command() -> Command {
    Command::new("bitcoin")
        .about("Bitcoin address and Luhn validation")
        .subcommand(
            Command::new("validate")
                .about("Validate a Bitcoin address")
                .arg(
                    Arg::new("address")
                        .required(true)
                        .help("Bitcoin address to validate"),
                ),
        )
        .subcommand(
            Command::new("luhn")
                .about("Luhn checksum validation")
                .arg(
                    Arg::new("number")
                        .required(true)
                        .help("Number to validate with Luhn algorithm"),
                ),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn bitcoin_meta() -> CommandMeta {
    CommandBuilder::from_clap(bitcoin_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Validate a Bitcoin address (basic format check).
/// Supports legacy (1...), P2SH (3...), and Bech32 (bc1...) formats.
fn validate_bitcoin_address(address: &str) -> bool {
    if address.is_empty() {
        return false;
    }

    // Bech32 addresses (bc1...)
    if address.starts_with("bc1") {
        return address.len() >= 14
            && address.len() <= 74
            && address[3..].chars().all(|c| {
                matches!(c, 'q' | 'p' | 'z' | 'r' | 'y' | '9' | 'x' | '8'
                    | 'g' | 'f' | '2' | 't' | 'v' | 'd' | 'w' | '0'
                    | 's' | '3' | 'j' | 'n' | '5' | '4' | 'k' | 'h'
                    | 'c' | 'e' | '6' | 'm' | 'u' | 'a' | '7' | 'l')
            });
    }

    // Legacy (1...) and P2SH (3...) addresses
    if address.starts_with('1') || address.starts_with('3') {
        return address.len() >= 25
            && address.len() <= 34
            && address.chars().all(|c| {
                c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l'
            });
    }

    false
}

/// Luhn algorithm check.
fn luhn_check(number: &str) -> bool {
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

/// Handle the `bitcoin` command.
pub fn handle_bitcoin(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("validate", sub)) => {
            let address = sub.get_one::<String>("address").unwrap();
            if validate_bitcoin_address(address) {
                println!("{}: valid", address);
            } else {
                println!("{}: invalid", address);
            }
            Ok(())
        }
        Some(("luhn", sub)) => {
            let number = sub.get_one::<String>("number").unwrap();
            if luhn_check(number) {
                println!("{}: valid (Luhn)", number);
            } else {
                println!("{}: invalid (Luhn)", number);
            }
            Ok(())
        }
        _ => {
            println!("bitcoin: use a subcommand (validate, luhn)");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitcoin_command_parses_validate() {
        let cmd = bitcoin_command();
        let matches = cmd
            .try_get_matches_from(["bitcoin", "validate", "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "validate");
        assert_eq!(
            sub.get_one::<String>("address").map(|s| s.as_str()),
            Some("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"),
        );
    }

    #[test]
    fn test_bitcoin_command_parses_luhn() {
        let cmd = bitcoin_command();
        let matches = cmd
            .try_get_matches_from(["bitcoin", "luhn", "4539578763621486"])
            .unwrap();
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "luhn");
        assert_eq!(
            sub.get_one::<String>("number").map(|s| s.as_str()),
            Some("4539578763621486"),
        );
    }

    #[test]
    fn test_bitcoin_validate_requires_address() {
        let cmd = bitcoin_command();
        let result = cmd.try_get_matches_from(["bitcoin", "validate"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_bitcoin_meta() {
        let meta = bitcoin_meta();
        assert_eq!(meta.name, "bitcoin");
        assert_eq!(meta.category, CommandCategory::Utility);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_validate_bitcoin_legacy() {
        assert!(validate_bitcoin_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"));
    }

    #[test]
    fn test_validate_bitcoin_bech32() {
        assert!(validate_bitcoin_address("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4"));
    }

    #[test]
    fn test_validate_bitcoin_invalid() {
        assert!(!validate_bitcoin_address(""));
        assert!(!validate_bitcoin_address("not-an-address"));
    }

    #[test]
    fn test_luhn_valid() {
        assert!(luhn_check("4539578763621486"));
        assert!(luhn_check("79927398713"));
    }

    #[test]
    fn test_luhn_invalid() {
        assert!(!luhn_check("1234567890"));
        assert!(!luhn_check(""));
    }
}
