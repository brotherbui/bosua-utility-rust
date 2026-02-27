//! SFTP utilities for file transfer operations and external tool execution.
//!
//! Provides an `SftpClient` for uploading/downloading files via SFTP and a
//! helper for invoking external system commands (ffmpeg, aria2c, etc.) through
//! `tokio::process::Command`.

use std::path::{Path, PathBuf};

use tokio::process::Command;
use tracing;

use crate::errors::{BosuaError, Result};

/// Result of a remote command execution.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Process exit code (0 = success).
    pub exit_code: i32,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
}

/// Configuration for an SFTP connection.
#[derive(Debug, Clone)]
pub struct SftpConfig {
    /// Remote host (IP or hostname).
    pub host: String,
    /// SSH port (default 22).
    pub port: u16,
    /// Username for authentication.
    pub username: String,
    /// Optional path to an SSH private key file.
    pub key_file: Option<PathBuf>,
}

impl Default for SftpConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 22,
            username: String::new(),
            key_file: None,
        }
    }
}

/// SFTP client for file transfer operations.
///
/// Uses the system `sftp` / `scp` commands under the hood, mirroring the Go
/// implementation which shells out to external tools rather than embedding a
/// full SSH library.
#[derive(Debug, Clone)]
pub struct SftpClient {
    config: SftpConfig,
}

impl SftpClient {
    /// Create a new SFTP client with the given configuration.
    pub fn new(config: SftpConfig) -> Self {
        Self { config }
    }

    /// Upload a local file to a remote path via `scp`.
    pub async fn upload(&self, local_path: &Path, remote_path: &str) -> Result<CommandOutput> {
        if !local_path.exists() {
            return Err(BosuaError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("local file not found: {}", local_path.display()),
            )));
        }

        let dest = format!("{}@{}:{}", self.config.username, self.config.host, remote_path);
        let mut args: Vec<String> = Vec::new();
        args.push("-P".into());
        args.push(self.config.port.to_string());
        if let Some(ref key) = self.config.key_file {
            args.push("-i".into());
            args.push(key.display().to_string());
        }
        args.push(local_path.display().to_string());
        args.push(dest);

        tracing::debug!(local = %local_path.display(), remote = remote_path, "sftp upload");
        execute_command("scp", &args).await
    }

    /// Download a remote file to a local path via `scp`.
    pub async fn download(&self, remote_path: &str, local_path: &Path) -> Result<CommandOutput> {
        let src = format!("{}@{}:{}", self.config.username, self.config.host, remote_path);
        let mut args: Vec<String> = Vec::new();
        args.push("-P".into());
        args.push(self.config.port.to_string());
        if let Some(ref key) = self.config.key_file {
            args.push("-i".into());
            args.push(key.display().to_string());
        }
        args.push(src);
        args.push(local_path.display().to_string());

        tracing::debug!(remote = remote_path, local = %local_path.display(), "sftp download");
        execute_command("scp", &args).await
    }

    /// List files in a remote directory via `ssh ls`.
    pub async fn list(&self, remote_dir: &str) -> Result<Vec<String>> {
        let mut args: Vec<String> = Vec::new();
        args.push("-p".into());
        args.push(self.config.port.to_string());
        if let Some(ref key) = self.config.key_file {
            args.push("-i".into());
            args.push(key.display().to_string());
        }
        let target = format!("{}@{}", self.config.username, self.config.host);
        args.push(target);
        args.push(format!("ls -1 {}", remote_dir));

        let output = execute_command("ssh", &args).await?;
        if output.exit_code != 0 {
            return Err(BosuaError::Command(format!(
                "remote ls failed (exit {}): {}",
                output.exit_code, output.stderr
            )));
        }
        Ok(output
            .stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect())
    }
}

/// Execute an external system command and capture its output.
///
/// This is the primary mechanism for invoking tools like `ffmpeg`, `aria2c`,
/// `scp`, `ssh`, etc.
pub async fn execute_command(program: &str, args: &[String]) -> Result<CommandOutput> {
    tracing::debug!(program, ?args, "executing command");

    let output = Command::new(program)
        .args(args)
        .output()
        .await
        .map_err(|e| {
            BosuaError::Command(format!("failed to execute '{}': {}", program, e))
        })?;

    let result = CommandOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    };

    if !output.status.success() {
        tracing::warn!(
            program,
            exit_code = result.exit_code,
            stderr = %result.stderr.trim(),
            "command exited with non-zero status"
        );
    }

    Ok(result)
}

/// Execute ffmpeg with the given arguments.
pub async fn run_ffmpeg(args: &[String]) -> Result<CommandOutput> {
    execute_command("ffmpeg", args).await
}

/// Execute aria2c with the given arguments.
pub async fn run_aria2c(args: &[String]) -> Result<CommandOutput> {
    execute_command("aria2c", args).await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sftp_config_default() {
        let cfg = SftpConfig::default();
        assert!(cfg.host.is_empty());
        assert_eq!(cfg.port, 22);
        assert!(cfg.username.is_empty());
        assert!(cfg.key_file.is_none());
    }

    #[test]
    fn sftp_client_creation() {
        let cfg = SftpConfig {
            host: "example.com".into(),
            port: 2222,
            username: "user".into(),
            key_file: Some(PathBuf::from("/home/user/.ssh/id_rsa")),
        };
        let client = SftpClient::new(cfg.clone());
        assert_eq!(client.config.host, "example.com");
        assert_eq!(client.config.port, 2222);
        assert_eq!(client.config.username, "user");
        assert!(client.config.key_file.is_some());
    }

    #[tokio::test]
    async fn upload_nonexistent_file_returns_error() {
        let client = SftpClient::new(SftpConfig {
            host: "localhost".into(),
            username: "test".into(),
            ..Default::default()
        });
        let result = client
            .upload(Path::new("/nonexistent/file.txt"), "/remote/file.txt")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn execute_command_echo() {
        let output = execute_command("echo", &["hello".into()]).await;
        assert!(output.is_ok());
        let out = output.unwrap();
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("hello"));
    }

    #[tokio::test]
    async fn execute_command_nonexistent_program() {
        let result = execute_command("nonexistent_program_xyz_123", &[]).await;
        assert!(result.is_err());
    }

    #[test]
    fn command_output_debug() {
        let out = CommandOutput {
            exit_code: 0,
            stdout: "ok".into(),
            stderr: String::new(),
        };
        let dbg = format!("{:?}", out);
        assert!(dbg.contains("exit_code: 0"));
    }
}
