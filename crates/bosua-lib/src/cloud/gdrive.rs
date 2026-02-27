//! Google Drive integration client.
//!
//! Provides OAuth2 authentication, file management (upload, download, copy,
//! move, delete), folder operations, batch operations, permission sharing,
//! and an interactive browse interface with completion support.
//!
//! Uses `FileLock` (GdriveLockFile, GdriveRetryLockFile) to coordinate
//! concurrent Google Drive operations across processes.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, RefreshToken,
    Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::cloud::CloudClient;
use crate::config::dynamic::DynamicConfig;
use crate::errors::{BosuaError, Result};
use crate::fileops::lock::FileLock;
use crate::http_client::HttpClient;

// ---------------------------------------------------------------------------
// Data models
// ---------------------------------------------------------------------------

/// Metadata for a Google Drive file or folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GDriveFile {
    pub id: String,
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub parents: Vec<String>,
    #[serde(rename = "createdTime", default)]
    pub created_time: Option<String>,
    #[serde(rename = "modifiedTime", default)]
    pub modified_time: Option<String>,
    #[serde(rename = "webViewLink", default)]
    pub web_view_link: Option<String>,
    #[serde(rename = "webContentLink", default)]
    pub web_content_link: Option<String>,
}

/// Response from the Google Drive files.list API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GDriveFileList {
    pub files: Vec<GDriveFile>,
    #[serde(rename = "nextPageToken", default)]
    pub next_page_token: Option<String>,
}

/// Request body for creating a file/folder in Google Drive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GDriveCreateRequest {
    pub name: String,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parents: Option<Vec<String>>,
}

/// Permission entry for sharing a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GDrivePermission {
    pub role: String,
    #[serde(rename = "type")]
    pub permission_type: String,
    #[serde(rename = "emailAddress", skip_serializing_if = "Option::is_none")]
    pub email_address: Option<String>,
}

/// Persisted OAuth2 token data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GDriveToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expiry: Option<String>,
}

/// Entry shown during interactive browse.
#[derive(Debug, Clone)]
pub struct BrowseEntry {
    pub file: GDriveFile,
    pub depth: usize,
}

/// Result of a batch operation on multiple files.
#[derive(Debug, Clone)]
pub struct BatchResult {
    pub succeeded: Vec<String>,
    pub failed: Vec<(String, String)>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const DRIVE_API_BASE: &str = "https://www.googleapis.com/drive/v3";
const DRIVE_UPLOAD_BASE: &str = "https://www.googleapis.com/upload/drive/v3";
const FOLDER_MIME_TYPE: &str = "application/vnd.google-apps.folder";

// ---------------------------------------------------------------------------
// GDriveClient
// ---------------------------------------------------------------------------

/// Google Drive client with OAuth2 authentication and file management.
///
/// Coordinates concurrent operations via `FileLock` and supports a
/// configurable default account from `DynamicConfig`.
pub struct GDriveClient {
    http: HttpClient,
    token: Arc<RwLock<Option<GDriveToken>>>,
    token_file: PathBuf,
    lock: FileLock,
    retry_lock: FileLock,
    default_account: Arc<RwLock<String>>,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl GDriveClient {
    /// Create a new `GDriveClient`.
    ///
    /// * `http` – shared HTTP client
    /// * `token_file` – path to persist OAuth2 tokens (from `SimplifiedConfig::token_file`)
    /// * `gdrive_lock_file` – path for the primary GDrive lock
    /// * `gdrive_retry_lock_file` – path for the retry lock
    /// * `config` – current `DynamicConfig` snapshot (provides `gdrive_default_account`)
    /// * `client_id` / `client_secret` – Google OAuth2 credentials
    pub fn new(
        http: HttpClient,
        token_file: PathBuf,
        gdrive_lock_file: PathBuf,
        gdrive_retry_lock_file: PathBuf,
        config: &DynamicConfig,
        client_id: String,
        client_secret: String,
    ) -> Self {
        Self {
            http,
            token: Arc::new(RwLock::new(None)),
            token_file,
            lock: FileLock::new(gdrive_lock_file),
            retry_lock: FileLock::new(gdrive_retry_lock_file),
            default_account: Arc::new(RwLock::new(config.gdrive_default_account.clone())),
            client_id,
            client_secret,
            redirect_uri: "urn:ietf:wg:oauth:2.0:oob".to_string(),
        }
    }

    /// Update the default account when `DynamicConfig` changes.
    pub async fn update_default_account(&self, account: &str) {
        *self.default_account.write().await = account.to_string();
    }

    /// Get the currently configured default account.
    pub async fn default_account(&self) -> String {
        self.default_account.read().await.clone()
    }

    // -----------------------------------------------------------------------
    // OAuth2 helpers
    // -----------------------------------------------------------------------

    /// Build the `oauth2::BasicClient` for Google Drive.
    fn oauth2_client(&self) -> Result<BasicClient> {
        let auth_url = AuthUrl::new(GOOGLE_AUTH_URL.to_string())
            .map_err(|e| BosuaError::OAuth2(format!("Invalid auth URL: {e}")))?;
        let token_url = TokenUrl::new(GOOGLE_TOKEN_URL.to_string())
            .map_err(|e| BosuaError::OAuth2(format!("Invalid token URL: {e}")))?;
        let redirect_url = RedirectUrl::new(self.redirect_uri.clone())
            .map_err(|e| BosuaError::OAuth2(format!("Invalid redirect URI: {e}")))?;

        let client = BasicClient::new(
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(self.client_secret.clone())),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(redirect_url);

        Ok(client)
    }

    /// Generate the authorization URL the user should visit.
    pub fn authorization_url(&self) -> Result<(String, CsrfToken)> {
        let client = self.oauth2_client()?;
        let (url, csrf) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/drive".to_string(),
            ))
            .url();
        Ok((url.to_string(), csrf))
    }

