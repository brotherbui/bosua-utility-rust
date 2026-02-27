/// Sanitize input by removing path traversal sequences, null bytes, and injection patterns.
///
/// Specifically neutralizes:
/// - `../` (Unix path traversal)
/// - `..\` (Windows path traversal)
/// - Null bytes (`\0`)
/// - Newlines (`\n`, `\r`) replaced with spaces to prevent header injection
pub fn sanitize_input(input: &str) -> String {
    input
        .replace("../", "")
        .replace("..\\", "")
        .replace('\0', "")
        .replace('\n', " ")
        .replace('\r', " ")
}

/// Check if a path contains traversal sequences or null bytes.
pub fn has_path_traversal(input: &str) -> bool {
    input.contains("../") || input.contains("..\\") || input.contains('\0')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_removes_unix_traversal() {
        assert_eq!(sanitize_input("../../etc/passwd"), "etc/passwd");
        assert_eq!(sanitize_input("foo/../bar"), "foo/bar");
    }

    #[test]
    fn sanitize_removes_windows_traversal() {
        assert_eq!(sanitize_input("..\\..\\windows\\system32"), "windows\\system32");
        assert_eq!(sanitize_input("foo\\..\\bar"), "foo\\bar");
    }

    #[test]
    fn sanitize_removes_null_bytes() {
        assert_eq!(sanitize_input("file\0name.txt"), "filename.txt");
        assert_eq!(sanitize_input("\0\0\0"), "");
    }

    #[test]
    fn sanitize_replaces_newlines_with_spaces() {
        assert_eq!(sanitize_input("line1\nline2"), "line1 line2");
        assert_eq!(sanitize_input("line1\rline2"), "line1 line2");
        assert_eq!(sanitize_input("line1\r\nline2"), "line1  line2");
    }

    #[test]
    fn sanitize_clean_input_unchanged() {
        assert_eq!(sanitize_input("normal-file.txt"), "normal-file.txt");
        assert_eq!(sanitize_input("path/to/file"), "path/to/file");
        assert_eq!(sanitize_input(""), "");
    }

    #[test]
    fn sanitize_combined_patterns() {
        assert_eq!(
            sanitize_input("../foo\0bar\n..\\baz"),
            "foobar baz"
        );
    }

    #[test]
    fn has_traversal_detects_unix() {
        assert!(has_path_traversal("../../etc/passwd"));
        assert!(has_path_traversal("foo/../bar"));
    }

    #[test]
    fn has_traversal_detects_windows() {
        assert!(has_path_traversal("..\\windows\\system32"));
    }

    #[test]
    fn has_traversal_detects_null_bytes() {
        assert!(has_path_traversal("file\0name"));
    }

    #[test]
    fn has_traversal_clean_input() {
        assert!(!has_path_traversal("normal/path/file.txt"));
        assert!(!has_path_traversal(""));
        assert!(!has_path_traversal("just-a-file.txt"));
    }
}
