use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DynamicConfig {
    #[serde(rename = "mediumPremiumDomain")]
    pub medium_premium_domain: String,
    #[serde(rename = "maxRetries")]
    pub max_retries: u32,
    #[serde(rename = "retryDelay")]
    pub retry_delay: u32,
    pub timeout: u32,
    #[serde(rename = "shortTimeout")]
    pub short_timeout: u32,
    #[serde(rename = "waitDuration")]
    pub wait_duration: u32,
    #[serde(rename = "tLSHandshakeTimeout")]
    pub tls_handshake_timeout: u32,
    #[serde(rename = "expectContinueTimeout")]
    pub expect_continue_timeout: u32,
    #[serde(rename = "idleConnTimeout")]
    pub idle_conn_timeout: u32,
    #[serde(rename = "readHeaderTimeout")]
    pub read_header_timeout: u32,
    #[serde(rename = "idleTimeout")]
    pub idle_timeout: u32,
    #[serde(rename = "largeBufferSize")]
    pub large_buffer_size: usize,
    #[serde(rename = "optimalBufferSize")]
    pub optimal_buffer_size: usize,
    #[serde(rename = "chunkSize")]
    pub chunk_size: usize,
    #[serde(rename = "maxIdleConns")]
    pub max_idle_conns: u32,
    #[serde(rename = "maxIdleConnsPerHost")]
    pub max_idle_conns_per_host: u32,
    #[serde(rename = "kodiUsername")]
    pub kodi_username: String,
    #[serde(rename = "kodiPassword")]
    pub kodi_password: String,
    #[serde(rename = "awsRegion")]
    pub aws_region: String,
    #[serde(rename = "gcloudRegion")]
    pub gcloud_region: String,
    #[serde(rename = "gdriveDefaultAccount")]
    pub gdrive_default_account: String,
    #[serde(rename = "backendIp")]
    pub backend_ip: String,
    #[serde(rename = "backendDomain")]
    pub backend_domain: String,
    #[serde(rename = "gcpIp")]
    pub gcp_ip: String,
    #[serde(rename = "gcpDomain")]
    pub gcp_domain: String,
}

impl Default for DynamicConfig {
    fn default() -> Self {
        Self {
            medium_premium_domain: "freedium.cfd".into(),
            max_retries: 5,
            retry_delay: 2,
            timeout: 30,
            short_timeout: 5,
            wait_duration: 3,
            tls_handshake_timeout: 10,
            expect_continue_timeout: 1,
            idle_conn_timeout: 90,
            read_header_timeout: 5,
            idle_timeout: 120,
            large_buffer_size: 256 * 1024,
            optimal_buffer_size: 2048 * 1024,
            chunk_size: 8192,
            max_idle_conns: 200,
            max_idle_conns_per_host: 100,
            kodi_username: "kodi".into(),
            kodi_password: "conchimnon".into(),
            aws_region: "ap-east-1".into(),
            gcloud_region: "asia-east2-c".into(),
            gdrive_default_account: String::new(),
            backend_ip: String::new(),
            backend_domain: String::new(),
            gcp_ip: String::new(),
            gcp_domain: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let config = DynamicConfig::default();
        assert_eq!(config.medium_premium_domain, "freedium.cfd");
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.retry_delay, 2);
        assert_eq!(config.timeout, 30);
        assert_eq!(config.short_timeout, 5);
        assert_eq!(config.wait_duration, 3);
        assert_eq!(config.tls_handshake_timeout, 10);
        assert_eq!(config.expect_continue_timeout, 1);
        assert_eq!(config.idle_conn_timeout, 90);
        assert_eq!(config.read_header_timeout, 5);
        assert_eq!(config.idle_timeout, 120);
        assert_eq!(config.large_buffer_size, 256 * 1024);
        assert_eq!(config.optimal_buffer_size, 2048 * 1024);
        assert_eq!(config.chunk_size, 8192);
        assert_eq!(config.max_idle_conns, 200);
        assert_eq!(config.max_idle_conns_per_host, 100);
        assert_eq!(config.kodi_username, "kodi");
        assert_eq!(config.kodi_password, "conchimnon");
        assert_eq!(config.aws_region, "ap-east-1");
        assert_eq!(config.gcloud_region, "asia-east2-c");
        assert_eq!(config.gdrive_default_account, "");
        assert_eq!(config.backend_ip, "");
        assert_eq!(config.backend_domain, "");
        assert_eq!(config.gcp_ip, "");
        assert_eq!(config.gcp_domain, "");
    }

    #[test]
    fn test_json_round_trip() {
        let config = DynamicConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: DynamicConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_serde_rename_camel_case() {
        let config = DynamicConfig::default();
        let value: serde_json::Value = serde_json::to_value(&config).unwrap();
        let obj = value.as_object().unwrap();

        // Verify Go-compatible camelCase field names
        assert!(obj.contains_key("mediumPremiumDomain"));
        assert!(obj.contains_key("maxRetries"));
        assert!(obj.contains_key("retryDelay"));
        assert!(obj.contains_key("timeout"));
        assert!(obj.contains_key("shortTimeout"));
        assert!(obj.contains_key("waitDuration"));
        assert!(obj.contains_key("tLSHandshakeTimeout"));
        assert!(obj.contains_key("expectContinueTimeout"));
        assert!(obj.contains_key("idleConnTimeout"));
        assert!(obj.contains_key("readHeaderTimeout"));
        assert!(obj.contains_key("idleTimeout"));
        assert!(obj.contains_key("largeBufferSize"));
        assert!(obj.contains_key("optimalBufferSize"));
        assert!(obj.contains_key("chunkSize"));
        assert!(obj.contains_key("maxIdleConns"));
        assert!(obj.contains_key("maxIdleConnsPerHost"));
        assert!(obj.contains_key("kodiUsername"));
        assert!(obj.contains_key("kodiPassword"));
        assert!(obj.contains_key("awsRegion"));
        assert!(obj.contains_key("gcloudRegion"));
        assert!(obj.contains_key("gdriveDefaultAccount"));
        assert!(obj.contains_key("backendIp"));
        assert!(obj.contains_key("backendDomain"));
        assert!(obj.contains_key("gcpIp"));
        assert!(obj.contains_key("gcpDomain"));
    }
}
