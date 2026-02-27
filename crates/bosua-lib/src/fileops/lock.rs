//! File-based advisory locking with RAII release.
//!
//! Uses `fs2` for cross-platform exclusive file locks. The lock file is
//! created on `acquire()` and removed when the returned [`LockGuard`] is
//! dropped.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use fs2::FileExt;

use crate::errors::{BosuaError, Result};

/// A file-based advisory lock.
///
/// Lock file paths are configurable — callers typically use one of the
/// well-known paths from `SimplifiedConfig` (DownloadLockFile,
/// GdriveLockFile, GdriveRetryLockFile).
pub struct FileLock {
    path: PathBuf,
}

/// RAII guard that holds an exclusive file lock.
///
/// The lock is released and the lock file is removed when this guard is
/// dropped.
#[derive(Debug)]
pub struct LockGuard {
    file: File,
    path: PathBuf,
}

impl FileLock {
    /// Create a new `FileLock` targeting the given path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Return the lock file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Try to acquire an exclusive lock.
    ///
    /// Returns a [`LockGuard`] on success. If the lock is already held by
    /// another process/thread, returns [`BosuaError::LockConflict`].
    pub fn acquire(&self) -> Result<LockGuard> {
        // Ensure parent directory exists so the lock file can be created.
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&self.path)?;

        file.try_lock_exclusive().map_err(|_| BosuaError::LockConflict {
            path: self.path.clone(),
        })?;

        Ok(LockGuard {
            file,
            path: self.path.clone(),
        })
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_acquire_and_release() {
        let tmp = TempDir::new().unwrap();
        let lock_path = tmp.path().join("test.lock");
        let lock = FileLock::new(&lock_path);

        {
            let _guard = lock.acquire().unwrap();
            assert!(lock_path.exists());
        }
        // Guard dropped — lock file removed.
        assert!(!lock_path.exists());
    }

    #[test]
    fn test_double_acquire_returns_lock_conflict() {
        let tmp = TempDir::new().unwrap();
        let lock_path = tmp.path().join("conflict.lock");
        let lock = FileLock::new(&lock_path);

        let _guard = lock.acquire().unwrap();

        // Second acquire on the same lock should fail.
        let result = lock.acquire();
        assert!(result.is_err());
        match result.unwrap_err() {
            BosuaError::LockConflict { path } => {
                assert_eq!(path, lock_path);
            }
            other => panic!("Expected LockConflict, got: {:?}", other),
        }
    }

    #[test]
    fn test_lock_released_after_guard_drop() {
        let tmp = TempDir::new().unwrap();
        let lock_path = tmp.path().join("reacquire.lock");
        let lock = FileLock::new(&lock_path);

        {
            let _guard = lock.acquire().unwrap();
        }

        // Should succeed after the first guard is dropped.
        let _guard2 = lock.acquire().unwrap();
    }

    #[test]
    fn test_creates_parent_directories() {
        let tmp = TempDir::new().unwrap();
        let lock_path = tmp.path().join("nested/dir/deep.lock");
        let lock = FileLock::new(&lock_path);

        let _guard = lock.acquire().unwrap();
        assert!(lock_path.exists());
    }

    #[test]
    fn test_path_accessor() {
        let lock = FileLock::new("/tmp/some.lock");
        assert_eq!(lock.path(), Path::new("/tmp/some.lock"));
    }
}
