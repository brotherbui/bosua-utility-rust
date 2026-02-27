//! Cloudflare integration client.
//!
//! Provides DNS record management, Tunnel management, page rules/ruleset
//! management, domain listing, and configuration validation.
//! Uses the Cloudflare REST API via `reqwest`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::cloud::CloudClient;
use crate::errors::{BosuaError, Result};
use crate::http_client::HttpClient;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CF_API_BASE: &str = "https://api.cloudflare.com/client/v4";

// ---------------------------------------------------------------------------
// Data models
// ---------------------------------------------------------------------------

/// A Cloudflare DNS record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    pub id: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub name: String,
    pub content: String,
    #[serde(default)]
    pub ttl: u32,
    #[serde(default)]
    pub proxied: bool,
}

/// A Cloudflare Tunnel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfTunnel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(rename = "created_at", default)]
    pub created_at: Option<String>,
}

/// A Cloudflare page rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRule {
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub priority: u32,
}

/// A Cloudflare ruleset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ruleset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub phase: String,
}

/// A Cloudflare zone (domain).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfZone {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub paused: bool,
}

/// Result of a configuration validation check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Generic Cloudflare API response wrapper.
#[derive(Debug, Deserialize)]
struct CfApiResponse<T> {
    pub success: bool,
    pub result: Option<T>,
    #[serde(default)]
    pub errors: Vec<CfApiError>,
}

#[derive(Debug, Deserialize)]
struct CfApiError {
    #[serde(default)]
    pub message: String,
}

// ---------------------------------------------------------------------------
// CloudflareClient
// ---------------------------------------------------------------------------

/// Cloudflare client for DNS, tunnels, rules, and domain management.
///
/// Authenticates via an API token passed at construction time.
pub struct CloudflareClient {
    http: HttpClient,
    api_token: String,
    zone_id: Option<String>,
}

impl CloudflareClient {
    /// Create a new `CloudflareClient`.
    ///
    /// * `http` – shared HTTP client
    /// * `api_token` – Cloudflare API token
    /// * `zone_id` – optional default zone ID
    pub fn new(http: HttpClient, api_token: String, zone_id: Option<String>) -> Self {
        Self {
            http,
            api_token,
            zone_id,
        }
    }

    /// Set the default zone ID.
    pub fn set_zone_id(&mut self, zone_id: String) {
        self.zone_id = Some(zone_id);
    }

    /// Get the zone ID, returning an error if not set.
    fn require_zone_id(&self) -> Result<&str> {
        self.zone_id.as_deref().ok_or_else(|| {
            BosuaError::Cloud {
                service: "cloudflare".into(),
                message: "Zone ID not configured".into(),
            }
        })
    }

