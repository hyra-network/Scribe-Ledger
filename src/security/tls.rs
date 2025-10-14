//! TLS configuration and support for secure communication
//!
//! This module provides TLS encryption for node-to-node communication and HTTPS API endpoints.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// TLS configuration for client and server
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsConfig {
    /// Enable TLS
    #[serde(default)]
    pub enabled: bool,
    /// Path to TLS certificate file (PEM format)
    pub cert_path: Option<PathBuf>,
    /// Path to TLS private key file (PEM format)
    pub key_path: Option<PathBuf>,
    /// Path to CA certificate for client verification (optional)
    pub ca_cert_path: Option<PathBuf>,
    /// Require client certificates (mutual TLS)
    #[serde(default)]
    pub require_client_cert: bool,
}

impl TlsConfig {
    /// Create a new TLS configuration with certificate and key
    pub fn new(cert_path: PathBuf, key_path: PathBuf) -> Self {
        Self {
            enabled: true,
            cert_path: Some(cert_path),
            key_path: Some(key_path),
            ca_cert_path: None,
            require_client_cert: false,
        }
    }

    /// Enable mutual TLS with CA certificate
    pub fn with_mutual_tls(mut self, ca_cert_path: PathBuf) -> Self {
        self.ca_cert_path = Some(ca_cert_path);
        self.require_client_cert = true;
        self
    }

    /// Validate TLS configuration
    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        if self.cert_path.is_none() {
            return Err("TLS certificate path is required when TLS is enabled".to_string());
        }

        if self.key_path.is_none() {
            return Err("TLS key path is required when TLS is enabled".to_string());
        }

        if self.require_client_cert && self.ca_cert_path.is_none() {
            return Err(
                "CA certificate path is required when client certificates are required".to_string(),
            );
        }

        Ok(())
    }
}

/// TLS server configuration wrapper
#[derive(Debug, Clone)]
pub struct TlsServerConfig {
    config: TlsConfig,
}

impl TlsServerConfig {
    /// Create a new TLS server configuration
    pub fn new(config: TlsConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Get the TLS configuration
    pub fn config(&self) -> &TlsConfig {
        &self.config
    }

    /// Check if TLS is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_default() {
        let config = TlsConfig::default();
        assert!(!config.enabled);
        assert!(config.cert_path.is_none());
        assert!(config.key_path.is_none());
        assert!(config.ca_cert_path.is_none());
        assert!(!config.require_client_cert);
    }

    #[test]
    fn test_tls_config_new() {
        let config = TlsConfig::new(PathBuf::from("/cert.pem"), PathBuf::from("/key.pem"));
        assert!(config.enabled);
        assert_eq!(config.cert_path, Some(PathBuf::from("/cert.pem")));
        assert_eq!(config.key_path, Some(PathBuf::from("/key.pem")));
        assert!(config.ca_cert_path.is_none());
        assert!(!config.require_client_cert);
    }

    #[test]
    fn test_tls_config_with_mutual_tls() {
        let config = TlsConfig::new(PathBuf::from("/cert.pem"), PathBuf::from("/key.pem"))
            .with_mutual_tls(PathBuf::from("/ca.pem"));
        assert!(config.enabled);
        assert_eq!(config.ca_cert_path, Some(PathBuf::from("/ca.pem")));
        assert!(config.require_client_cert);
    }

    #[test]
    fn test_tls_config_validate_disabled() {
        let config = TlsConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_tls_config_validate_missing_cert() {
        let mut config = TlsConfig::default();
        config.enabled = true;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_tls_config_validate_missing_key() {
        let mut config = TlsConfig::default();
        config.enabled = true;
        config.cert_path = Some(PathBuf::from("/cert.pem"));
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_tls_config_validate_valid() {
        let config = TlsConfig::new(PathBuf::from("/cert.pem"), PathBuf::from("/key.pem"));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_tls_server_config_new() {
        let tls_config = TlsConfig::new(PathBuf::from("/cert.pem"), PathBuf::from("/key.pem"));
        let server_config = TlsServerConfig::new(tls_config.clone());
        assert!(server_config.is_ok());
        let server_config = server_config.unwrap();
        assert!(server_config.is_enabled());
    }

    #[test]
    fn test_tls_server_config_invalid() {
        let mut tls_config = TlsConfig::default();
        tls_config.enabled = true;
        let server_config = TlsServerConfig::new(tls_config);
        assert!(server_config.is_err());
    }

    #[test]
    fn test_tls_config_validate_mutual_tls_missing_ca() {
        let mut config = TlsConfig::new(PathBuf::from("/cert.pem"), PathBuf::from("/key.pem"));
        config.require_client_cert = true;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_tls_config_validate_mutual_tls_valid() {
        let config = TlsConfig::new(PathBuf::from("/cert.pem"), PathBuf::from("/key.pem"))
            .with_mutual_tls(PathBuf::from("/ca.pem"));
        assert!(config.validate().is_ok());
    }
}
