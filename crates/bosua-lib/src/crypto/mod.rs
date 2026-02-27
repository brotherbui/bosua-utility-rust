//! Cryptographic utilities: hash functions and TLS helpers.
//!
//! SHA-256 is provided via the `ring` crate; MD5 via the `md-5` crate.
//! TLS configuration helpers use `rustls`.

use ring::digest;
use rustls::ClientConfig;
use std::sync::Arc;

/// Compute SHA-256 hash of input bytes, returning a lowercase hex string.
pub fn sha256(data: &[u8]) -> String {
    let d = digest::digest(&digest::SHA256, data);
    hex_encode(d.as_ref())
}

/// Compute MD5 hash of input bytes, returning a lowercase hex string.
///
/// MD5 is not available in `ring`; we use the `md-5` crate instead.
/// **Warning:** MD5 is cryptographically broken — use only for legacy
/// compatibility (e.g. checksum verification against existing Go output).
pub fn md5(data: &[u8]) -> String {
    let d = <md5::Md5 as md5::Digest>::digest(data);
    hex_encode(d.as_ref())
}

/// Encode a byte slice as a lowercase hex string.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Build a default `rustls` `ClientConfig` with system root certificates
/// and safe defaults (TLS 1.2+).
///
/// This is the shared TLS configuration used by the HTTP client and
/// any other component that needs outbound TLS.
pub fn default_tls_client_config() -> ClientConfig {
    let root_store = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };

    ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("TLS protocol version configuration failed")
        .with_root_certificates(root_store)
        .with_no_client_auth()
}

/// Convenience wrapper returning an `Arc<ClientConfig>` ready for use
/// with `tokio-rustls` or `reqwest`.
pub fn default_tls_client_config_arc() -> Arc<ClientConfig> {
    Arc::new(default_tls_client_config())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_empty() {
        // SHA-256 of empty input is a well-known constant
        let hash = sha256(b"");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_sha256_hello() {
        let hash = sha256(b"hello");
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_md5_empty() {
        let hash = md5(b"");
        assert_eq!(hash, "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_md5_hello() {
        let hash = md5(b"hello");
        assert_eq!(hash, "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn test_sha256_deterministic() {
        let data = b"deterministic test input";
        assert_eq!(sha256(data), sha256(data));
    }

    #[test]
    fn test_md5_deterministic() {
        let data = b"deterministic test input";
        assert_eq!(md5(data), md5(data));
    }

    #[test]
    fn test_hex_encode() {
        assert_eq!(hex_encode(&[0x00, 0xff, 0x0a, 0xbc]), "00ff0abc");
    }

    #[test]
    fn test_default_tls_client_config() {
        // Should not panic and should produce a valid config
        let _config = default_tls_client_config();
    }

    #[test]
    fn test_default_tls_client_config_arc() {
        let config = default_tls_client_config_arc();
        // Just verify it wraps correctly
        assert!(Arc::strong_count(&config) == 1);
    }
}
