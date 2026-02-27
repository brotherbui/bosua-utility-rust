//! Aria2 JSON-RPC 2.0 client for advanced download management.
//!
//! Sends JSON-RPC requests to a configurable Aria2 daemon endpoint.
//! Supports common Aria2 methods: `addUri`, `tellStatus`, `remove`,
//! `pause`, `unpause`, and `getGlobalStat`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::{BosuaError, Result};
use crate::http_client::HttpClient;

/// Default Aria2 JSON-RPC endpoint.
const DEFAULT_ARIA2_ENDPOINT: &str = "http://localhost:6800/jsonrpc";

/// A JSON-RPC 2.0 request payload for Aria2.
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: String,
    method: String,
    params: Vec<Value>,
}

/// A JSON-RPC 2.0 response from Aria2.
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    id: Option<Value>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

/// Client for communicating with an Aria2 daemon via JSON-RPC 2.0.
pub struct Aria2Client {
    http_client: HttpClient,
    endpoint: String,
    token: Option<String>,
}

impl Aria2Client {
    /// Create a new `Aria2Client`.
    ///
    /// - `http_client` — shared HTTP client for making requests
    /// - `endpoint` — Aria2 JSON-RPC endpoint URL (e.g. `http://localhost:6800/jsonrpc`)
    /// - `token` — optional Aria2 RPC secret token
    pub fn new(http_client: HttpClient, endpoint: Option<String>, token: Option<String>) -> Self {
        Self {
            http_client,
            endpoint: endpoint.unwrap_or_else(|| DEFAULT_ARIA2_ENDPOINT.to_string()),
            token,
        }
    }

    /// Send a JSON-RPC 2.0 request to the Aria2 daemon.
    ///
    /// Prepends the secret token to `params` if one is configured.
    /// Returns the `result` field from the response, or an error if the
    /// RPC call failed.
    pub async fn aria2_rpc(&self, method: &str, params: Vec<Value>) -> Result<Value> {
        let mut rpc_params = Vec::with_capacity(params.len() + 1);

        // Aria2 expects the secret token as the first parameter when configured.
        if let Some(ref tok) = self.token {
            rpc_params.push(Value::String(format!("token:{}", tok)));
        }
        rpc_params.extend(params);

        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: uuid_v4_simple(),
            method: format!("aria2.{}", method),
            params: rpc_params,
        };

        let client = self.http_client.get_client().await;
        let response = client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        let status = response.status();
        if !status.is_success() {
            return Err(BosuaError::Download(format!(
                "Aria2 RPC HTTP error: {}",
                status
            )));
        }

        let rpc_response: JsonRpcResponse = response.json().await.map_err(BosuaError::Http)?;

        if let Some(err) = rpc_response.error {
            return Err(BosuaError::Download(format!(
                "Aria2 RPC error ({}): {}",
                err.code, err.message
            )));
        }

        rpc_response
            .result
            .ok_or_else(|| BosuaError::Download("Aria2 RPC returned no result".into()))
    }

    /// Add a download by URI.
    ///
    /// `uris` — one or more mirrors for the same file.
    /// `options` — optional Aria2 download options (e.g. `dir`, `out`).
    pub async fn add_uri(
        &self,
        uris: Vec<String>,
        options: Option<Value>,
    ) -> Result<Value> {
        let mut params = vec![Value::Array(
            uris.into_iter().map(Value::String).collect(),
        )];
        if let Some(opts) = options {
            params.push(opts);
        }
        self.aria2_rpc("addUri", params).await
    }

    /// Query the status of a download.
    ///
    /// `gid` — the Aria2 download GID.
    /// `keys` — optional list of status keys to return (returns all if empty).
    pub async fn tell_status(&self, gid: &str, keys: Option<Vec<String>>) -> Result<Value> {
        let mut params = vec![Value::String(gid.to_string())];
        if let Some(k) = keys {
            params.push(Value::Array(k.into_iter().map(Value::String).collect()));
        }
        self.aria2_rpc("tellStatus", params).await
    }

    /// Remove a download.
    ///
    /// `gid` — the Aria2 download GID.
    pub async fn remove(&self, gid: &str) -> Result<Value> {
        self.aria2_rpc("remove", vec![Value::String(gid.to_string())])
            .await
    }

    /// Pause a download.
    ///
    /// `gid` — the Aria2 download GID.
    pub async fn pause(&self, gid: &str) -> Result<Value> {
        self.aria2_rpc("pause", vec![Value::String(gid.to_string())])
            .await
    }

    /// Unpause a paused download.
    ///
    /// `gid` — the Aria2 download GID.
    pub async fn unpause(&self, gid: &str) -> Result<Value> {
        self.aria2_rpc("unpause", vec![Value::String(gid.to_string())])
            .await
    }

    /// Get global download statistics.
    pub async fn get_global_stat(&self) -> Result<Value> {
        self.aria2_rpc("getGlobalStat", vec![]).await
    }
}

/// Generate a simple unique ID for JSON-RPC requests.
/// Uses a timestamp + counter approach to avoid pulling in a UUID crate.
fn uuid_v4_simple() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("bosua-{:x}-{:x}", ts, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_v4_simple_unique() {
        let a = uuid_v4_simple();
        let b = uuid_v4_simple();
        assert_ne!(a, b);
    }

    #[test]
    fn test_json_rpc_request_serialization() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            id: "test-1".to_string(),
            method: "aria2.addUri".to_string(),
            params: vec![Value::Array(vec![Value::String(
                "https://example.com/file.zip".to_string(),
            )])],
        };

        let json = serde_json::to_value(&req).expect("should serialize");
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], "test-1");
        assert_eq!(json["method"], "aria2.addUri");
        assert!(json["params"].is_array());
    }

    #[test]
    fn test_json_rpc_response_deserialization_success() {
        let json = r#"{"id":"test-1","jsonrpc":"2.0","result":"2089b05ecca3d829"}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).expect("should deserialize");
        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap(), "2089b05ecca3d829");
    }

    #[test]
    fn test_json_rpc_response_deserialization_error() {
        let json = r#"{"id":"test-1","jsonrpc":"2.0","error":{"code":1,"message":"GID not found"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).expect("should deserialize");
        assert!(resp.result.is_none());
        let err = resp.error.unwrap();
        assert_eq!(err.code, 1);
        assert_eq!(err.message, "GID not found");
    }

    #[test]
    fn test_aria2_client_new_defaults() {
        let http_client = HttpClient::from_defaults().expect("should build");
        let client = Aria2Client::new(http_client, None, None);
        assert_eq!(client.endpoint, DEFAULT_ARIA2_ENDPOINT);
        assert!(client.token.is_none());
    }

    #[test]
    fn test_aria2_client_new_custom() {
        let http_client = HttpClient::from_defaults().expect("should build");
        let client = Aria2Client::new(
            http_client,
            Some("http://myhost:6800/jsonrpc".to_string()),
            Some("mysecret".to_string()),
        );
        assert_eq!(client.endpoint, "http://myhost:6800/jsonrpc");
        assert_eq!(client.token.as_deref(), Some("mysecret"));
    }
}
