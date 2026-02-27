//! HTTP endpoint handlers.
//!
//! Each handler delegates to the service layer via `AppState` and returns
//! JSON responses. Handlers that don't need state remain stateless.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::errors::BosuaError;
use crate::search::SearchSource;

use super::AppState;

/// Maps a `BosuaError` to an HTTP status code and JSON error body.
pub fn error_to_response(err: &BosuaError) -> (StatusCode, Json<serde_json::Value>) {
    let status = match err {
        BosuaError::Auth(_) | BosuaError::OAuth2(_) => StatusCode::UNAUTHORIZED,
        BosuaError::Cloud { .. } | BosuaError::Http(_) => StatusCode::SERVICE_UNAVAILABLE,
        BosuaError::Config(_) | BosuaError::Command(_) => StatusCode::BAD_REQUEST,
        BosuaError::LockConflict { .. } => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(json!({
            "success": false,
            "error": err.to_string()
        })),
    )
}

/// Standard stub response for unimplemented endpoints.
fn stub_response(endpoint: &str) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": format!("{} endpoint not yet wired to HTTP", endpoint)
        })),
    )
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

pub async fn health() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({ "success": true, "message": "ok" })),
    )
}

// ---------------------------------------------------------------------------
// Links
// ---------------------------------------------------------------------------

pub async fn append_links() -> impl IntoResponse {
    stub_response("append-links")
}

// ---------------------------------------------------------------------------
// Config (Task 14.1)
// ---------------------------------------------------------------------------

pub async fn get_config(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let config = state.config_manager.get_config().await;
    (StatusCode::OK, Json(json!({ "success": true, "data": config })))
}

#[derive(Deserialize)]
pub struct UpdateConfigRequest {
    #[serde(flatten)]
    pub fields: serde_json::Map<String, serde_json::Value>,
}

pub async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpdateConfigRequest>,
) -> impl IntoResponse {
    match state.config_manager.update_config(body.fields).await {
        Ok(()) => {
            let config = state.config_manager.get_config().await;
            (StatusCode::OK, Json(json!({ "success": true, "data": config })))
        }
        Err(e) => {
            let (status, json) = error_to_response(&e);
            (status, json)
        }
    }
}

pub async fn reset_config(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.config_manager.reset_to_defaults().await {
        Ok(()) => {
            let config = state.config_manager.get_config().await;
            (StatusCode::OK, Json(json!({ "success": true, "data": config, "message": "Config reset to defaults" })))
        }
        Err(e) => {
            let (status, json) = error_to_response(&e);
            (status, json)
        }
    }
}

// ---------------------------------------------------------------------------
// Search (Task 14.2)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct SearchRequest {
    pub source: SearchSource,
    pub query: Option<String>,
}

pub async fn search(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SearchRequest>,
) -> impl IntoResponse {
    match state.search_engine.search(body.source, body.query.as_deref()).await {
        Ok(results) => {
            (StatusCode::OK, Json(json!({ "success": true, "data": results })))
        }
        Err(e) => {
            let (status, json) = error_to_response(&e);
            (status, json)
        }
    }
}

// ---------------------------------------------------------------------------
// FShare (Task 14.2)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct FShareLinksRequest {
    pub urls: Vec<String>,
}

pub async fn fshare_links(
    State(state): State<Arc<AppState>>,
    Json(body): Json<FShareLinksRequest>,
) -> impl IntoResponse {
    let results = state.fshare.resolve_vip_links(&body.urls).await;
    let resolved: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(url, result)| match result {
            Ok(direct_url) => json!({ "url": url, "direct_url": direct_url }),
            Err(e) => json!({ "url": url, "error": e.to_string() }),
        })
        .collect();
    (StatusCode::OK, Json(json!({ "success": true, "data": resolved })))
}

pub async fn fshare_scan() -> impl IntoResponse {
    stub_response("fshare-scan")
}

