//! Integration tests for security features (Task 11.4)
//!
//! This test suite validates the security hardening features including:
//! - TLS configuration
//! - Authentication and authorization
//! - Rate limiting
//! - Access control
//! - Audit logging

use hyra_scribe_ledger::security::{
    AuthConfig, AuthMiddleware, Permission, RateLimiter, RateLimiterConfig, Role, TlsConfig,
    TlsServerConfig,
};
use std::path::PathBuf;

#[test]
fn test_tls_configuration() {
    // Test default TLS configuration
    let default_config = TlsConfig::default();
    assert!(!default_config.enabled);
    assert!(default_config.validate().is_ok());

    // Test basic TLS configuration
    let tls_config = TlsConfig::new(PathBuf::from("/cert.pem"), PathBuf::from("/key.pem"));
    assert!(tls_config.enabled);
    assert!(tls_config.validate().is_ok());

    // Test mutual TLS configuration
    let mutual_tls = TlsConfig::new(PathBuf::from("/cert.pem"), PathBuf::from("/key.pem"))
        .with_mutual_tls(PathBuf::from("/ca.pem"));
    assert!(mutual_tls.enabled);
    assert!(mutual_tls.require_client_cert);
    assert!(mutual_tls.validate().is_ok());

    // Test TLS server configuration
    let server_config = TlsServerConfig::new(tls_config);
    assert!(server_config.is_ok());
    assert!(server_config.unwrap().is_enabled());
}

#[test]
fn test_authentication_roles() {
    // Test read-only role
    let read_only = Role::read_only();
    assert!(read_only.has_permission(Permission::Read));
    assert!(!read_only.has_permission(Permission::Write));
    assert!(!read_only.has_permission(Permission::Delete));
    assert!(!read_only.has_permission(Permission::Admin));

    // Test read-write role
    let read_write = Role::read_write();
    assert!(read_write.has_permission(Permission::Read));
    assert!(read_write.has_permission(Permission::Write));
    assert!(!read_write.has_permission(Permission::Delete));
    assert!(!read_write.has_permission(Permission::Admin));

    // Test admin role
    let admin = Role::admin();
    assert!(admin.has_permission(Permission::Read));
    assert!(admin.has_permission(Permission::Write));
    assert!(admin.has_permission(Permission::Delete));
    assert!(admin.has_permission(Permission::Admin));
}

#[test]
fn test_authentication_configuration() {
    // Test default configuration
    let mut config = AuthConfig::default();
    assert!(!config.enabled);
    assert!(config.validate().is_ok());

    // Test enabled configuration with API keys
    config.enabled = true;
    config.add_api_key("admin-key".to_string(), Role::admin());
    config.add_api_key("read-key".to_string(), Role::read_only());
    config.add_api_key("write-key".to_string(), Role::read_write());

    assert!(config.validate().is_ok());
    assert_eq!(config.api_keys.len(), 3);
    assert!(config.get_role("admin-key").is_some());
    assert_eq!(config.get_role("admin-key").unwrap().name, "admin");
}

#[tokio::test]
async fn test_authentication_middleware() {
    // Create authentication middleware
    let mut config = AuthConfig::new(true);
    config.add_api_key("test-admin-key".to_string(), Role::admin());
    config.add_api_key("test-read-key".to_string(), Role::read_only());

    let middleware = AuthMiddleware::new(config);

    // Test with admin key
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("x-api-key", "test-admin-key".parse().unwrap());

    let result = middleware.authenticate(&headers, "GET", "/test").await;
    assert!(result.is_ok());

    // Test with read key on write operation (should fail)
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("x-api-key", "test-read-key".parse().unwrap());

    let result = middleware.authenticate(&headers, "PUT", "/test").await;
    assert!(result.is_err());

    // Test with invalid key (should fail)
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("x-api-key", "invalid-key".parse().unwrap());

    let result = middleware.authenticate(&headers, "GET", "/test").await;
    assert!(result.is_err());
}

#[test]
fn test_rate_limiter_configuration() {
    // Test default configuration
    let default_config = RateLimiterConfig::default();
    assert!(!default_config.enabled);
    assert_eq!(default_config.max_requests, 100);
    assert_eq!(default_config.window_secs, 60);

    // Test custom configuration
    let custom_config = RateLimiterConfig::new(50, 30).with_burst_size(10);
    assert!(custom_config.enabled);
    assert_eq!(custom_config.max_requests, 50);
    assert_eq!(custom_config.window_secs, 30);
    assert_eq!(custom_config.burst_size, 10);
    assert!(custom_config.validate().is_ok());
}

#[tokio::test]
async fn test_rate_limiter_enforcement() {
    // Create rate limiter with low limits for testing
    let config = RateLimiterConfig::new(5, 60); // 5 requests per minute, burst of 0 (10% of 5 = 0)
    let limiter = RateLimiter::new(config).unwrap();

    // With capacity = max_requests + burst_size = 5 + 0 = 5
    // First 5 requests should succeed
    for i in 1..=5 {
        assert!(
            limiter.check_rate_limit("test-client").await,
            "Request {} should succeed",
            i
        );
    }

    // Next request should be rate limited
    assert!(
        !limiter.check_rate_limit("test-client").await,
        "Request should be rate limited after capacity exhausted"
    );
}

#[tokio::test]
async fn test_rate_limiter_per_client_isolation() {
    // Create rate limiter
    let config = RateLimiterConfig::new(3, 60);
    let limiter = RateLimiter::new(config).unwrap();

    // Exhaust quota for client1
    for _ in 0..4 {
        limiter.check_rate_limit("client1").await;
    }

    // Client1 should be rate limited
    assert!(!limiter.check_rate_limit("client1").await);

    // Client2 should still have quota
    assert!(limiter.check_rate_limit("client2").await);
    assert!(limiter.check_rate_limit("client2").await);
    assert!(limiter.check_rate_limit("client2").await);
}