    /// Exchange an authorization code for tokens and persist them.
    pub async fn exchange_code(&self, code: &str) -> Result<GDriveToken> {
        let client = self.oauth2_client()?;

        let token_result = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(oauth2::reqwest::async_http_client)
            .await
            .map_err(|e| BosuaError::OAuth2(format!("Token exchange failed: {e}")))?;

        let gdrive_token = GDriveToken {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result
                .refresh_token()
                .map(|t| t.secret().clone()),
            token_type: "Bearer".to_string(),
            expiry: None,
        };

        self.save_token(&gdrive_token).await?;
        *self.token.write().await = Some(gdrive_token.clone());
        Ok(gdrive_token)
    }

    /// Refresh the access token using the stored refresh token.
    pub async fn refresh_token(&self) -> Result<GDriveToken> {
        let current = self.token.read().await;
        let refresh = current
            .as_ref()
            .and_then(|t| t.refresh_token.clone())
            .ok_or_else(|| BosuaError::OAuth2("No refresh token available".into()))?;
        let old_refresh = current
            .as_ref()
            .and_then(|t| t.refresh_token.clone());
        drop(current);

        let client = self.oauth2_client()?;

        let token_result = client
            .exchange_refresh_token(&RefreshToken::new(refresh))
            .request_async(oauth2::reqwest::async_http_client)
            .await
            .map_err(|e| BosuaError::OAuth2(format!("Token refresh failed: {e}")))?;

        let gdrive_token = GDriveToken {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result
                .refresh_token()
                .map(|t| t.secret().clone())
                .or(old_refresh),
            token_type: "Bearer".to_string(),
            expiry: None,
        };

        self.save_token(&gdrive_token).await?;
        *self.token.write().await = Some(gdrive_token.clone());
        Ok(gdrive_token)
    }

    /// Load a persisted token from disk.
    pub async fn load_token(&self) -> Result<Option<GDriveToken>> {
        if !self.token_file.exists() {
            return Ok(None);
        }
        let data = tokio::fs::read_to_string(&self.token_file).await?;
        let token: GDriveToken = serde_json::from_str(&data)?;
        *self.token.write().await = Some(token.clone());
        Ok(Some(token))
    }

