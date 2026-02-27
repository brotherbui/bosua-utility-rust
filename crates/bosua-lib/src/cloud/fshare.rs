//! FShare integration client.
//!
//! Provides VIP download link resolution, multi-threaded downloads,
//! folder scanning, and session-based authentication for the FShare
//! Vietnamese file sharing service.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::cloud::CloudClient;
use crate::errors::{BosuaError, Result};
use crate::http_client::HttpClient;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const _FSHARE_API_BASE: &str = "https://api.fshare.vn/api";
const FSHARE_LOGIN_URL: &str = "https://api.fshare.vn/api/user/login";
const FSHARE_DOWNLOAD_URL: &str = "https://api.fshare.vn/api/session/download";
const FSHARE_FOLDER_URL: &str = "https://api.fshare.vn/api/fileops/getFolderList";

// ---------------------------------------------------------------------------
// Data models
// ---------------------------------------------------------------------------

/// FShare login request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FShareLoginRequest {
    #[serde(rename = "user_email")]
    pub email: String,
    pub password: String,
    #[serde(rename = "app_key")]
    pub app_key: String,
}

/// FShare login response containing the session token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FShareLoginResponse {
    pub token: Option<String>,
    #[serde(rename = "session_id")]
    pub session_id: Option<String>,
    pub code: Option<i32>,
    pub msg: Option<String>,
}

/// Request to resolve a VIP download link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FShareDownloadRequest {
    pub url: String,
    pub token: String,
    pub password: Option<String>,
}

/// Response from VIP download link resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FShareDownloadResponse {
    pub location: Option<String>,
    pub code: Option<i32>,
    pub msg: Option<String>,
}

/// A file entry from an FShare folder listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FShareFileEntry {
    pub id: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub file_type: Option<i32>,
    pub size: Option<u64>,
    #[serde(rename = "linkcode")]
    pub link_code: Option<String>,
    #[serde(rename = "folderId")]
    pub folder_id: Option<String>,
    pub path: Option<String>,
    #[serde(rename = "created")]
    pub created: Option<String>,
    #[serde(rename = "modified")]
    pub modified: Option<String>,
}

/// Response from folder listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FShareFolderResponse {
    #[serde(default)]
    pub files: Vec<FShareFileEntry>,
    pub code: Option<i32>,
    pub msg: Option<String>,
    #[serde(rename = "current")]
    pub current_page: Option<u32>,
    #[serde(rename = "total")]
    pub total_pages: Option<u32>,
}

// ---------------------------------------------------------------------------
// FShareClient
// ---------------------------------------------------------------------------

/// FShare client for VIP link resolution, folder scanning, and downloads.
///
/// Authenticates via email + password to obtain a session token, then uses
/// that token for VIP download link resolution and folder operations.
pub struct FShareClient {
    http: HttpClient,
    email: String,
    password: String,
    app_key: String,
    token: tokio::sync::RwLock<Option<String>>,
    session_id: tokio::sync::RwLock<Option<String>>,
}

impl FShareClient {
    /// Create a new `FShareClient`.
    ///
    /// * `http` – shared HTTP client
    /// * `email` – FShare account email
    /// * `password` – FShare account password
    /// * `app_key` – FShare API application key
    pub fn new(http: HttpClient, email: String, password: String, app_key: String) -> Self {
        Self {
            http,
            email,
            password,
            app_key,
            token: tokio::sync::RwLock::new(None),
            session_id: tokio::sync::RwLock::new(None),
        }
    }

