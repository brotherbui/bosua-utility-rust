//! Tailscale integration client.
//!
//! Provides OAuth-based authentication with the Tailscale API, device
//! listing/management, ACL policy management, and route advertisement.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::cloud::CloudClient;
use crate::errors::{BosuaError, Result};
use crate::http_client::HttpClient;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TS_API_BASE: &str = "https://api.tailscale.com/api/v2";

// ---------------------------------------------------------------------------
// Data models
// ---------------------------------------------------------------------------

/// A Tailscale device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsDevice {
    pub id: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub addresses: Vec<String>,
    #[serde(rename = "os", default)]
    pub os: String,
    #[serde(rename = "lastSeen", default)]
    pub last_seen: Option<String>,
    #[serde(default)]
    pub online: bool,
}

/// Tailscale ACL policy document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsAclPolicy {
    #[serde(default)]
    pub acls: Vec<serde_json::Value>,
    #[serde(default)]
    pub groups: serde_json::Value,
    #[serde(default)]
    pub hosts: serde_json::Value,
}

/// A Tailscale route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsRoute {
    pub id: String,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub advertised: bool,
    #[serde(default)]
    pub enabled: bool,
}

/// A Tailscale auth key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsAuthKey {
    pub id: String,
    pub key: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub revoked: bool,
    #[serde(rename = "expiresAt", default)]
    pub expires_at: Option<String>,
}

// ---------------------------------------------------------------------------
// TailscaleClient
// ---------------------------------------------------------------------------

/// Tailscale client for device management, ACLs, and route advertisement.
///
/// Authenticates via an OAuth access token or API key.
pub struct TailscaleClient {
    http: HttpClient,
    api_key: String,
    tailnet: String,
}

impl TailscaleClient {
    /// Create a new `TailscaleClient`.
    ///
    /// * `http` – shared HTTP client
    /// * `api_key` – Tailscale API key or OAuth access token
    /// * `tailnet` – the tailnet name (e.g. "example.com" or org name)
    pub fn new(http: HttpClient, api_key: String, tailnet: String) -> Self {
        Self {
            http,
            api_key,
            tailnet,
        }
    }

    /// Helper to make an authenticated GET request.
    async fn ts_get<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let client = self.http.get_client().await;
        let resp = client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "tailscale".into(),
                message: format!("API request failed ({status}): {body}"),
            });
        }

        resp.json::<T>().await.map_err(BosuaError::Http)
    }

    // -----------------------------------------------------------------------
    // Device management
    // -----------------------------------------------------------------------

    /// List devices in the tailnet.
    pub async fn list_devices(&self) -> Result<Vec<TsDevice>> {
        let url = format!("{TS_API_BASE}/tailnet/{}/devices", self.tailnet);

        #[derive(Deserialize)]
        struct DevicesResponse {
            devices: Vec<TsDevice>,
        }

        let resp: DevicesResponse = self.ts_get(&url).await?;
        Ok(resp.devices)
    }

    /// Get a single device by ID.
    pub async fn get_device(&self, device_id: &str) -> Result<TsDevice> {
        let url = format!("{TS_API_BASE}/device/{device_id}");
        self.ts_get::<TsDevice>(&url).await
    }

    /// Delete a device from the tailnet.
    pub async fn delete_device(&self, device_id: &str) -> Result<()> {
        let client = self.http.get_client().await;
        let url = format!("{TS_API_BASE}/device/{device_id}");

        let resp = client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "tailscale".into(),
                message: format!("delete_device failed ({status}): {body}"),
            });
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // ACL policy management
    // -----------------------------------------------------------------------

    /// Get the current ACL policy.
    pub async fn get_acl(&self) -> Result<TsAclPolicy> {
        let url = format!("{TS_API_BASE}/tailnet/{}/acl", self.tailnet);
        self.ts_get::<TsAclPolicy>(&url).await
    }

    /// Update the ACL policy.
    pub async fn set_acl(&self, policy: &TsAclPolicy) -> Result<()> {
        let client = self.http.get_client().await;
        let url = format!("{TS_API_BASE}/tailnet/{}/acl", self.tailnet);

        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(policy)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "tailscale".into(),
                message: format!("set_acl failed ({status}): {body}"),
            });
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Route advertisement
    // -----------------------------------------------------------------------

    /// Get routes for a device.
    pub async fn get_device_routes(&self, device_id: &str) -> Result<Vec<TsRoute>> {
        let url = format!("{TS_API_BASE}/device/{device_id}/routes");

        #[derive(Deserialize)]
        struct RoutesResponse {
            routes: Vec<TsRoute>,
        }

        let resp: RoutesResponse = self.ts_get(&url).await?;
        Ok(resp.routes)
    }

    /// Enable or disable routes on a device.
    pub async fn set_device_routes(
        &self,
        device_id: &str,
        routes: &[String],
    ) -> Result<()> {
        let client = self.http.get_client().await;
        let url = format!("{TS_API_BASE}/device/{device_id}/routes");

        let body = serde_json::json!({ "routes": routes });

        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "tailscale".into(),
                message: format!("set_device_routes failed ({status}): {text}"),
            });
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Auth keys
    // -----------------------------------------------------------------------

    /// List auth keys for the tailnet.
    pub async fn list_keys(&self) -> Result<Vec<TsAuthKey>> {
        let url = format!("{TS_API_BASE}/tailnet/{}/keys", self.tailnet);

        #[derive(Deserialize)]
        struct KeysResponse {
            keys: Vec<TsAuthKey>,
        }

        let resp: KeysResponse = self.ts_get(&url).await?;
        Ok(resp.keys)
    }
}