    /// Helper to make an authenticated GET request.
    async fn cf_get<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let client = self.http.get_client().await;
        let resp = client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "cloudflare".into(),
                message: format!("API request failed ({status}): {body}"),
            });
        }

        let api_resp: CfApiResponse<T> = resp.json().await.map_err(BosuaError::Http)?;
        if !api_resp.success {
            let msgs: Vec<String> = api_resp.errors.iter().map(|e| e.message.clone()).collect();
            return Err(BosuaError::Cloud {
                service: "cloudflare".into(),
                message: msgs.join("; "),
            });
        }

        api_resp.result.ok_or_else(|| BosuaError::Cloud {
            service: "cloudflare".into(),
            message: "Empty result from API".into(),
        })
    }

    // -----------------------------------------------------------------------
    // DNS record management
    // -----------------------------------------------------------------------

    /// List DNS records for the configured zone.
    pub async fn list_dns_records(&self) -> Result<Vec<DnsRecord>> {
        let zone_id = self.require_zone_id()?;
        let url = format!("{CF_API_BASE}/zones/{zone_id}/dns_records");
        self.cf_get::<Vec<DnsRecord>>(&url).await
    }

    /// Create a DNS record.
    pub async fn create_dns_record(
        &self,
        record_type: &str,
        name: &str,
        content: &str,
        ttl: u32,
        proxied: bool,
    ) -> Result<DnsRecord> {
        let zone_id = self.require_zone_id()?;
        let url = format!("{CF_API_BASE}/zones/{zone_id}/dns_records");
        let client = self.http.get_client().await;

        let body = serde_json::json!({
            "type": record_type,
            "name": name,
            "content": content,
            "ttl": ttl,
            "proxied": proxied,
        });

        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&body)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "cloudflare".into(),
                message: format!("create_dns_record failed ({status}): {text}"),
            });
        }

        let api_resp: CfApiResponse<DnsRecord> = resp.json().await.map_err(BosuaError::Http)?;
        api_resp.result.ok_or_else(|| BosuaError::Cloud {
            service: "cloudflare".into(),
            message: "No record returned".into(),
        })
    }

    /// Delete a DNS record by ID.
    pub async fn delete_dns_record(&self, record_id: &str) -> Result<()> {
        let zone_id = self.require_zone_id()?;
        let url = format!("{CF_API_BASE}/zones/{zone_id}/dns_records/{record_id}");
        let client = self.http.get_client().await;

        let resp = client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "cloudflare".into(),
                message: format!("delete_dns_record failed ({status}): {body}"),
            });
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Tunnel management
    // -----------------------------------------------------------------------

    /// List Cloudflare Tunnels for the account.
    pub async fn list_tunnels(&self, account_id: &str) -> Result<Vec<CfTunnel>> {
        let url = format!("{CF_API_BASE}/accounts/{account_id}/cfd_tunnel");
        self.cf_get::<Vec<CfTunnel>>(&url).await
    }

    /// Create a Cloudflare Tunnel.
    pub async fn create_tunnel(&self, account_id: &str, name: &str) -> Result<CfTunnel> {
        let url = format!("{CF_API_BASE}/accounts/{account_id}/cfd_tunnel");
        let client = self.http.get_client().await;

        let body = serde_json::json!({ "name": name, "config_src": "cloudflare" });

        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&body)
            .send()
            .await
            .map_err(BosuaError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(BosuaError::Cloud {
                service: "cloudflare".into(),
                message: format!("create_tunnel failed ({status}): {text}"),
            });
        }

        let api_resp: CfApiResponse<CfTunnel> = resp.json().await.map_err(BosuaError::Http)?;
        api_resp.result.ok_or_else(|| BosuaError::Cloud {
            service: "cloudflare".into(),
            message: "No tunnel returned".into(),
        })
    }

    // -----------------------------------------------------------------------
    // Page rules and rulesets
    // -----------------------------------------------------------------------

    /// List page rules for the configured zone.
    pub async fn list_page_rules(&self) -> Result<Vec<PageRule>> {
        let zone_id = self.require_zone_id()?;
        let url = format!("{CF_API_BASE}/zones/{zone_id}/pagerules");
        self.cf_get::<Vec<PageRule>>(&url).await
    }

    /// List rulesets for the configured zone.
    pub async fn list_rulesets(&self) -> Result<Vec<Ruleset>> {
        let zone_id = self.require_zone_id()?;
        let url = format!("{CF_API_BASE}/zones/{zone_id}/rulesets");
        self.cf_get::<Vec<Ruleset>>(&url).await
    }

    // -----------------------------------------------------------------------
    // Domain listing
    // -----------------------------------------------------------------------

    /// List zones (domains) in the account.
    pub async fn list_zones(&self) -> Result<Vec<CfZone>> {
        let url = format!("{CF_API_BASE}/zones");
        self.cf_get::<Vec<CfZone>>(&url).await
    }

    // -----------------------------------------------------------------------
    // Configuration validation
    // -----------------------------------------------------------------------

    /// Validate the current Cloudflare configuration.
    ///
    /// Checks that the API token is valid and the zone is accessible.
    pub async fn validate(&self) -> Result<ValidationResult> {
        let client = self.http.get_client().await;
        let url = format!("{CF_API_BASE}/user/tokens/verify");

        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await
            .map_err(BosuaError::Http)?;

        let mut result = ValidationResult {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        if !resp.status().is_success() {
            result.valid = false;
            result.errors.push("API token verification failed".into());
            return Ok(result);
        }

        // If a zone is configured, verify it's accessible
        if let Some(zone_id) = &self.zone_id {
            let zone_url = format!("{CF_API_BASE}/zones/{zone_id}");
            let zone_resp = client
                .get(&zone_url)
                .header("Authorization", format!("Bearer {}", self.api_token))
                .send()
                .await
                .map_err(BosuaError::Http)?;

            if !zone_resp.status().is_success() {
                result.valid = false;
                result
                    .errors
                    .push(format!("Zone {zone_id} is not accessible"));
            }
        } else {
            result.warnings.push("No zone ID configured".into());
        }

        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// CloudClient trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl CloudClient for CloudflareClient {
    fn name(&self) -> &str {
        "Cloudflare"
    }

    async fn authenticate(&self) -> Result<()> {
        let result = self.validate().await?;
        if result.valid {
            Ok(())
        } else {
            Err(BosuaError::Auth(
                result.errors.join("; "),
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

    #[test]
    fn test_cloudflare_client_name() {
        let http = HttpClient::from_defaults().unwrap();
        let client = CloudflareClient::new(http, "test-token".into(), None);
        assert_eq!(client.name(), "Cloudflare");
    }

    #[test]
    fn test_require_zone_id_without_zone() {
        let http = HttpClient::from_defaults().unwrap();
        let client = CloudflareClient::new(http, "test-token".into(), None);
        assert!(client.require_zone_id().is_err());
    }

    #[test]
    fn test_require_zone_id_with_zone() {
        let http = HttpClient::from_defaults().unwrap();
        let client = CloudflareClient::new(http, "test-token".into(), Some("zone123".into()));
        assert_eq!(client.require_zone_id().unwrap(), "zone123");
    }

    #[test]
    fn test_set_zone_id() {
        let http = HttpClient::from_defaults().unwrap();
        let mut client = CloudflareClient::new(http, "test-token".into(), None);
        assert!(client.require_zone_id().is_err());
        client.set_zone_id("zone456".into());
        assert_eq!(client.require_zone_id().unwrap(), "zone456");
    }

    #[test]
    fn test_dns_record_serialization() {
        let record = DnsRecord {
            id: "rec123".into(),
            record_type: "A".into(),
            name: "example.com".into(),
            content: "1.2.3.4".into(),
            ttl: 300,
            proxied: true,
        };
        let json = serde_json::to_string(&record).unwrap();
        let deser: DnsRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.record_type, "A");
        assert!(deser.proxied);
    }

    #[test]
    fn test_tunnel_serialization() {
        let tunnel = CfTunnel {
            id: "tun123".into(),
            name: "my-tunnel".into(),
            status: "active".into(),
            created_at: Some("2024-01-01T00:00:00Z".into()),
        };
        let json = serde_json::to_string(&tunnel).unwrap();
        let deser: CfTunnel = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.name, "my-tunnel");
    }

    #[test]
    fn test_validation_result_serialization() {
        let result = ValidationResult {
            valid: false,
            errors: vec!["token invalid".into()],
            warnings: vec!["no zone".into()],
        };
        let json = serde_json::to_string(&result).unwrap();
        let deser: ValidationResult = serde_json::from_str(&json).unwrap();
        assert!(!deser.valid);
        assert_eq!(deser.errors.len(), 1);
    }
}
