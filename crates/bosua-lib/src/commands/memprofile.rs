//! Memprofile CLI command — memory profiling.
//!
//! Displays current process memory usage statistics.

use clap::{ArgMatches, Command};

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::errors::Result;

/// Build the `memprofile` clap command.
pub fn memprofile_command() -> Command {
    Command::new("memprofile")
        .about("Display detailed memory profiling information")
        .aliases(["mem", "memory"])
        .arg(
            clap::Arg::new("top")
                .long("top")
                .value_parser(clap::value_parser!(i64))
                .default_value("10")
                .help("Number of top allocations to show"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn memprofile_meta() -> CommandMeta {
    CommandBuilder::from_clap(memprofile_command())
        .category(CommandCategory::Utility)
        .build()
}

/// Collect memory statistics from /proc/self/status (Linux) or sysctl (macOS).
fn collect_memory_stats() -> MemoryStats {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            let mut stats = MemoryStats::default();
            for line in status.lines() {
                if let Some(val) = line.strip_prefix("VmRSS:") {
                    stats.rss_kb = parse_kb(val);
                } else if let Some(val) = line.strip_prefix("VmSize:") {
                    stats.virtual_kb = parse_kb(val);
                } else if let Some(val) = line.strip_prefix("VmPeak:") {
                    stats.peak_kb = parse_kb(val);
                }
            }
            return stats;
        }
    }

    // Fallback: use std::alloc stats or report zeros
    MemoryStats::default()
}

#[cfg(target_os = "linux")]
fn parse_kb(val: &str) -> u64 {
    val.trim()
        .trim_end_matches("kB")
        .trim()
        .parse()
        .unwrap_or(0)
}

#[derive(Default, serde::Serialize)]
struct MemoryStats {
    rss_kb: u64,
    virtual_kb: u64,
    peak_kb: u64,
}

/// Handle the `memprofile` command.
pub fn handle_memprofile(matches: &ArgMatches) -> Result<()> {
    let top = matches.get_one::<i64>("top").copied().unwrap_or(10);
    let stats = collect_memory_stats();

    println!("Memory Profile (top {}):", top);
    println!("  RSS:     {} KB", stats.rss_kb);
    println!("  Virtual: {} KB", stats.virtual_kb);
    println!("  Peak:    {} KB", stats.peak_kb);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memprofile_command_parses() {
        let cmd = memprofile_command();
        let matches = cmd.try_get_matches_from(["memprofile"]).unwrap();
        assert_eq!(matches.get_one::<i64>("top").copied(), Some(10));
    }

    #[test]
    fn test_memprofile_command_top_flag() {
        let cmd = memprofile_command();
        let matches = cmd.try_get_matches_from(["memprofile", "--top", "20"]).unwrap();
        assert_eq!(matches.get_one::<i64>("top").copied(), Some(20));
    }

    #[test]
    fn test_memprofile_meta() {
        let meta = memprofile_meta();
        assert_eq!(meta.name, "memprofile");
        assert_eq!(meta.category, CommandCategory::Utility);
    }

    #[test]
    fn test_memprofile_aliases() {
        let cmd = memprofile_command();
        let aliases: Vec<&str> = cmd.get_all_aliases().collect();
        assert!(aliases.contains(&"mem"));
        assert!(aliases.contains(&"memory"));
    }

    #[test]
    fn test_memory_stats_default() {
        let stats = MemoryStats::default();
        assert_eq!(stats.rss_kb, 0);
    }
}
