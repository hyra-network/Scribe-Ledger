//! Security module for TLS, authentication, rate limiting, and access control
//!
//! This module provides security features for the Hyra Scribe Ledger including:
//! - TLS encryption for node-to-node communication
//! - API authentication (bearer tokens, API keys)
//! - Request rate limiting
//! - Role-based access control (RBAC)
//! - Audit logging for security events

pub mod auth;
pub mod rate_limit;
pub mod tls;

pub use auth::{AuthConfig, AuthMiddleware, Permission, Role};
pub use rate_limit::{RateLimiter, RateLimiterConfig};
pub use tls::{TlsConfig, TlsServerConfig};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_module_structure() {
        // Verify module structure is properly set up
        let _auth_config = AuthConfig::default();
        let _rate_config = RateLimiterConfig::default();
        let _tls_config = TlsConfig::default();
    }
}
