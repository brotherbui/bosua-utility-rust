pub mod browser;
pub mod country;
pub mod html;
pub mod pdf;
pub mod sanitize;
pub mod sftp;
pub mod telegram;

use crate::errors::{BosuaError, Result};

/// Run an external tool, capturing stdout. Returns error if tool is missing or exits non-zero.
pub async fn run_external_tool(tool: &str, args: &[&str]) -> Result<String> {
    let output = tokio::process::Command::new(tool)
        .args(args)
        .output()
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BosuaError::Command(format!(
                    "'{}' not found. Please install it to use this feature.",
                    tool
                ))
            } else {
                BosuaError::Io(e)
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BosuaError::Command(format!(
            "{} failed: {}",
            tool,
            stderr.trim()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_external_tool_missing_tool() {
        let result = run_external_tool("nonexistent_tool_xyz_12345", &[]).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match &err {
            BosuaError::Command(msg) => {
                assert!(msg.contains("nonexistent_tool_xyz_12345"));
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected BosuaError::Command, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_run_external_tool_success() {
        let result = run_external_tool("echo", &["hello"]).await;
        assert!(result.is_ok());
        assert!(result.unwrap().trim() == "hello");
    }

    #[tokio::test]
    async fn test_run_external_tool_nonzero_exit() {
        let result = run_external_tool("false", &[]).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            BosuaError::Command(msg) => {
                assert!(msg.contains("false failed"));
            }
            e => panic!("Expected BosuaError::Command, got {:?}", e),
        }
    }
}
