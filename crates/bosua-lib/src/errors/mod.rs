use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BosuaError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Server error ({status}): {message}")]
    Server { status: u16, message: String },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Cloud error ({service}): {message}")]
    Cloud { service: String, message: String },

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("OAuth2 error: {0}")]
    OAuth2(String),

    #[error("Download error: {0}")]
    Download(String),

    #[error("Lock conflict: {path}")]
    LockConflict { path: PathBuf },

    #[error("Command error: {0}")]
    Command(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("{0}")]
    Application(String),

    #[error("Panic recovered: {0}")]
    Panic(String),
}

pub type Result<T> = std::result::Result<T, BosuaError>;

/// Wraps a closure, catching panics and converting them to `BosuaError::Panic`.
///
/// If the closure panics, the panic payload is extracted as a string message.
/// If the closure returns normally, its result is passed through unchanged.
pub fn safe_run<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + std::panic::UnwindSafe,
{
    match std::panic::catch_unwind(f) {
        Ok(result) => result,
        Err(panic_info) => {
            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };
            Err(BosuaError::Panic(msg))
        }
    }
}

/// Logs a fatal error and exits the process with code 1.
///
/// This function never returns (`-> !`). It is intended for unrecoverable
/// errors during initialization or command execution.
pub fn handle_fatal(err: BosuaError) -> ! {
    tracing::error!("Fatal error: {}", err);
    std::process::exit(1)
}

/// Maps a `BosuaError` to user-friendly CLI output with actionable suggestions.
///
/// Uses `crate::output::error()` for the main error message and
/// `crate::output::info()` for hints and suggestions.
pub fn handle_command_error(err: &BosuaError) {
    use crate::output;

    match err {
        BosuaError::Auth(msg) => {
            output::error(&format!("Authentication error: {}", msg));
            if msg.contains("GDrive") || msg.contains("OAuth2") {
                output::info("Run `bosua gdrive oauth2 login` to authenticate.");
            } else if msg.contains("FShare") {
                output::info("Run `bosua fshare account login` to authenticate.");
            } else if msg.contains("AWS") {
                output::info("Check your AWS credentials configuration.");
            } else if msg.contains("Cloudflare") {
                output::info("Run `bosua cloudflare account set-token` to set your API token.");
            } else if msg.contains("Tailscale") {
                output::info("Run `bosua tailscale account set-key` to set your API key.");
            }
        }
        BosuaError::OAuth2(msg) => {
            output::error(&format!("OAuth2 error: {}", msg));
            output::info("Run `bosua gdrive oauth2 login` to re-authenticate.");
        }
        BosuaError::Cloud { service, message } => {
            output::error(&format!("{} error: {}", service, message));
            output::info("Check your network connection and service credentials.");
        }
        BosuaError::Http(e) => {
            output::error(&format!("Network error: {}", e));
            output::info("Check your internet connection.");
        }
        BosuaError::Io(e) => {
            output::error(&format!("File error: {}", e));
        }
        BosuaError::Config(msg) => {
            output::error(&format!("Configuration error: {}", msg));
        }
        BosuaError::LockConflict { path } => {
            output::error(&format!("Lock conflict: {}", path.display()));
            output::info("Another process may be using this resource. Wait and try again.");
        }
        BosuaError::Command(msg) => {
            output::error(&format!("Error: {}", msg));
        }
        _ => {
            output::error(&format!("{}", err));
        }
    }
}

