//! Cron job scheduling and management.
//!
//! Provides `CronManager` for scheduling automated tasks with support for
//! daemon mode (persistent execution).

use std::collections::HashMap;

use crate::errors::{BosuaError, Result};

// ---------------------------------------------------------------------------
// CronJob
// ---------------------------------------------------------------------------

/// A scheduled cron job.
#[derive(Debug, Clone)]
pub struct CronJob {
    /// Unique job name.
    pub name: String,
    /// Cron schedule expression (e.g. "0 */6 * * *").
    pub schedule: String,
    /// Command to execute when the job fires.
    pub command: String,
    /// Whether the job is enabled.
    pub enabled: bool,
}

impl CronJob {
    /// Create a new enabled cron job.
    pub fn new(name: impl Into<String>, schedule: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            schedule: schedule.into(),
            command: command.into(),
            enabled: true,
        }
    }
}

// ---------------------------------------------------------------------------
// CronManager
// ---------------------------------------------------------------------------

/// Manages cron job scheduling and execution.
pub struct CronManager {
    jobs: HashMap<String, CronJob>,
}

impl CronManager {
    /// Create a new empty `CronManager`.
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
        }
    }

    /// Add a cron job. Returns error if a job with the same name already exists.
    pub fn add_job(&mut self, job: CronJob) -> Result<()> {
        if self.jobs.contains_key(&job.name) {
            return Err(BosuaError::Application(format!(
                "Cron job '{}' already exists",
                job.name
            )));
        }
        self.jobs.insert(job.name.clone(), job);
        Ok(())
    }

    /// Remove a cron job by name. Returns error if the job does not exist.
    pub fn remove_job(&mut self, name: &str) -> Result<CronJob> {
        self.jobs.remove(name).ok_or_else(|| {
            BosuaError::Application(format!("Cron job '{}' not found", name))
        })
    }

    /// List all registered cron jobs.
    pub fn list_jobs(&self) -> Vec<&CronJob> {
        let mut jobs: Vec<&CronJob> = self.jobs.values().collect();
        jobs.sort_by(|a, b| a.name.cmp(&b.name));
        jobs
    }

    /// Run all enabled pending jobs. Returns the names of jobs that were executed.
    ///
    /// In a full implementation this would check each job's schedule against the
    /// current time. For now it collects enabled jobs and logs them.
    pub fn run_pending(&self) -> Vec<String> {
        self.jobs
            .values()
            .filter(|j| j.enabled)
            .map(|j| {
                // Stub: in production this would parse the cron expression,
                // check if it's due, and execute the command.
                println!("cron run_pending: job '{}' ({}): not yet implemented", j.name, j.schedule);
                j.name.clone()
            })
            .collect()
    }

    /// Number of registered jobs.
    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    /// Whether the manager has no jobs.
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }
}

impl Default for CronManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_job_new() {
        let job = CronJob::new("backup", "0 2 * * *", "bosua gdrive-sync");
        assert_eq!(job.name, "backup");
        assert_eq!(job.schedule, "0 2 * * *");
        assert_eq!(job.command, "bosua gdrive-sync");
        assert!(job.enabled);
    }

    #[test]
    fn test_cron_manager_add_job() {
        let mut mgr = CronManager::new();
        let job = CronJob::new("test", "* * * * *", "echo hello");
        assert!(mgr.add_job(job).is_ok());
        assert_eq!(mgr.len(), 1);
    }

    #[test]
    fn test_cron_manager_add_duplicate_fails() {
        let mut mgr = CronManager::new();
        mgr.add_job(CronJob::new("test", "* * * * *", "echo 1")).unwrap();
        let result = mgr.add_job(CronJob::new("test", "0 * * * *", "echo 2"));
        assert!(result.is_err());
    }

    #[test]
    fn test_cron_manager_remove_job() {
        let mut mgr = CronManager::new();
        mgr.add_job(CronJob::new("test", "* * * * *", "echo hello")).unwrap();
        let removed = mgr.remove_job("test").unwrap();
        assert_eq!(removed.name, "test");
        assert!(mgr.is_empty());
    }

    #[test]
    fn test_cron_manager_remove_nonexistent_fails() {
        let mut mgr = CronManager::new();
        assert!(mgr.remove_job("nope").is_err());
    }

    #[test]
    fn test_cron_manager_list_jobs_sorted() {
        let mut mgr = CronManager::new();
        mgr.add_job(CronJob::new("zebra", "* * * * *", "cmd1")).unwrap();
        mgr.add_job(CronJob::new("alpha", "* * * * *", "cmd2")).unwrap();
        mgr.add_job(CronJob::new("middle", "* * * * *", "cmd3")).unwrap();
        let jobs = mgr.list_jobs();
        let names: Vec<&str> = jobs.iter().map(|j| j.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "middle", "zebra"]);
    }

    #[test]
    fn test_cron_manager_run_pending_only_enabled() {
        let mut mgr = CronManager::new();
        mgr.add_job(CronJob::new("enabled-job", "* * * * *", "cmd1")).unwrap();
        let mut disabled = CronJob::new("disabled-job", "* * * * *", "cmd2");
        disabled.enabled = false;
        mgr.add_job(disabled).unwrap();

        let executed = mgr.run_pending();
        assert_eq!(executed.len(), 1);
        assert_eq!(executed[0], "enabled-job");
    }

    #[test]
    fn test_cron_manager_empty() {
        let mgr = CronManager::new();
        assert!(mgr.is_empty());
        assert_eq!(mgr.len(), 0);
        assert!(mgr.list_jobs().is_empty());
    }
}