// ---------------------------------------------------------------------------
// Daemon (Task 14.3)
// ---------------------------------------------------------------------------

pub async fn daemon_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.daemon_manager.status() {
        Ok(status) => {
            (StatusCode::OK, Json(json!({ "success": true, "data": { "status": status.to_string() } })))
        }
        Err(e) => {
            let (status, json) = error_to_response(&e);
            (status, json)
        }
    }
}

// ---------------------------------------------------------------------------
// GDrive (Task 14.3)
// ---------------------------------------------------------------------------

pub async fn gdrive_files(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.gdrive.list_files(None, None, None).await {
        Ok(file_list) => {
            (StatusCode::OK, Json(json!({ "success": true, "data": file_list })))
        }
        Err(e) => {
            let (status, json) = error_to_response(&e);
            (status, json)
        }
    }
}

#[derive(Deserialize)]
pub struct GDriveUploadRequest {
    pub filename: String,
    /// Base64-encoded file content.
    pub content: String,
    pub parent_id: Option<String>,
}

pub async fn gdrive_upload(
    State(state): State<Arc<AppState>>,
    Json(body): Json<GDriveUploadRequest>,
) -> impl IntoResponse {
    use base64::Engine;

    // Decode base64 content
    let bytes = match base64::engine::general_purpose::STANDARD.decode(&body.content) {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": format!("Invalid base64 content: {}", e) })),
            );
        }
    };

    // Write to a temp file and upload
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(&body.filename);
    if let Err(e) = tokio::fs::write(&temp_path, &bytes).await {
        return error_to_response(&BosuaError::Io(e));
    }

    let result = state
        .gdrive
        .upload_file(&temp_path, body.parent_id.as_deref())
        .await;

    // Clean up temp file (best effort)
    let _ = tokio::fs::remove_file(&temp_path).await;

    match result {
        Ok(file) => (StatusCode::OK, Json(json!({ "success": true, "data": file }))),
        Err(e) => {
            let (status, json) = error_to_response(&e);
            (status, json)
        }
    }
}

pub async fn gdrive_proxy() -> impl IntoResponse {
    stub_response("gdrive-proxy")
}

// ---------------------------------------------------------------------------
// Stats (Task 14.3)
// ---------------------------------------------------------------------------

pub async fn stats() -> impl IntoResponse {
    let uptime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "uptime_epoch_secs": uptime,
                "active_downloads": 0,
            }
        })),
    )
}

// ---------------------------------------------------------------------------
// Logs (Task 14.3)
// ---------------------------------------------------------------------------

pub async fn logs(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.daemon_manager.logs(100) {
        Ok(log_output) => {
            let lines: Vec<&str> = log_output.lines().collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": { "lines": lines } })))
        }
        Err(e) => {
            let (status, json) = error_to_response(&e);
            (status, json)
        }
    }
}

// ---------------------------------------------------------------------------
// Notifications
// ---------------------------------------------------------------------------

pub async fn list_notifications() -> impl IntoResponse {
    stub_response("list-notifications")
}

pub async fn notifications_sse() -> impl IntoResponse {
    stub_response("notifications-sse")
}

// ---------------------------------------------------------------------------
// Files
// ---------------------------------------------------------------------------

pub async fn list_files() -> impl IntoResponse {
    stub_response("list-files")
}

pub async fn upload_file() -> impl IntoResponse {
    stub_response("upload-file")
}

pub async fn delete_file() -> impl IntoResponse {
    stub_response("delete-file")
}

// ---------------------------------------------------------------------------
// Memprofile
// ---------------------------------------------------------------------------

pub async fn memprofile() -> impl IntoResponse {
    stub_response("memprofile")
}

// ---------------------------------------------------------------------------
// PDF / LaTeX
// ---------------------------------------------------------------------------

pub async fn pdf() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "PDF processing endpoint",
            "operations": ["merge", "split", "enhance", "compress"]
        })),
    )
}

