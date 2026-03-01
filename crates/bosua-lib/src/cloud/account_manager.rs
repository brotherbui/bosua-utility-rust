//! Generic cloud account manager for config-file-based account management.
//!
//! Shared pattern used by Cloudflare, Tailscale, AWS, GCloud, etc.
//! Accounts are stored as directories under `~/.config/<service>/` with
//! `account.json` tracking the current account and `credentials.json` per account.

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::errors::{BosuaError, Result};

/// Current account selection config.
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountConfig {
    pub current: String,
}

/// Generic credentials stored as JSON.
pub type Credentials = serde_json::Value;

/// A generic account manager for a cloud service.
pub struct AccountManager {
    /// e.g. "cloudflare", "tailscale"
    service_name: String,
    /// Base path, e.g. ~/.config/cloudflare
    base_path: PathBuf,
}

impl AccountManager {
    /// Create a new account manager for the given service.
    pub fn new(service_name: &str) -> Result<Self> {
        let home = std::env::var("HOME")
            .map_err(|_| BosuaError::Command("Cannot determine home directory".into()))?;
        let base_path = PathBuf::from(home).join(".config").join(service_name);
        Ok(Self {
            service_name: service_name.to_string(),
            base_path,
        })
    }

    /// List all account names (directories with credentials.json).
    pub fn list_accounts(&self) -> Result<Vec<String>> {
        std::fs::create_dir_all(&self.base_path).map_err(|e| {
            BosuaError::Command(format!("Failed to create {}: {}", self.base_path.display(), e))
        })?;
        let mut accounts = Vec::new();
        let entries = std::fs::read_dir(&self.base_path).map_err(|e| {
            BosuaError::Command(format!("Failed to list {}: {}", self.base_path.display(), e))
        })?;
        for entry in entries.flatten() {
            if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                let cred_path = entry.path().join("credentials.json");
                if cred_path.exists() {
                    if let Some(name) = entry.file_name().to_str() {
                        accounts.push(name.to_string());
                    }
                }
            }
        }
        accounts.sort();
        Ok(accounts)
    }

    /// Load the current account config.
    pub fn load_current(&self) -> Result<String> {
        let config_path = self.base_path.join("account.json");
        if !config_path.exists() {
            return Err(BosuaError::Command(format!(
                "No account selected.\nUse `bosua {} account list` to show all accounts\nUse `bosua {} account switch` to select one",
                self.service_name, self.service_name
            )));
        }
        let data = std::fs::read_to_string(&config_path).map_err(|e| {
            BosuaError::Command(format!("Failed to read account config: {}", e))
        })?;
        let config: AccountConfig = serde_json::from_str(&data)?;
        Ok(config.current)
    }

    /// Save the current account selection.
    pub fn save_current(&self, name: &str) -> Result<()> {
        std::fs::create_dir_all(&self.base_path).map_err(|e| {
            BosuaError::Command(format!("Failed to create dir: {}", e))
        })?;
        let config = AccountConfig { current: name.to_string() };
        let data = serde_json::to_string_pretty(&config)?;
        let path = self.base_path.join("account.json");
        std::fs::write(&path, data).map_err(|e| {
            BosuaError::Command(format!("Failed to write account config: {}", e))
        })?;
        Ok(())
    }

    /// Load credentials for a specific account.
    pub fn load_credentials(&self, account_name: &str) -> Result<Credentials> {
        let path = self.base_path.join(account_name).join("credentials.json");
        let data = std::fs::read_to_string(&path).map_err(|e| {
            BosuaError::Command(format!("Failed to read credentials for '{}': {}", account_name, e))
        })?;
        let creds: Credentials = serde_json::from_str(&data)?;
        Ok(creds)
    }

    /// Save credentials for a specific account.
    pub fn save_credentials(&self, account_name: &str, creds: &Credentials) -> Result<()> {
        let dir = self.base_path.join(account_name);
        std::fs::create_dir_all(&dir).map_err(|e| {
            BosuaError::Command(format!("Failed to create account dir: {}", e))
        })?;
        let data = serde_json::to_string_pretty(creds)?;
        let path = dir.join("credentials.json");
        std::fs::write(&path, data).map_err(|e| {
            BosuaError::Command(format!("Failed to write credentials: {}", e))
        })?;
        Ok(())
    }

    /// Check if an account exists.
    pub fn account_exists(&self, name: &str) -> bool {
        self.base_path.join(name).join("credentials.json").exists()
    }

    /// Remove an account directory.
    pub fn remove_account(&self, name: &str) -> Result<()> {
        let path = self.base_path.join(name);
        if path.exists() {
            std::fs::remove_dir_all(&path).map_err(|e| {
                BosuaError::Command(format!("Failed to remove account '{}': {}", name, e))
            })?;
        }
        // If this was the current account, remove account.json
        if let Ok(current) = self.load_current() {
            if current == name {
                let _ = std::fs::remove_file(self.base_path.join("account.json"));
            }
        }
        Ok(())
    }

    /// Export account credentials to a JSON file in the current directory.
    pub fn export_account(&self, name: &str) -> Result<()> {
        if !self.account_exists(name) {
            return Err(BosuaError::Command(format!("Account '{}' not found", name)));
        }
        let creds = self.load_credentials(name)?;
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{name}_{timestamp}.json", self.service_name);
        let data = serde_json::to_string_pretty(&creds)?;
        std::fs::write(&filename, data).map_err(|e| {
            BosuaError::Command(format!("Failed to write export file: {}", e))
        })?;
        println!("Account '{}' exported to {}", name, filename);
        Ok(())
    }

    /// Import account from a JSON file.
    pub fn import_account(&self, json_path: &str) -> Result<()> {
        if !Path::new(json_path).exists() {
            return Err(BosuaError::Command(format!("File does not exist: {}", json_path)));
        }
        let data = std::fs::read_to_string(json_path).map_err(|e| {
            BosuaError::Command(format!("Failed to read file: {}", e))
        })?;
        let creds: Credentials = serde_json::from_str(&data)?;

        // Extract account name from filename
        let filename = Path::new(json_path)
            .file_stem()
            .and_then(|s: &std::ffi::OsStr| s.to_str())
            .unwrap_or("imported");
        let account_name = filename
            .strip_prefix(&format!("{}_", self.service_name))
            .unwrap_or(filename)
            .split('_')
            .next()
            .unwrap_or(filename);

        // Prompt for name
        print!("Import as account name [{}]: ", account_name);
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input).ok();
        let input = input.trim();
        let final_name = if input.is_empty() { account_name } else { input };

        self.save_credentials(final_name, &creds)?;
        self.save_current(final_name)?;
        println!("Account '{}' imported successfully", final_name);
        Ok(())
    }

    /// Interactive add account (prompts for credentials).
    pub fn add_account_interactive(&self, prompts: &[(&str, bool)]) -> Result<()> {
        let stdin = io::stdin();
        let mut reader = stdin.lock();

        print!("Enter account name: ");
        io::stdout().flush().ok();
        let mut name = String::new();
        reader.read_line(&mut name).ok();
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(BosuaError::Command("Account name cannot be empty".into()));
        }

        let mut creds = serde_json::Map::new();
        for (prompt_label, required) in prompts {
            print!("{}: ", prompt_label);
            io::stdout().flush().ok();
            let mut val = String::new();
            reader.read_line(&mut val).ok();
            let val = val.trim().to_string();
            if *required && val.is_empty() {
                return Err(BosuaError::Command(format!("{} cannot be empty", prompt_label)));
            }
            // Convert prompt label to JSON key (lowercase, spaces to camelCase)
            let key = prompt_label
                .to_lowercase()
                .replace(' ', "_")
                .replace("enter_", "")
                .replace("(optional,_press_enter_to_skip)", "");
            let key = key.trim_matches('_').to_string();
            if !val.is_empty() {
                creds.insert(key, serde_json::Value::String(val));
            }
        }

        self.save_credentials(&name, &serde_json::Value::Object(creds))?;
        self.save_current(&name)?;
        println!("Account '{}' added successfully", name);
        Ok(())
    }

    /// Print account list with current marker.
    pub fn print_list(&self) -> Result<()> {
        let accounts = self.list_accounts()?;
        if accounts.is_empty() {
            println!("No accounts configured");
            println!("\nTo add an account:");
            println!("  bosua {} account add", self.service_name);
            return Ok(());
        }
        let current = self.load_current().unwrap_or_default();
        println!("{:<20} {}", "NAME", "CURRENT");
        for account in &accounts {
            let marker = if *account == current { "*" } else { "" };
            println!("{:<20} {}", account, marker);
        }
        Ok(())
    }

    /// Print current account name.
    pub fn print_current(&self) -> Result<()> {
        let current = self.load_current()?;
        println!("{}", current);
        Ok(())
    }

    /// Switch to a different account.
    pub fn switch_account(&self, name: &str) -> Result<()> {
        if !self.account_exists(name) {
            return Err(BosuaError::Command(format!("Account '{}' not found", name)));
        }
        self.save_current(name)?;
        println!("Switched to account '{}'", name);
        Ok(())
    }

    /// Remove account with confirmation prompt.
    pub fn remove_account_interactive(&self, name: &str) -> Result<()> {
        if !self.account_exists(name) {
            return Err(BosuaError::Command(format!("Account '{}' not found", name)));
        }
        print!("Are you sure you want to remove account '{}'? (y/N): ", name);
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input).ok();
        let response = input.trim().to_lowercase();
        if response != "y" && response != "yes" {
            println!("Account removal cancelled");
            return Ok(());
        }
        self.remove_account(name)?;
        println!("Account '{}' removed successfully", name);
        Ok(())
    }

    /// Show detailed account info.
    pub fn show_info(&self, name: Option<&str>) -> Result<()> {
        let account_name = match name {
            Some(n) => n.to_string(),
            None => self.load_current()?,
        };
        if !self.account_exists(&account_name) {
            return Err(BosuaError::Command(format!("Account '{}' not found", account_name)));
        }
        let current = self.load_current().unwrap_or_default();
        println!("Account: {}", account_name);
        if account_name == current {
            println!("Status: Current Account ✓");
        } else {
            println!("Status: Available");
        }
        let cred_path = self.base_path.join(&account_name).join("credentials.json");
        println!("Config Path: {}", self.base_path.join(&account_name).display());
        println!("Credentials File: {}", cred_path.display());
        if let Ok(metadata) = std::fs::metadata(&cred_path) {
            println!("File Status: Exists ({:?})", metadata.permissions());
            if let Ok(creds) = self.load_credentials(&account_name) {
                println!("\n--- Credentials Details ---");
                if let Some(obj) = creds.as_object() {
                    for (key, val) in obj {
                        if let Some(s) = val.as_str() {
                            if s.len() > 12 {
                                println!("{}: {}...{}", key, &s[..8], &s[s.len()-4..]);
                            } else {
                                println!("{}: ****", key);
                            }
                        }
                    }
                }
            }
        } else {
            println!("File Status: Missing ❌");
        }
        Ok(())
    }
}