    /// Log in to FShare and store the session token.
    pub async fn login(&self) -> Result<FShareLoginResponse> {
        let client = self.http.get_client().await;
        let req = FShareLoginRequest {
            email: self.email.clone(),
            password: self.password.clone(),
            app_key: self.app_key.clone(),
        };

        let resp = client
            .post(FSHARE_LOGIN_URL)
            .json(&req)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "fshare".into(),
                message: format!("login failed ({status}): {body}"),
            });
        }

        let login_resp: FShareLoginResponse = resp.json().await.map_err(BosuaError::Http)?;

        if let Some(ref token) = login_resp.token {
            *self.token.write().await = Some(token.clone());
        }
        if let Some(ref sid) = login_resp.session_id {
            *self.session_id.write().await = Some(sid.clone());
        }

        Ok(login_resp)
    }

    /// Get the current session token, or `None` if not logged in.
    pub async fn get_token(&self) -> Option<String> {
        self.token.read().await.clone()
    }

    /// Set the session token directly (e.g. from a saved token).
    pub async fn set_token(&self, token: String) {
        *self.token.write().await = Some(token);
    }

    /// Resolve a VIP download link from an FShare URL.
    ///
    /// Returns the direct download URL on success.
    pub async fn resolve_vip_link(&self, fshare_url: &str) -> Result<String> {
        let token = self.token.read().await.clone().ok_or_else(|| {
            BosuaError::Auth("not logged in to FShare — call login() first".into())
        })?;

        let client = self.http.get_client().await;
        let req = FShareDownloadRequest {
            url: fshare_url.to_string(),
            token: token.clone(),
            password: None,
        };

        let resp = client
            .post(FSHARE_DOWNLOAD_URL)
            .header("Cookie", format!("session_id={}", self.session_id_or_empty().await))
            .json(&req)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "fshare".into(),
                message: format!("VIP link resolution failed ({status}): {body}"),
            });
        }

        let dl_resp: FShareDownloadResponse = resp.json().await.map_err(BosuaError::Http)?;

        dl_resp.location.ok_or_else(|| BosuaError::Cloud {
            service: "fshare".into(),
            message: dl_resp
                .msg
                .unwrap_or_else(|| "no download location returned".into()),
        })
    }

    /// Resolve multiple VIP download links concurrently.
    ///
    /// Returns a vec of `(url, Result<direct_link>)` pairs.
    pub async fn resolve_vip_links(
        &self,
        urls: &[String],
    ) -> Vec<(String, Result<String>)> {
        let mut handles = Vec::with_capacity(urls.len());

        for url in urls {
            let url = url.clone();
            let token = self.token.read().await.clone();
            let session_id = self.session_id.read().await.clone();
            let client = self.http.get_client().await;

            handles.push(tokio::spawn(async move {
                let token = match token {
                    Some(t) => t,
                    None => {
                        return (
                            url,
                            Err(BosuaError::Auth("not logged in to FShare".into())),
                        );
                    }
                };

                let req = FShareDownloadRequest {
                    url: url.clone(),
                    token,
                    password: None,
                };

                let sid = session_id.unwrap_or_default();
                let resp = client
                    .post(FSHARE_DOWNLOAD_URL)
                    .header("Cookie", format!("session_id={sid}"))
                    .json(&req)
                    .send()
                    .await;

                match resp {
                    Ok(r) if r.status().is_success() => {
                        match r.json::<FShareDownloadResponse>().await {
                            Ok(dl) => match dl.location {
                                Some(loc) => (url, Ok(loc)),
                                None => (
                                    url,
                                    Err(BosuaError::Cloud {
                                        service: "fshare".into(),
                                        message: dl.msg.unwrap_or_else(|| {
                                            "no download location".into()
                                        }),
                                    }),
                                ),
                            },
                            Err(e) => (url, Err(BosuaError::Http(e))),
                        }
                    }
                    Ok(r) => {
                        let status = r.status().as_u16();
                        let body = r.text().await.unwrap_or_default();
                        (
                            url,
                            Err(BosuaError::Cloud {
                                service: "fshare".into(),
                                message: format!("VIP link failed ({status}): {body}"),
                            }),
                        )
                    }
                    Err(e) => (url, Err(BosuaError::Http(e))),
                }
            }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push((
                    String::new(),
                    Err(BosuaError::Application(format!("task join error: {e}"))),
                )),
            }
        }
        results
    }

    /// List files in an FShare folder.
    ///
    /// * `link_code` – the folder's link code (from the FShare URL)
    /// * `page` – page number (1-based, `None` for first page)
    pub async fn scan_folder(
        &self,
        link_code: &str,
        page: Option<u32>,
    ) -> Result<FShareFolderResponse> {
        let token = self.token.read().await.clone().ok_or_else(|| {
            BosuaError::Auth("not logged in to FShare — call login() first".into())
        })?;

        let client = self.http.get_client().await;

        let mut params = vec![
            ("linkcode", link_code.to_string()),
            ("token", token),
        ];
        if let Some(p) = page {
            params.push(("page", p.to_string()));
        }

        let resp = client
            .get(FSHARE_FOLDER_URL)
            .header(
                "Cookie",
                format!("session_id={}", self.session_id_or_empty().await),
            )
            .query(&params)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "fshare".into(),
                message: format!("folder scan failed ({status}): {body}"),
            });
        }

        resp.json::<FShareFolderResponse>()
            .await
            .map_err(BosuaError::Http)
    }

    /// Helper to get the session ID or an empty string.
    async fn session_id_or_empty(&self) -> String {
        self.session_id
            .read()
            .await
            .clone()
            .unwrap_or_default()
    }
}