#[test]
fn test_access_control_permissions() {
    let mut config = AuthConfig::new(true);

    // Add users with different permissions
    config.add_api_key("admin".to_string(), Role::admin());
    config.add_api_key("writer".to_string(), Role::read_write());
    config.add_api_key("reader".to_string(), Role::read_only());

    // Verify admin has all permissions
    let admin_role = config.get_role("admin").unwrap();
    assert!(admin_role.has_permission(Permission::Read));
    assert!(admin_role.has_permission(Permission::Write));
    assert!(admin_role.has_permission(Permission::Delete));
    assert!(admin_role.has_permission(Permission::Admin));

    // Verify writer has read and write, but not delete or admin
    let writer_role = config.get_role("writer").unwrap();
    assert!(writer_role.has_permission(Permission::Read));
    assert!(writer_role.has_permission(Permission::Write));
    assert!(!writer_role.has_permission(Permission::Delete));
    assert!(!writer_role.has_permission(Permission::Admin));

    // Verify reader only has read permission
    let reader_role = config.get_role("reader").unwrap();
    assert!(reader_role.has_permission(Permission::Read));
    assert!(!reader_role.has_permission(Permission::Write));
    assert!(!reader_role.has_permission(Permission::Delete));
    assert!(!reader_role.has_permission(Permission::Admin));
}

#[test]
fn test_audit_logging_events() {
    use hyra_scribe_ledger::logging::{audit_log, AuditEvent};

    // Test various audit events
    audit_log(
        AuditEvent::AuthSuccess,
        Some("user@example.com"),
        "login",
        Some("/auth"),
        "success",
        Some("User authenticated successfully"),
    );

    audit_log(
        AuditEvent::AuthFailure,
        Some("attacker@example.com"),
        "login",
        Some("/auth"),
        "failure",
        Some("Invalid credentials"),
    );

    audit_log(
        AuditEvent::DataWrite,
        Some("admin@example.com"),
        "put",
        Some("/data/key1"),
        "success",
        Some("Data written successfully"),
    );

    audit_log(
        AuditEvent::RateLimitExceeded,
        Some("abuser@example.com"),
        "request",
        Some("/api/endpoint"),
        "blocked",
        Some("Rate limit exceeded"),
    );

    audit_log(
        AuditEvent::AuthzDenied,
        Some("user@example.com"),
        "delete",
        Some("/data/key1"),
        "denied",
        Some("Insufficient permissions"),
    );

    // All audit log calls should succeed without panic
}

#[tokio::test]
async fn test_combined_security_features() {
    // Test authentication + rate limiting together
    let mut auth_config = AuthConfig::new(true);
    auth_config.add_api_key("test-key".to_string(), Role::admin());
    let auth_middleware = AuthMiddleware::new(auth_config);

    let rate_config = RateLimiterConfig::new(10, 60);
    let rate_limiter = RateLimiter::new(rate_config).unwrap();

    // Test authenticated request with rate limit check
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("x-api-key", "test-key".parse().unwrap());

    // Should pass authentication
    let auth_result = auth_middleware.authenticate(&headers, "GET", "/test").await;
    assert!(auth_result.is_ok());

    // Should pass rate limit
    let rate_result = rate_limiter.check_rate_limit("test-key").await;
    assert!(rate_result);
}

#[test]
fn test_security_configuration_validation() {
    // Test TLS validation
    let mut tls_config = TlsConfig::default();
    tls_config.enabled = true;
    assert!(tls_config.validate().is_err()); // Missing cert and key

    // Test auth validation
    let auth_config = AuthConfig::new(true);
    assert!(auth_config.validate().is_err()); // No API keys

    // Test rate limiter validation
    let mut rate_config = RateLimiterConfig::new(100, 60);
    rate_config.burst_size = 200; // Exceeds max_requests
    assert!(rate_config.validate().is_err());
}

#[tokio::test]
async fn test_rate_limiter_cleanup() {
    let config = RateLimiterConfig::new(100, 1);
    let limiter = RateLimiter::new(config).unwrap();

    // Create some buckets
    for i in 0..5 {
        let client = format!("client{}", i);
        limiter.check_rate_limit(&client).await;
    }

    // Cleanup should work without error
    limiter.cleanup_old_buckets().await;

    // Buckets should still be usable after cleanup
    assert!(limiter.check_rate_limit("client1").await);
}

#[test]
fn test_permission_based_routing() {
    // Test that different operations require different permissions
    use hyra_scribe_ledger::security::AuthMiddleware;

    // GET operations require Read permission
    let get_perm = AuthMiddleware::required_permission("GET", "/data/key1");
    assert_eq!(get_perm, Permission::Read);

    // PUT operations require Write permission
    let put_perm = AuthMiddleware::required_permission("PUT", "/data/key1");
    assert_eq!(put_perm, Permission::Write);

    // DELETE operations require Delete permission
    let delete_perm = AuthMiddleware::required_permission("DELETE", "/data/key1");
    assert_eq!(delete_perm, Permission::Delete);

    // Cluster operations require Admin permission
    let cluster_perm = AuthMiddleware::required_permission("GET", "/cluster/info");
    assert_eq!(cluster_perm, Permission::Admin);

    // Metrics operations require Admin permission
    let metrics_perm = AuthMiddleware::required_permission("GET", "/metrics");
    assert_eq!(metrics_perm, Permission::Admin);
}