pub async fn latex2pdf() -> impl IntoResponse {
    stub_response("latex2pdf")
}

// ---------------------------------------------------------------------------
// Onflix
// ---------------------------------------------------------------------------

pub async fn onflix() -> impl IntoResponse {
    stub_response("onflix")
}

// ---------------------------------------------------------------------------
// Cloud providers (Task 14.4) — stubs delegating to cloud clients
// ---------------------------------------------------------------------------

// GCloud
pub async fn gcloud_instances() -> impl IntoResponse {
    stub_response("gcloud-instances")
}

// GCP
pub async fn gcp_browse() -> impl IntoResponse {
    stub_response("gcp-browse")
}

pub async fn gcp_download() -> impl IntoResponse {
    stub_response("gcp-download")
}

pub async fn gcp_list() -> impl IntoResponse {
    stub_response("gcp-list")
}

pub async fn gcp_play() -> impl IntoResponse {
    stub_response("gcp-play")
}

pub async fn gcp_push() -> impl IntoResponse {
    stub_response("gcp-push")
}

// AWS
pub async fn aws_ec2() -> impl IntoResponse {
    stub_response("aws-ec2")
}

pub async fn aws_ec2_start() -> impl IntoResponse {
    stub_response("aws-ec2-start")
}

pub async fn aws_ec2_stop() -> impl IntoResponse {
    stub_response("aws-ec2-stop")
}

pub async fn aws_sg() -> impl IntoResponse {
    stub_response("aws-sg")
}

pub async fn aws_regions() -> impl IntoResponse {
    stub_response("aws-regions")
}

pub async fn aws_zones() -> impl IntoResponse {
    stub_response("aws-zones")
}

// Tailscale
pub async fn tailscale_devices() -> impl IntoResponse {
    stub_response("tailscale-devices")
}

pub async fn tailscale_acl() -> impl IntoResponse {
    stub_response("tailscale-acl")
}

pub async fn tailscale_routes() -> impl IntoResponse {
    stub_response("tailscale-routes")
}

pub async fn tailscale_keys() -> impl IntoResponse {
    stub_response("tailscale-keys")
}

// WordPress
pub async fn wordpress() -> impl IntoResponse {
    stub_response("wordpress")
}

// Cache
pub async fn get_cache() -> impl IntoResponse {
    stub_response("get-cache")
}

pub async fn clear_cache() -> impl IntoResponse {
    stub_response("clear-cache")
}

// Accounts
pub async fn accounts() -> impl IntoResponse {
    stub_response("accounts")
}

// Instance
pub async fn instance_create() -> impl IntoResponse {
    stub_response("instance-create")
}

// Kodi
pub async fn kodi_repo() -> impl IntoResponse {
    stub_response("kodi-repo")
}

// Media
pub async fn play() -> impl IntoResponse {
    stub_response("play")
}


// ---------------------------------------------------------------------------
// Stub fallbacks (used when AppState is not available)
// ---------------------------------------------------------------------------

pub async fn get_config_stub() -> impl IntoResponse {
    stub_response("get-config")
}

pub async fn update_config_stub() -> impl IntoResponse {
    stub_response("update-config")
}

pub async fn reset_config_stub() -> impl IntoResponse {
    stub_response("reset-config")
}

pub async fn search_stub() -> impl IntoResponse {
    stub_response("search")
}

pub async fn fshare_links_stub() -> impl IntoResponse {
    stub_response("fshare-links")
}

pub async fn daemon_status_stub() -> impl IntoResponse {
    stub_response("daemon-status")
}

pub async fn gdrive_files_stub() -> impl IntoResponse {
    stub_response("gdrive-files")
}

pub async fn gdrive_upload_stub() -> impl IntoResponse {
    stub_response("gdrive-upload")
}

pub async fn logs_stub() -> impl IntoResponse {
    stub_response("logs")
}
