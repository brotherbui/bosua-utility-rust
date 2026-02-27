use indicatif::{ProgressBar, ProgressStyle};

/// Create a progress bar for download operations with bytes, speed, and ETA display.
pub fn create_download_progress(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );
    pb
}

/// Create a spinner for indeterminate operations.
pub fn create_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_message(msg.to_string());
    pb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_download_progress() {
        let pb = create_download_progress(1024);
        assert_eq!(pb.length(), Some(1024));
        pb.finish_and_clear();
    }

    #[test]
    fn test_create_download_progress_zero() {
        let pb = create_download_progress(0);
        assert_eq!(pb.length(), Some(0));
        pb.finish_and_clear();
    }

    #[test]
    fn test_create_spinner() {
        let pb = create_spinner("Loading...");
        assert_eq!(pb.message(), "Loading...");
        pb.finish_and_clear();
    }

    #[test]
    fn test_create_spinner_empty_message() {
        let pb = create_spinner("");
        assert_eq!(pb.message(), "");
        pb.finish_and_clear();
    }
}