// ---------------------------------------------------------------------------
// CloudClient trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl CloudClient for TailscaleClient {
    fn name(&self) -> &str {
        "Tailscale"
    }

    /// Authenticate by verifying the API key against the Tailscale API.
    async fn authenticate(&self) -> Result<()> {
        // Attempt to list devices as a connectivity/auth check
        let client = self.http.get_client().await;
        let url = format!("{TS_API_BASE}/tailnet/{}/devices", self.tailnet);

        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            return Err(BosuaError::Auth(
                "Tailscale authentication failed".into(),
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tailscale_client_name() {
        let http = HttpClient::from_defaults().unwrap();
        let client = TailscaleClient::new(http, "tskey-test".into(), "example.com".into());
        assert_eq!(client.name(), "Tailscale");
    }

    #[test]
    fn test_device_serialization() {
        let device = TsDevice {
            id: "dev123".into(),
            hostname: "my-laptop".into(),
            name: "my-laptop.tail1234.ts.net".into(),
            addresses: vec!["100.64.0.1".into()],
            os: "linux".into(),
            last_seen: Some("2024-01-01T00:00:00Z".into()),
            online: true,
        };
        let json = serde_json::to_string(&device).unwrap();
        let deser: TsDevice = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.hostname, "my-laptop");
        assert!(deser.online);
    }

    #[test]
    fn test_route_serialization() {
        let route = TsRoute {
            id: "route123".into(),
            prefix: "10.0.0.0/24".into(),
            advertised: true,
            enabled: true,
        };
        let json = serde_json::to_string(&route).unwrap();
        let deser: TsRoute = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.prefix, "10.0.0.0/24");
        assert!(deser.enabled);
    }

    #[test]
    fn test_auth_key_serialization() {
        let key = TsAuthKey {
            id: "key123".into(),
            key: "tskey-auth-xxx".into(),
            description: "test key".into(),
            revoked: false,
            expires_at: Some("2025-01-01T00:00:00Z".into()),
        };
        let json = serde_json::to_string(&key).unwrap();
        let deser: TsAuthKey = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.key, "tskey-auth-xxx");
        assert!(!deser.revoked);
    }
}
