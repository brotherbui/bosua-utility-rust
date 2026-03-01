use unicode_normalization::UnicodeNormalization;

/// Normalize text to Unicode NFC form and remove diacritical marks (accents).
pub fn normalize(input: &str) -> String {
    input
        .nfd()
        .filter(|c| !('\u{0300}'..='\u{036F}').contains(c))
        .nfc()
        .collect()
}

/// Sanitize a string for use as a filename.
/// Keeps only alphanumeric, hyphens, underscores, dots, and spaces.
/// Replaces path separators and other unsafe chars with underscores.
pub fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Format a byte count as a human-readable string (B, KB, MB, GB, TB).
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

/// Format a duration in seconds as a human-readable string (e.g., "1h 23m 45s").
pub fn format_duration(seconds: u64) -> String {
    if seconds == 0 {
        return "0s".to_string();
    }
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 {
        parts.push(format!("{}m", minutes));
    }
    if secs > 0 {
        parts.push(format!("{}s", secs));
    }
    parts.join(" ")
}
/// Deobfuscate a string by picking characters at the given byte indices.
///
/// Matches Go's `utils.Deobfs()` — extracts characters from `input` at each
/// position in `indices` and concatenates them.
pub fn deobfs(input: &str, indices: &[usize]) -> String {
    let bytes = input.as_bytes();
    indices
        .iter()
        .filter_map(|&i| {
            if i < bytes.len() {
                Some(bytes[i] as char)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- normalize ---

    #[test]
    fn normalize_plain_ascii() {
        assert_eq!(normalize("hello world"), "hello world");
    }

    #[test]
    fn normalize_removes_accents() {
        assert_eq!(normalize("café"), "cafe");
        assert_eq!(normalize("naïve"), "naive");
        assert_eq!(normalize("résumé"), "resume");
    }

    #[test]
    fn normalize_handles_combined_characters() {
        // Pre-composed é (U+00E9) should become e after NFD decomposition + accent removal
        assert_eq!(normalize("\u{00E9}"), "e");
    }

    #[test]
    fn normalize_preserves_non_latin() {
        // CJK characters should pass through unchanged
        assert_eq!(normalize("日本語"), "日本語");
    }

    #[test]
    fn normalize_empty_string() {
        assert_eq!(normalize(""), "");
    }

    #[test]
    fn normalize_idempotent() {
        let input = "Ångström café naïve";
        let once = normalize(input);
        let twice = normalize(&once);
        assert_eq!(once, twice);
    }

    // --- sanitize_filename ---

    #[test]
    fn sanitize_filename_keeps_safe_chars() {
        assert_eq!(sanitize_filename("hello-world_v2.txt"), "hello-world_v2.txt");
    }

    #[test]
    fn sanitize_filename_replaces_spaces() {
        assert_eq!(sanitize_filename("my file.txt"), "my_file.txt");
    }

    #[test]
    fn sanitize_filename_replaces_path_separators() {
        assert_eq!(sanitize_filename("path/to\\file"), "path_to_file");
    }

    #[test]
    fn sanitize_filename_replaces_special_chars() {
        assert_eq!(sanitize_filename("file<>:\"|?*name"), "file_______name");
    }

    #[test]
    fn sanitize_filename_empty_string() {
        assert_eq!(sanitize_filename(""), "");
    }

    // --- format_size ---

    #[test]
    fn format_size_zero() {
        assert_eq!(format_size(0), "0 B");
    }

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1), "1 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
    }

    #[test]
    fn format_size_megabytes() {
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(5 * 1024 * 1024), "5.00 MB");
    }

    #[test]
    fn format_size_gigabytes() {
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn format_size_terabytes() {
        assert_eq!(format_size(1024u64 * 1024 * 1024 * 1024), "1.00 TB");
    }

    // --- format_duration ---

    #[test]
    fn format_duration_zero() {
        assert_eq!(format_duration(0), "0s");
    }

    #[test]
    fn format_duration_seconds_only() {
        assert_eq!(format_duration(45), "45s");
    }

    #[test]
    fn format_duration_minutes_and_seconds() {
        assert_eq!(format_duration(90), "1m 30s");
    }

    #[test]
    fn format_duration_hours_minutes_seconds() {
        assert_eq!(format_duration(3600 + 23 * 60 + 45), "1h 23m 45s");
    }

    #[test]
    fn format_duration_exact_hour() {
        assert_eq!(format_duration(3600), "1h");
    }

    #[test]
    fn format_duration_exact_minute() {
        assert_eq!(format_duration(60), "1m");
    }

    #[test]
    fn format_duration_hours_and_seconds_no_minutes() {
        assert_eq!(format_duration(3601), "1h 1s");
    }

    // --- deobfs ---

    #[test]
    fn deobfs_basic() {
        assert_eq!(deobfs("hello world", &[0, 6]), "hw");
    }

    #[test]
    fn deobfs_empty_indices() {
        assert_eq!(deobfs("hello", &[]), "");
    }

    #[test]
    fn deobfs_out_of_bounds_skipped() {
        assert_eq!(deobfs("abc", &[0, 100, 2]), "ac");
    }
}