    /// Persist the token to disk.
    async fn save_token(&self, token: &GDriveToken) -> Result<()> {
        if let Some(parent) = self.token_file.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let data = serde_json::to_string_pretty(token)?;
        tokio::fs::write(&self.token_file, data).await?;
        Ok(())
    }

    /// Get a valid access token, refreshing if necessary.
    async fn access_token(&self) -> Result<String> {
        let guard = self.token.read().await;
        match &*guard {
            Some(t) => Ok(t.access_token.clone()),
            None => {
                drop(guard);
                if let Some(t) = self.load_token().await? {
                    return Ok(t.access_token);
                }
                Err(BosuaError::Auth(
                    "Not authenticated. Run the OAuth2 flow first.".into(),
                ))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Lock helpers
    // -----------------------------------------------------------------------

    /// Acquire the primary GDrive lock. Returns a guard that releases on drop.
    pub fn acquire_lock(&self) -> Result<crate::fileops::lock::LockGuard> {
        self.lock.acquire()
    }

    /// Acquire the retry lock. Returns a guard that releases on drop.
    pub fn acquire_retry_lock(&self) -> Result<crate::fileops::lock::LockGuard> {
        self.retry_lock.acquire()
    }

    // -----------------------------------------------------------------------
    // File operations
    // -----------------------------------------------------------------------

    /// List files in a folder (or root if `folder_id` is `None`).
    pub async fn list_files(
        &self,
        folder_id: Option<&str>,
        page_token: Option<&str>,
        page_size: Option<u32>,
    ) -> Result<GDriveFileList> {
        let token = self.access_token().await?;
        let client = self.http.get_client().await;

        let parent = folder_id.unwrap_or("root");
        let q = format!("'{parent}' in parents and trashed = false");
        let size = page_size.unwrap_or(100).to_string();

        let mut params: Vec<(&str, &str)> = vec![
            ("q", q.as_str()),
            ("pageSize", size.as_str()),
            (
                "fields",
                "files(id,name,mimeType,size,parents,createdTime,modifiedTime,webViewLink,webContentLink),nextPageToken",
            ),
        ];

        if let Some(pt) = page_token {
            params.push(("pageToken", pt));
        }

        let resp = client
            .get(format!("{DRIVE_API_BASE}/files"))
            .bearer_auth(&token)
            .query(&params)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "gdrive".into(),
                message: format!("list_files failed ({status}): {body}"),
            });
        }

        resp.json::<GDriveFileList>()
            .await
            .map_err(BosuaError::Http)
    }