#[async_trait]
impl CloudClient for FShareClient {
    fn name(&self) -> &str {
        "FShare"
    }

    async fn authenticate(&self) -> Result<()> {
        let resp = self.login().await?;
        if resp.token.is_some() {
            Ok(())
        } else {
            Err(BosuaError::Auth(
                resp.msg
                    .unwrap_or_else(|| "FShare login returned no token".into()),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_client() -> FShareClient {
        let http = HttpClient::from_defaults().expect("http client");
        FShareClient::new(
            http,
            "test@example.com".into(),
            "password123".into(),
            "test-app-key".into(),
        )
    }

    #[test]
    fn test_new_client() {
        let client = make_test_client();
        assert_eq!(client.name(), "FShare");
        assert_eq!(client.email, "test@example.com");
        assert_eq!(client.app_key, "test-app-key");
    }

    #[tokio::test]
    async fn test_get_token_initially_none() {
        let client = make_test_client();
        assert!(client.get_token().await.is_none());
    }

    #[tokio::test]
    async fn test_set_and_get_token() {
        let client = make_test_client();
        client.set_token("my-session-token".into()).await;
        assert_eq!(client.get_token().await, Some("my-session-token".into()));
    }

    #[tokio::test]
    async fn test_resolve_vip_link_requires_auth() {
        let client = make_test_client();
        let result = client.resolve_vip_link("https://www.fshare.vn/file/ABC123").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not logged in"));
    }

    #[tokio::test]
    async fn test_scan_folder_requires_auth() {
        let client = make_test_client();
        let result = client.scan_folder("FOLDER123", None).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not logged in"));
    }

    #[test]
    fn test_login_request_serde() {
        let req = FShareLoginRequest {
            email: "user@example.com".into(),
            password: "pass".into(),
            app_key: "key123".into(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["user_email"], "user@example.com");
        assert_eq!(json["password"], "pass");
        assert_eq!(json["app_key"], "key123");

        let roundtrip: FShareLoginRequest = serde_json::from_value(json).unwrap();
        assert_eq!(roundtrip.email, "user@example.com");
    }

    #[test]
    fn test_login_response_serde() {
        let json = serde_json::json!({
            "token": "abc123",
            "session_id": "sess456",
            "code": 200,
            "msg": "OK"
        });
        let resp: FShareLoginResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.token, Some("abc123".into()));
        assert_eq!(resp.session_id, Some("sess456".into()));
        assert_eq!(resp.code, Some(200));
    }

    #[test]
    fn test_download_response_serde() {
        let json = serde_json::json!({
            "location": "https://download.fshare.vn/direct/abc123",
            "code": 200
        });
        let resp: FShareDownloadResponse = serde_json::from_value(json).unwrap();
        assert_eq!(
            resp.location,
            Some("https://download.fshare.vn/direct/abc123".into())
        );
    }

    #[test]
    fn test_file_entry_serde() {
        let json = serde_json::json!({
            "name": "movie.mkv",
            "type": 1,
            "size": 1073741824_u64,
            "linkcode": "XYZ789",
            "folderId": "FOLDER1"
        });
        let entry: FShareFileEntry = serde_json::from_value(json).unwrap();
        assert_eq!(entry.name, "movie.mkv");
        assert_eq!(entry.file_type, Some(1));
        assert_eq!(entry.size, Some(1073741824));
        assert_eq!(entry.link_code, Some("XYZ789".into()));
    }

    #[test]
    fn test_folder_response_serde() {
        let json = serde_json::json!({
            "files": [
                { "name": "file1.txt", "size": 100 },
                { "name": "file2.txt", "size": 200 }
            ],
            "code": 200,
            "current": 1,
            "total": 3
        });
        let resp: FShareFolderResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.files.len(), 2);
        assert_eq!(resp.current_page, Some(1));
        assert_eq!(resp.total_pages, Some(3));
    }

    #[test]
    fn test_download_request_serde() {
        let req = FShareDownloadRequest {
            url: "https://www.fshare.vn/file/ABC".into(),
            token: "tok123".into(),
            password: Some("secret".into()),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["url"], "https://www.fshare.vn/file/ABC");
        assert_eq!(json["token"], "tok123");
        assert_eq!(json["password"], "secret");

        let roundtrip: FShareDownloadRequest = serde_json::from_value(json).unwrap();
        assert_eq!(roundtrip.url, "https://www.fshare.vn/file/ABC");
    }
}
