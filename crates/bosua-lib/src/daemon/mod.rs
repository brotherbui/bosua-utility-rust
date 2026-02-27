//! Daemon management with systemd integration and worker abstractions.
//!
//! Provides `DaemonManager` for controlling the Bosua daemon service on Linux
//! via systemd, along with `DaemonConfig` and worker function utilities for
//! parallel task execution.

pub mod cron;

use std::fmt;
use std::process::Command as ProcessCommand;

use crate::errors::{BosuaError, Result};

// ---------------------------------------------------------------------------
// DaemonStatus
// ---------------------------------------------------------------------------

/// Status of the daemon service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonStatus {
    Running,
    Stopped,
    Unknown,
}

impl fmt::Display for DaemonStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

// ---------------------------------------------------------------------------
// DaemonConfig
// ---------------------------------------------------------------------------

/// Configuration for the daemon service.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Systemd service name.
    pub service_name: String,
    /// Working directory for the daemon.
    pub working_dir: Option<String>,
    /// Number of worker threads for parallel task execution.
    pub worker_count: usize,
    /// Whether to restart on failure.
    pub restart_on_failure: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            service_name: "bosua".to_string(),
            working_dir: None,
            worker_count: 4,
            restart_on_failure: true,
        }
    }
}

// ---------------------------------------------------------------------------
// WorkerHandle
// ---------------------------------------------------------------------------

/// Handle for a spawned worker task.
pub struct WorkerHandle {
    pub name: String,
    handle: Option<std::thread::JoinHandle<Result<()>>>,
}

impl WorkerHandle {
    /// Wait for the worker to complete.
    pub fn join(mut self) -> Result<()> {
        if let Some(h) = self.handle.take() {
            h.join()
                .map_err(|_| BosuaError::Application(format!("Worker '{}' panicked", self.name)))?
        } else {
            Ok(())
        }
    }
}

/// Spawn a worker function on a new thread.
pub fn spawn_worker<F>(name: impl Into<String>, f: F) -> WorkerHandle
where
    F: FnOnce() -> Result<()> + Send + 'static,
{
    let name = name.into();
    let thread_name = name.clone();
    let handle = std::thread::Builder::new()
        .name(thread_name)
        .spawn(f)
        .expect("failed to spawn worker thread");
    WorkerHandle {
        name,
        handle: Some(handle),
    }
}

/// Spawn multiple worker functions and collect their handles.
pub fn spawn_workers<F>(count: usize, name_prefix: &str, factory: F) -> Vec<WorkerHandle>
where
    F: Fn(usize) -> Box<dyn FnOnce() -> Result<()> + Send> + 'static,
{
    (0..count)
        .map(|i| {
            let name = format!("{}-{}", name_prefix, i);
            let work = factory(i);
            spawn_worker(name, work)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// DaemonManager
// ---------------------------------------------------------------------------

/// Manages the Bosua daemon service via systemd on Linux.
pub struct DaemonManager {
    config: DaemonConfig,
}

impl DaemonManager {
    /// Create a new `DaemonManager` with the given configuration.
    pub fn new(config: DaemonConfig) -> Self {
        Self { config }
    }

    /// Create a `DaemonManager` with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(DaemonConfig::default())
    }

    /// Start the daemon service.
    pub fn start(&self) -> Result<()> {
        self.systemctl("start")
    }

    /// Stop the daemon service.
    pub fn stop(&self) -> Result<()> {
        self.systemctl("stop")
    }

    /// Restart the daemon service.
    pub fn restart(&self) -> Result<()> {
        self.systemctl("restart")
    }

    /// Query the current daemon status.
    pub fn status(&self) -> Result<DaemonStatus> {
        let output = ProcessCommand::new("systemctl")
            .args(["is-active", &self.config.service_name])
            .output()
            .map_err(|e| {
                BosuaError::Application(format!("Failed to query systemctl: {}", e))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        match stdout.as_str() {
            "active" => Ok(DaemonStatus::Running),
            "inactive" | "failed" | "dead" => Ok(DaemonStatus::Stopped),
            _ => Ok(DaemonStatus::Unknown),
        }
    }

    /// Retrieve recent daemon logs.
    pub fn logs(&self, lines: usize) -> Result<String> {
        let output = ProcessCommand::new("journalctl")
            .args([
                "-u",
                &self.config.service_name,
                "-n",
                &lines.to_string(),
                "--no-pager",
            ])
            .output()
            .map_err(|e| {
                BosuaError::Application(format!("Failed to read journalctl: {}", e))
            })?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get the current daemon configuration.
    pub fn get_config(&self) -> &DaemonConfig {
        &self.config
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn systemctl(&self, action: &str) -> Result<()> {
        let output = ProcessCommand::new("systemctl")
            .args([action, &self.config.service_name])
            .output()
            .map_err(|e| {
                BosuaError::Application(format!(
                    "Failed to {} daemon via systemctl: {}",
                    action, e
                ))
            })?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(BosuaError::Application(format!(
                "systemctl {} failed: {}",
                action,
                stderr.trim()
            )))
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_status_display() {
        assert_eq!(DaemonStatus::Running.to_string(), "running");
        assert_eq!(DaemonStatus::Stopped.to_string(), "stopped");
        assert_eq!(DaemonStatus::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_daemon_config_default() {
        let config = DaemonConfig::default();
        assert_eq!(config.service_name, "bosua");
        assert!(config.working_dir.is_none());
        assert_eq!(config.worker_count, 4);
        assert!(config.restart_on_failure);
    }

    #[test]
    fn test_daemon_manager_creation() {
        let manager = DaemonManager::with_defaults();
        assert_eq!(manager.get_config().service_name, "bosua");
    }

    #[test]
    fn test_daemon_manager_custom_config() {
        let config = DaemonConfig {
            service_name: "bosua-test".to_string(),
            working_dir: Some("/opt/bosua".to_string()),
            worker_count: 8,
            restart_on_failure: false,
        };
        let manager = DaemonManager::new(config);
        assert_eq!(manager.get_config().service_name, "bosua-test");
        assert_eq!(manager.get_config().worker_count, 8);
        assert!(!manager.get_config().restart_on_failure);
    }

    #[test]
    fn test_spawn_worker() {
        let handle = spawn_worker("test-worker", || Ok(()));
        assert_eq!(handle.name, "test-worker");
        assert!(handle.join().is_ok());
    }

    #[test]
    fn test_spawn_worker_with_error() {
        let handle = spawn_worker("err-worker", || {
            Err(BosuaError::Application("test error".to_string()))
        });
        assert!(handle.join().is_err());
    }

    #[test]
    fn test_spawn_workers() {
        let handles = spawn_workers(3, "pool", |i| {
            Box::new(move || {
                let _ = i;
                Ok(())
            })
        });
        assert_eq!(handles.len(), 3);
        for h in handles {
            assert!(h.join().is_ok());
        }
    }
}