    /// Get metadata for a single file.
    pub async fn get_file_metadata(&self, file_id: &str) -> Result<GDriveFile> {
        let token = self.access_token().await?;
        let client = self.http.get_client().await;

        let resp = client
            .get(format!("{DRIVE_API_BASE}/files/{file_id}"))
            .bearer_auth(&token)
            .query(&[(
                "fields",
                "id,name,mimeType,size,parents,createdTime,modifiedTime,webViewLink,webContentLink",
            )])
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "gdrive".into(),
                message: format!("get_file_metadata failed ({status}): {body}"),
            });
        }

        resp.json::<GDriveFile>().await.map_err(BosuaError::Http)
    }

    /// Download a file's content from Google Drive.
    ///
    /// Returns the raw bytes of the file. For Google Docs/Sheets/Slides,
    /// use the export endpoint instead (not covered here).
    pub async fn download_file(&self, file_id: &str) -> Result<Vec<u8>> {
        let token = self.access_token().await?;
        let client = self.http.get_client().await;

        let resp = client
            .get(format!("{DRIVE_API_BASE}/files/{file_id}"))
            .bearer_auth(&token)
            .query(&[("alt", "media")])
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "gdrive".into(),
                message: format!("download_file failed ({status}): {body}"),
            });
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(BosuaError::Http)
    }

    /// Upload a file to Google Drive using simple JSON metadata + raw body.
    ///
    /// The file is placed in `parent_id` (or root if `None`).
    pub async fn upload_file(
        &self,
        file_path: &Path,
        parent_id: Option<&str>,
    ) -> Result<GDriveFile> {
        let _guard = self.acquire_lock()?;
        let token = self.access_token().await?;
        let client = self.http.get_client().await;

        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("untitled");

        // Step 1: Create file metadata
        let mut metadata = serde_json::json!({ "name": file_name });
        if let Some(pid) = parent_id {
            metadata["parents"] = serde_json::json!([pid]);
        }

        let create_resp = client
            .post(format!("{DRIVE_API_BASE}/files"))
            .bearer_auth(&token)
            .json(&metadata)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !create_resp.status().is_success() {
            let status = create_resp.status().as_u16();
            let body = create_resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "gdrive".into(),
                message: format!("upload_file metadata failed ({status}): {body}"),
            });
        }

        let created: GDriveFile = create_resp.json().await.map_err(BosuaError::Http)?;

        // Step 2: Upload file content
        let file_bytes = tokio::fs::read(file_path).await?;

        let upload_resp = client
            .patch(format!(
                "{DRIVE_UPLOAD_BASE}/files/{}?uploadType=media",
                created.id
            ))
            .bearer_auth(&token)
            .header("Content-Type", "application/octet-stream")
            .body(file_bytes)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !upload_resp.status().is_success() {
            let status = upload_resp.status().as_u16();
            let body = upload_resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "gdrive".into(),
                message: format!("upload_file content failed ({status}): {body}"),
            });
        }

        upload_resp
            .json::<GDriveFile>()
            .await
            .map_err(BosuaError::Http)
    }

    /// Create a folder in Google Drive.
    pub async fn create_folder(
        &self,
        name: &str,
        parent_id: Option<&str>,
    ) -> Result<GDriveFile> {
        let _guard = self.acquire_lock()?;
        let token = self.access_token().await?;
        let client = self.http.get_client().await;

        let mut body = GDriveCreateRequest {
            name: name.to_string(),
            mime_type: Some(FOLDER_MIME_TYPE.to_string()),
            parents: None,
        };
        if let Some(pid) = parent_id {
            body.parents = Some(vec![pid.to_string()]);
        }

        let resp = client
            .post(format!("{DRIVE_API_BASE}/files"))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "gdrive".into(),
                message: format!("create_folder failed ({status}): {text}"),
            });
        }

        resp.json::<GDriveFile>().await.map_err(BosuaError::Http)
    }

    /// Delete a file or folder by ID.
    pub async fn delete_file(&self, file_id: &str) -> Result<()> {
        let _guard = self.acquire_lock()?;
        let token = self.access_token().await?;
        let client = self.http.get_client().await;

        let resp = client
            .delete(format!("{DRIVE_API_BASE}/files/{file_id}"))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "gdrive".into(),
                message: format!("delete_file failed ({status}): {body}"),
            });
        }

        Ok(())
    }

    /// Move a file to a different folder.
    pub async fn move_file(
        &self,
        file_id: &str,
        new_parent_id: &str,
    ) -> Result<GDriveFile> {
        let _guard = self.acquire_lock()?;
        let token = self.access_token().await?;
        let client = self.http.get_client().await;

        // Get current parents to remove
        let meta = self.get_file_metadata(file_id).await?;
        let remove_parents = meta.parents.join(",");

        let resp = client
            .patch(format!("{DRIVE_API_BASE}/files/{file_id}"))
            .bearer_auth(&token)
            .query(&[
                ("addParents", new_parent_id),
                ("removeParents", remove_parents.as_str()),
                ("fields", "id,name,mimeType,size,parents"),
            ])
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "gdrive".into(),
                message: format!("move_file failed ({status}): {body}"),
            });
        }

        resp.json::<GDriveFile>().await.map_err(BosuaError::Http)
    }

    /// Copy a file.
    pub async fn copy_file(
        &self,
        file_id: &str,
        new_name: Option<&str>,
        parent_id: Option<&str>,
    ) -> Result<GDriveFile> {
        let _guard = self.acquire_lock()?;
        let token = self.access_token().await?;
        let client = self.http.get_client().await;

        let mut body = serde_json::Map::new();
        if let Some(name) = new_name {
            body.insert("name".into(), serde_json::Value::String(name.to_string()));
        }
        if let Some(pid) = parent_id {
            body.insert(
                "parents".into(),
                serde_json::Value::Array(vec![serde_json::Value::String(pid.to_string())]),
            );
        }

        let resp = client
            .post(format!("{DRIVE_API_BASE}/files/{file_id}/copy"))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "gdrive".into(),
                message: format!("copy_file failed ({status}): {text}"),
            });
        }

        resp.json::<GDriveFile>().await.map_err(BosuaError::Http)
    }

    /// Share a file by creating a permission.
    pub async fn share_file(
        &self,
        file_id: &str,
        permission: &GDrivePermission,
    ) -> Result<()> {
        let token = self.access_token().await?;
        let client = self.http.get_client().await;

        let resp = client
            .post(format!("{DRIVE_API_BASE}/files/{file_id}/permissions"))
            .bearer_auth(&token)
            .json(permission)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "gdrive".into(),
                message: format!("share_file failed ({status}): {body}"),
            });
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Batch operations
    // -----------------------------------------------------------------------

    /// Delete multiple files. Returns a summary of successes and failures.
    pub async fn batch_delete(&self, file_ids: &[String]) -> Result<BatchResult> {
        let _guard = self.acquire_lock()?;
        let mut result = BatchResult {
            succeeded: Vec::new(),
            failed: Vec::new(),
        };

        for id in file_ids {
            let token = self.access_token().await?;
            let client = self.http.get_client().await;

            let resp = client
                .delete(format!("{DRIVE_API_BASE}/files/{id}"))
                .bearer_auth(&token)
                .send()
                .await;

            match resp {
                Ok(r) if r.status().is_success() => {
                    result.succeeded.push(id.clone());
                }
                Ok(r) => {
                    let status = r.status().as_u16();
                    let body = r.text().await.unwrap_or_default();
                    result.failed.push((id.clone(), format!("{status}: {body}")));
                }
                Err(e) => {
                    result.failed.push((id.clone(), e.to_string()));
                }
            }
        }

        Ok(result)
    }

    /// Move multiple files to a target folder.
    pub async fn batch_move(
        &self,
        file_ids: &[String],
        target_folder_id: &str,
    ) -> Result<BatchResult> {
        let _guard = self.acquire_lock()?;
        let mut result = BatchResult {
            succeeded: Vec::new(),
            failed: Vec::new(),
        };

        for id in file_ids {
            let token = self.access_token().await?;
            let client = self.http.get_client().await;

            // Get current parents
            let meta_resp = client
                .get(format!("{DRIVE_API_BASE}/files/{id}"))
                .bearer_auth(&token)
                .query(&[("fields", "parents")])
                .send()
                .await;

            let remove_parents = match meta_resp {
                Ok(r) if r.status().is_success() => {
                    let meta: serde_json::Value = r.json().await.unwrap_or_default();
                    meta["parents"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<_>>()
                                .join(",")
                        })
                        .unwrap_or_default()
                }
                _ => String::new(),
            };

            let resp = client
                .patch(format!("{DRIVE_API_BASE}/files/{id}"))
                .bearer_auth(&token)
                .query(&[
                    ("addParents", target_folder_id),
                    ("removeParents", remove_parents.as_str()),
                ])
                .send()
                .await;

            match resp {
                Ok(r) if r.status().is_success() => {
                    result.succeeded.push(id.clone());
                }
                Ok(r) => {
                    let status = r.status().as_u16();
                    let body = r.text().await.unwrap_or_default();
                    result.failed.push((id.clone(), format!("{status}: {body}")));
                }
                Err(e) => {
                    result.failed.push((id.clone(), e.to_string()));
                }
            }
        }

        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Interactive browse
    // -----------------------------------------------------------------------

    /// List folder contents for the interactive browse interface.
    ///
    /// Returns all entries (paginating automatically) in the given folder.
    pub async fn browse_list(&self, folder_id: Option<&str>) -> Result<Vec<BrowseEntry>> {
        let mut entries = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let list = self
                .list_files(folder_id, page_token.as_deref(), Some(100))
                .await?;

            for file in list.files {
                entries.push(BrowseEntry { file, depth: 0 });
            }

            match list.next_page_token {
                Some(pt) => page_token = Some(pt),
                None => break,
            }
        }

        Ok(entries)
    }

    /// Provide completion suggestions for the browse interface.
    ///
    /// Returns file/folder names that start with the given `prefix` in the
    /// specified folder.
    pub async fn browse_completions(
        &self,
        folder_id: Option<&str>,
        prefix: &str,
    ) -> Result<Vec<String>> {
        let entries = self.browse_list(folder_id).await?;
        let lower_prefix = prefix.to_lowercase();
        Ok(entries
            .into_iter()
            .filter(|e| e.file.name.to_lowercase().starts_with(&lower_prefix))
            .map(|e| e.file.name)
            .collect())
    }

    /// Interactive browse using `dialoguer` for selection.
    ///
    /// On Linux, this uses a stub implementation that prints a flat list
    /// instead of a full TUI.
    pub async fn browse_interactive(
        &self,
        start_folder_id: Option<&str>,
    ) -> Result<Option<GDriveFile>> {
        browse_display(self, start_folder_id).await
    }
}

