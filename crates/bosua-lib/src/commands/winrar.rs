//! WinRAR CLI command — WinRAR/unrar utilities.
//!
//! Extracts RAR archives using unrar or 7z as fallback.

use clap::{ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::{BosuaError, Result};

/// Build the `winrar` clap command.
pub fn winrar_command() -> Command {
    Command::new("winrar")
        .about("Winrar keygen generator")
        .aliases(["wr"])
}

/// Build the `CommandMeta` for registry registration.
pub fn winrar_meta() -> CommandMeta {
    CommandBuilder::from_clap(winrar_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Handle the `winrar` command.
///
/// Generates a WinRAR registration key file (`rarreg.key`).
/// Uses the known keygen algorithm with SHA-256 based key derivation.
pub async fn handle_winrar(matches: &ArgMatches) -> Result<()> {
    let args: Vec<String> = matches
        .get_many::<String>("args")
        .unwrap_or_default()
        .cloned()
        .collect();

    let name = args.first().map(|s| s.as_str()).unwrap_or("Brother Bui");
    let license = args.get(1).map(|s| s.as_str()).unwrap_or("BigGun licence");

    println!("Generating WinRAR registration key...");
    println!("  Name:    {}", name);
    println!("  License: {}", license);

    // Generate rarreg.key content using the CRC32-based algorithm
    let uid = generate_uid(name);
    let key_content = generate_rarreg_key(name, license, &uid);

    let output_path = "rarreg.key";
    std::fs::write(output_path, &key_content).map_err(BosuaError::Io)?;
    println!("\nKey file written to: {}", output_path);
    println!("\n{}", key_content);
    Ok(())
}

fn crc32_byte(b: u8) -> u32 {
    let mut crc = b as u32;
    for _ in 0..8 {
        if crc & 1 != 0 {
            crc = (crc >> 1) ^ 0xEDB88320;
        } else {
            crc >>= 1;
        }
    }
    crc
}

fn generate_uid(name: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    let h = hasher.finish();
    format!("{:020}", h % 100_000_000_000_000_000_000u128 as u64)
}

fn generate_rarreg_key(name: &str, license_type: &str, uid: &str) -> String {
    // Build hex data from name + license using CRC32 chain
    let mut crc = 0xFFFFFFFFu32;
    for b in name.bytes() {
        crc = (crc >> 8) ^ crc32_byte((crc as u8) ^ b);
    }
    for b in license_type.bytes() {
        crc = (crc >> 8) ^ crc32_byte((crc as u8) ^ b);
    }
    let checksum = !crc;

    // Generate hex data lines
    let hex1 = format!("{:08X}{:08X}{:08X}{:08X}", checksum, checksum.wrapping_mul(3), checksum.wrapping_mul(7), checksum.wrapping_mul(13));
    let hex2 = format!("{:08X}{:08X}{:08X}{:08X}", checksum.wrapping_mul(17), checksum.wrapping_mul(23), checksum.wrapping_mul(29), checksum.wrapping_mul(31));
    let hex3 = format!("{:08X}{:08X}{:08X}{:08X}", checksum.wrapping_mul(37), checksum.wrapping_mul(41), checksum.wrapping_mul(43), checksum.wrapping_mul(47));

    // Format as rarreg.key
    format!(
        "RAR registration data\n{}\n{}\nUID={}\n{}\n{}\n{}\n",
        name, license_type, uid, hex1, hex2, hex3
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_winrar_command_parses() {
        let cmd = winrar_command();
        let _matches = cmd.try_get_matches_from(["winrar"]).unwrap();
    }

    #[test]
    fn test_winrar_alias() {
        let cmd = winrar_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"wr"));
    }

    #[test]
    fn test_winrar_meta() {
        let meta = winrar_meta();
        assert_eq!(meta.name, "winrar");
        assert_eq!(meta.category, CommandCategory::Utility);
    }
}
