use std::sync::atomic::{AtomicBool, Ordering};
use tracing_subscriber::{fmt, EnvFilter};

static VERBOSE: AtomicBool = AtomicBool::new(false);

/// Initialize the tracing subscriber with timestamp, level, and structured fields.
/// If `debug` is true, sets the log level to DEBUG; otherwise INFO.
pub fn init(debug: bool) {
    let filter = if debug {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    fmt()
        .with_env_filter(filter)
        .with_timer(fmt::time::SystemTime)
        .with_level(true)
        .with_target(true)
        .init();
}

/// Set the global verbose mode flag.
pub fn set_verbose(enabled: bool) {
    VERBOSE.store(enabled, Ordering::SeqCst);
}

/// Check whether verbose mode is currently enabled.
pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verbose_set_and_get() {
        // Tests run in parallel sharing the global AtomicBool,
        // so we test the set/get round-trip in a single test.
        set_verbose(true);
        assert!(is_verbose());

        set_verbose(false);
        assert!(!is_verbose());

        // Toggle back and forth
        set_verbose(true);
        assert!(is_verbose());
        set_verbose(false);
        assert!(!is_verbose());
    }
}