// ---------------------------------------------------------------------------
// Platform-specific browse display
// ---------------------------------------------------------------------------

/// Full interactive browse (macOS / Windows) using dialoguer.
#[cfg(not(target_os = "linux"))]
async fn browse_display(
    client: &GDriveClient,
    folder_id: Option<&str>,
) -> Result<Option<GDriveFile>> {
    use dialoguer::Select;

    let mut current_folder: Option<String> = folder_id.map(|s| s.to_string());

    loop {
        let entries = client.browse_list(current_folder.as_deref()).await?;

        if entries.is_empty() {
            tracing::info!("Folder is empty");
            return Ok(None);
        }

        let display_names: Vec<String> = std::iter::once(".. (go back)".to_string())
            .chain(entries.iter().map(|e| {
                let icon = if e.file.mime_type == FOLDER_MIME_TYPE {
                    "📁"
                } else {
                    "📄"
                };
                format!("{icon} {}", e.file.name)
            }))
            .collect();

        let selection = Select::new()
            .with_prompt("Browse Google Drive")
            .items(&display_names)
            .default(0)
            .interact_opt()
            .map_err(|e| BosuaError::Command(format!("Browse selection failed: {e}")))?;

        match selection {
            None => return Ok(None),
            Some(0) => return Ok(None),
            Some(idx) => {
                let entry = &entries[idx - 1];
                if entry.file.mime_type == FOLDER_MIME_TYPE {
                    current_folder = Some(entry.file.id.clone());
                } else {
                    return Ok(Some(entry.file.clone()));
                }
            }
        }
    }
}

