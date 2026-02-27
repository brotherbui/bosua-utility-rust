//! File and folder management utilities.
//!
//! Provides create, copy, move, delete, and list operations for files and
//! directories, plus a Unix permission helper that defaults to 0o644.

use std::fs;
use std::path::{Path, PathBuf};

use crate::errors::Result;

pub mod lock;

/// Create a file with the given content and set permissions to 0o644.
pub fn create_file(path: &Path, content: &[u8]) -> Result<()> {
    fs::write(path, content)?;
    set_permissions(path, 0o644)?;
    Ok(())
}

/// Copy a file from `src` to `dst`, returning the number of bytes copied.
pub fn copy_file(src: &Path, dst: &Path) -> Result<u64> {
    Ok(fs::copy(src, dst)?)
}

/// Move (rename) a file from `src` to `dst`.
pub fn move_file(src: &Path, dst: &Path) -> Result<()> {
    fs::rename(src, dst)?;
    Ok(())
}

/// Delete a single file.
pub fn delete_file(path: &Path) -> Result<()> {
    fs::remove_file(path)?;
    Ok(())
}

/// Create a directory and all missing parent directories.
pub fn create_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    Ok(())
}

/// Recursively delete a directory and all its contents.
pub fn delete_dir(path: &Path) -> Result<()> {
    fs::remove_dir_all(path)?;
    Ok(())
}

/// List immediate children of a directory, returning their full paths.
pub fn list_dir(path: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        entries.push(entry?.path());
    }
    Ok(entries)
}

/// Set file permissions on Unix (mode bits such as 0o644).
///
/// On non-Unix platforms this is a no-op.
#[cfg(unix)]
pub fn set_permissions(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = fs::Permissions::from_mode(mode);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
pub fn set_permissions(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_and_read_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("hello.txt");
        create_file(&path, b"hello world").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"hello world");
    }

    #[cfg(unix)]
    #[test]
    fn test_create_file_sets_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("perms.txt");
        create_file(&path, b"data").unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o644);
    }

    #[test]
    fn test_copy_file() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src.txt");
        let dst = tmp.path().join("dst.txt");
        create_file(&src, b"copy me").unwrap();
        let bytes = copy_file(&src, &dst).unwrap();
        assert_eq!(bytes, 7);
        assert_eq!(fs::read(&dst).unwrap(), b"copy me");
    }

    #[test]
    fn test_move_file() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("a.txt");
        let dst = tmp.path().join("b.txt");
        create_file(&src, b"move me").unwrap();
        move_file(&src, &dst).unwrap();
        assert!(!src.exists());
        assert_eq!(fs::read(&dst).unwrap(), b"move me");
    }

    #[test]
    fn test_delete_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("del.txt");
        create_file(&path, b"bye").unwrap();
        assert!(path.exists());
        delete_file(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_create_and_delete_dir() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("a/b/c");
        create_dir(&dir).unwrap();
        assert!(dir.is_dir());
        delete_dir(&tmp.path().join("a")).unwrap();
        assert!(!tmp.path().join("a").exists());
    }

    #[test]
    fn test_list_dir() {
        let tmp = TempDir::new().unwrap();
        create_file(&tmp.path().join("one.txt"), b"1").unwrap();
        create_file(&tmp.path().join("two.txt"), b"2").unwrap();
        let mut entries = list_dir(tmp.path()).unwrap();
        entries.sort();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].ends_with("one.txt"));
        assert!(entries[1].ends_with("two.txt"));
    }

    #[test]
    fn test_delete_nonexistent_file_returns_error() {
        let tmp = TempDir::new().unwrap();
        let result = delete_file(&tmp.path().join("nope.txt"));
        assert!(result.is_err());
    }
}
