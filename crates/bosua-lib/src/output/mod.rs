pub mod progress;

use crossterm::style::{Color, Stylize};

/// Print a success message in green to stdout.
pub fn success(msg: &str) {
    println!("{}", msg.with(Color::Green));
}

/// Print an error message in red to stderr.
pub fn error(msg: &str) {
    eprintln!("{}", msg.with(Color::Red));
}

/// Print a warning message in yellow to stderr.
pub fn warning(msg: &str) {
    eprintln!("{}", msg.with(Color::Yellow));
}

/// Print an info message in cyan to stdout.
pub fn info(msg: &str) {
    println!("{}", msg.with(Color::Cyan));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_does_not_panic() {
        success("Operation completed");
    }

    #[test]
    fn test_error_does_not_panic() {
        error("Something went wrong");
    }

    #[test]
    fn test_warning_does_not_panic() {
        warning("Careful now");
    }

    #[test]
    fn test_info_does_not_panic() {
        info("FYI");
    }
}