/// Stub browse display for Linux — prints a flat list to stdout.
#[cfg(target_os = "linux")]
async fn browse_display(
    client: &GDriveClient,
    folder_id: Option<&str>,
) -> Result<Option<GDriveFile>> {
    let entries = client.browse_list(folder_id).await?;

    if entries.is_empty() {
        println!("(empty folder)");
        return Ok(None);
    }

    for (i, entry) in entries.iter().enumerate() {
        let kind = if entry.file.mime_type == FOLDER_MIME_TYPE {
            "DIR "
        } else {
            "FILE"
        };
        println!("[{:>3}] {kind}  {}", i + 1, entry.file.name);
    }

    // On Linux server variants the browse is non-interactive; callers
    // should use the HTTP API endpoints instead.
    Ok(None)
}

// ---------------------------------------------------------------------------
// CloudClient trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl CloudClient for GDriveClient {
    fn name(&self) -> &str {
        "Google Drive"
    }

    async fn authenticate(&self) -> Result<()> {
        // Try loading an existing token first.
        if let Some(_token) = self.load_token().await? {
            tracing::info!("Loaded existing Google Drive token");
            return Ok(());
        }

        // No persisted token — the caller must run the interactive OAuth2
        // flow via `authorization_url()` + `exchange_code()`.
        Err(BosuaError::Auth(
            "No stored credentials. Use the OAuth2 flow to authenticate.".into(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_client() -> GDriveClient {
        let http = HttpClient::from_defaults().unwrap();
        let config = DynamicConfig::default();
        GDriveClient::new(
            http,
            PathBuf::from("/tmp/test-gdrive-token.json"),
            PathBuf::from("/tmp/test-gdrive.lock"),
            PathBuf::from("/tmp/test-gdrive-retry.lock"),
            &config,
            "test-client-id".into(),
            "test-client-secret".into(),
        )
    }

    #[test]
    fn test_new_client() {
        let client = make_test_client();
        assert_eq!(client.name(), "Google Drive");
        assert_eq!(client.client_id, "test-client-id");
    }

    #[tokio::test]
    async fn test_default_account() {
        let client = make_test_client();
        assert_eq!(client.default_account().await, "");

        client.update_default_account("user@example.com").await;
        assert_eq!(client.default_account().await, "user@example.com");
    }

    #[test]
    fn test_authorization_url() {
        let client = make_test_client();
        let (url, _csrf) = client.authorization_url().unwrap();
        assert!(url.contains("accounts.google.com"));
        assert!(url.contains("test-client-id"));
    }

    #[test]
    fn test_lock_paths() {
        let client = make_test_client();
        assert_eq!(client.lock.path(), Path::new("/tmp/test-gdrive.lock"));
        assert_eq!(
            client.retry_lock.path(),
            Path::new("/tmp/test-gdrive-retry.lock")
        );
    }

    #[tokio::test]
    async fn test_authenticate_no_token() {
        let client = make_test_client();
        let result = client.authenticate().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            BosuaError::Auth(msg) => {
                assert!(msg.contains("OAuth2"));
            }
            other => panic!("Expected Auth error, got: {:?}", other),
        }
    }

    #[test]
    fn test_gdrive_file_serde() {
        let file = GDriveFile {
            id: "abc123".into(),
            name: "test.txt".into(),
            mime_type: "text/plain".into(),
            size: Some(1024),
            parents: vec!["root".into()],
            created_time: Some("2024-01-01T00:00:00Z".into()),
            modified_time: None,
            web_view_link: None,
            web_content_link: None,
        };

        let json = serde_json::to_string(&file).unwrap();
        let deserialized: GDriveFile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "abc123");
        assert_eq!(deserialized.name, "test.txt");
        assert_eq!(deserialized.size, Some(1024));
    }

    #[test]
    fn test_gdrive_permission_serde() {
        let perm = GDrivePermission {
            role: "reader".into(),
            permission_type: "user".into(),
            email_address: Some("user@example.com".into()),
        };

        let json = serde_json::to_string(&perm).unwrap();
        assert!(json.contains("\"type\":\"user\""));
        assert!(json.contains("\"emailAddress\""));
    }

    #[test]
    fn test_gdrive_token_serde() {
        let token = GDriveToken {
            access_token: "ya29.xxx".into(),
            refresh_token: Some("1//xxx".into()),
            token_type: "Bearer".into(),
            expiry: None,
        };

        let json = serde_json::to_string(&token).unwrap();
        let deserialized: GDriveToken = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.access_token, "ya29.xxx");
        assert_eq!(deserialized.refresh_token, Some("1//xxx".into()));
    }

    #[test]
    fn test_create_request_serde() {
        let req = GDriveCreateRequest {
            name: "My Folder".into(),
            mime_type: Some(FOLDER_MIME_TYPE.into()),
            parents: Some(vec!["parent123".into()]),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"mimeType\""));
        assert!(json.contains("application/vnd.google-apps.folder"));
    }
}
