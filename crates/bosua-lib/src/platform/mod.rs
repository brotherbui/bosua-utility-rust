/// Platform detection and feature-gated module declarations.

/// Supported platforms for the Bosua CLI toolkit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOS,
    Linux,
    Windows,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::MacOS => write!(f, "macOS"),
            Platform::Linux => write!(f, "Linux"),
            Platform::Windows => write!(f, "Windows"),
        }
    }
}

/// Returns the platform detected at compile time.
pub fn current_platform() -> Platform {
    #[cfg(target_os = "macos")]
    {
        Platform::MacOS
    }
    #[cfg(target_os = "linux")]
    {
        Platform::Linux
    }
    #[cfg(target_os = "windows")]
    {
        Platform::Windows
    }
}

/// Returns `true` on Linux, where the HTTP server runs.
pub fn is_server_platform() -> bool {
    current_platform() == Platform::Linux
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_platform_returns_valid_variant() {
        let platform = current_platform();
        // On any supported OS, we should get one of the three variants
        assert!(
            platform == Platform::MacOS
                || platform == Platform::Linux
                || platform == Platform::Windows
        );
    }

    #[test]
    fn is_server_platform_matches_linux() {
        let expected = cfg!(target_os = "linux");
        assert_eq!(is_server_platform(), expected);
    }

    #[test]
    fn platform_display() {
        assert_eq!(Platform::MacOS.to_string(), "macOS");
        assert_eq!(Platform::Linux.to_string(), "Linux");
        assert_eq!(Platform::Windows.to_string(), "Windows");
    }

    #[test]
    fn platform_clone_and_eq() {
        let p = Platform::Linux;
        let p2 = p;
        assert_eq!(p, p2);
    }
}
