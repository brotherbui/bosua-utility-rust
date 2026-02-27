//! TLS setup using rustls for the HTTP server.
//!
//! Loads certificate and key files, configures TLS 1.2+ minimum protocol
//! version, and sets ALPN protocols (h2, http/1.1).

use crate::errors::{BosuaError, Result};
use crate::server::config::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::fs;
use std::io::BufReader;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;

/// Build a `TlsAcceptor` from the server configuration.
///
/// # Errors
///
/// Returns an error if:
/// - `cert_file` or `key_file` is `None` when TLS is enabled
/// - The certificate or key file cannot be read
/// - The certificate or key data is invalid
pub fn setup_tls(config: &ServerConfig) -> Result<TlsAcceptor> {
    let cert_path = config
        .cert_file
        .as_ref()
        .ok_or_else(|| BosuaError::Config("TLS enabled but cert_file is not set".into()))?;

    let key_path = config
        .key_file
        .as_ref()
        .ok_or_else(|| BosuaError::Config("TLS enabled but key_file is not set".into()))?;

    // Load certificates
    let cert_data = fs::read(cert_path).map_err(|e| {
        BosuaError::Config(format!("Failed to read cert file {}: {}", cert_path.display(), e))
    })?;
    let certs = load_certs(&cert_data)?;

    // Load private key
    let key_data = fs::read(key_path).map_err(|e| {
        BosuaError::Config(format!("Failed to read key file {}: {}", key_path.display(), e))
    })?;
    let key = load_private_key(&key_data)?;

    // Build rustls ServerConfig with TLS 1.2+ minimum
    let mut tls_config = rustls::ServerConfig::builder_with_protocol_versions(&[
        &rustls::version::TLS12,
        &rustls::version::TLS13,
    ])
    .with_no_client_auth()
    .with_single_cert(certs, key)
    .map_err(|e| BosuaError::Config(format!("TLS configuration error: {}", e)))?;

    // ALPN negotiation: prefer h2, fall back to http/1.1
    tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(TlsAcceptor::from(Arc::new(tls_config)))
}

/// Parse PEM-encoded certificates from raw bytes.
fn load_certs(data: &[u8]) -> Result<Vec<CertificateDer<'static>>> {
    let mut reader = BufReader::new(data);
    let certs: Vec<CertificateDer<'static>> =
        rustls_pemfile::certs(&mut reader)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| BosuaError::Config(format!("Failed to parse certificates: {}", e)))?;

    if certs.is_empty() {
        return Err(BosuaError::Config(
            "No certificates found in cert file".into(),
        ));
    }
    Ok(certs)
}

/// Parse a PEM-encoded private key from raw bytes.
///
/// Tries PKCS#8 first, then RSA, then EC key formats.
fn load_private_key(data: &[u8]) -> Result<PrivateKeyDer<'static>> {
    let mut reader = BufReader::new(data);

    loop {
        match rustls_pemfile::read_one(&mut reader) {
            Ok(Some(rustls_pemfile::Item::Pkcs8Key(key))) => {
                return Ok(PrivateKeyDer::Pkcs8(key));
            }
            Ok(Some(rustls_pemfile::Item::Pkcs1Key(key))) => {
                return Ok(PrivateKeyDer::Pkcs1(key));
            }
            Ok(Some(rustls_pemfile::Item::Sec1Key(key))) => {
                return Ok(PrivateKeyDer::Sec1(key));
            }
            Ok(Some(_)) => {
                // Skip other PEM items (e.g. certificates mixed in)
                continue;
            }
            Ok(None) => break,
            Err(e) => {
                return Err(BosuaError::Config(format!(
                    "Failed to parse private key: {}",
                    e
                )));
            }
        }
    }

    Err(BosuaError::Config(
        "No private key found in key file".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_tls_fails_without_cert_file() {
        let config = ServerConfig {
            tls: true,
            cert_file: None,
            key_file: Some("/tmp/key.pem".into()),
            ..Default::default()
        };
        let result = setup_tls(&config);
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("cert_file"));
    }

    #[test]
    fn setup_tls_fails_without_key_file() {
        let config = ServerConfig {
            tls: true,
            cert_file: Some("/tmp/cert.pem".into()),
            key_file: None,
            ..Default::default()
        };
        let result = setup_tls(&config);
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("key_file"));
    }

    #[test]
    fn setup_tls_fails_with_nonexistent_cert() {
        let config = ServerConfig {
            tls: true,
            cert_file: Some("/nonexistent/cert.pem".into()),
            key_file: Some("/nonexistent/key.pem".into()),
            ..Default::default()
        };
        let result = setup_tls(&config);
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("Failed to read cert file"));
    }

    #[test]
    fn load_certs_fails_on_empty_data() {
        let err = load_certs(b"").unwrap_err();
        assert!(err.to_string().contains("No certificates found"));
    }

    #[test]
    fn load_private_key_fails_on_empty_data() {
        let err = load_private_key(b"").unwrap_err();
        assert!(err.to_string().contains("No private key found"));
    }
}
