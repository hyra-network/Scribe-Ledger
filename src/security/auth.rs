//! Authentication and authorization module
//!
//! This module provides authentication mechanisms and role-based access control (RBAC).

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Permission levels for operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    /// Read permission (GET operations)
    Read,
    /// Write permission (PUT operations)
    Write,
    /// Delete permission (DELETE operations)
    Delete,
    /// Admin permission (cluster management, metrics)
    Admin,
}

/// User role with associated permissions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    /// Role name
    pub name: String,
    /// Set of permissions
    pub permissions: HashSet<Permission>,
}

impl Role {
    /// Create a new role
    pub fn new(name: impl Into<String>, permissions: HashSet<Permission>) -> Self {
        Self {
            name: name.into(),
            permissions,
        }
    }

    /// Create a read-only role
    pub fn read_only() -> Self {
        let mut permissions = HashSet::new();
        permissions.insert(Permission::Read);
        Self::new("read_only", permissions)
    }

    /// Create a read-write role
    pub fn read_write() -> Self {
        let mut permissions = HashSet::new();
        permissions.insert(Permission::Read);
        permissions.insert(Permission::Write);
        Self::new("read_write", permissions)
    }

    /// Create an admin role with all permissions
    pub fn admin() -> Self {
        let mut permissions = HashSet::new();
        permissions.insert(Permission::Read);
        permissions.insert(Permission::Write);
        permissions.insert(Permission::Delete);
        permissions.insert(Permission::Admin);
        Self::new("admin", permissions)
    }

    /// Check if role has a specific permission
    pub fn has_permission(&self, permission: Permission) -> bool {
        self.permissions.contains(&permission)
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    /// Enable authentication
    #[serde(default)]
    pub enabled: bool,
    /// API keys and their associated roles
    #[serde(skip)]
    pub api_keys: HashMap<String, Role>,
}

impl AuthConfig {
    /// Create a new authentication configuration
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            api_keys: HashMap::new(),
        }
    }

    /// Add an API key with a role
    pub fn add_api_key(&mut self, api_key: String, role: Role) {
        self.api_keys.insert(api_key, role);
    }

    /// Get role for an API key
    pub fn get_role(&self, api_key: &str) -> Option<&Role> {
        self.api_keys.get(api_key)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.api_keys.is_empty() {
            return Err(
                "At least one API key must be configured when authentication is enabled"
                    .to_string(),
            );
        }
        Ok(())
    }
}

/// Authentication middleware state
#[derive(Clone)]
pub struct AuthMiddleware {
    config: Arc<RwLock<AuthConfig>>,
}

impl AuthMiddleware {
    /// Create a new authentication middleware
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Get the authentication configuration
    pub async fn config(&self) -> AuthConfig {
        self.config.read().await.clone()
    }

    /// Extract API key from request headers
    fn extract_api_key(headers: &HeaderMap) -> Option<String> {
        // Support both Authorization: Bearer <token> and X-API-Key: <key>
        if let Some(auth_header) = headers.get("authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if let Some(token) = auth_str.strip_prefix("Bearer ") {
                    return Some(token.to_string());
                }
            }
        }

        if let Some(api_key_header) = headers.get("x-api-key") {
            if let Ok(api_key) = api_key_header.to_str() {
                return Some(api_key.to_string());
            }
        }

        None
    }

    /// Determine required permission for a request
    pub fn required_permission(method: &str, path: &str) -> Permission {
        // Admin endpoints
        if path.starts_with("/cluster/") || path.starts_with("/metrics") {
            return Permission::Admin;
        }

        // Data operation endpoints
        match method {
            "GET" => Permission::Read,
            "PUT" => Permission::Write,
            "DELETE" => Permission::Delete,
            _ => Permission::Admin, // Default to admin for unknown methods
        }
    }

    /// Authenticate and authorize a request
    pub async fn authenticate(
        &self,
        headers: &HeaderMap,
        method: &str,
        path: &str,
    ) -> Result<(), Response> {
        let config = self.config.read().await;

        // If authentication is disabled, allow all requests
        if !config.enabled {
            return Ok(());
        }

        // Extract API key
        let api_key = Self::extract_api_key(headers);
        if api_key.is_none() {
            warn!("Authentication failed: No API key provided");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Authentication required. Provide API key via Authorization: Bearer <token> or X-API-Key: <key>"
                })),
            )
                .into_response());
        }

        let api_key = api_key.unwrap();

        // Validate API key
        let role = config.get_role(&api_key);
        if role.is_none() {
            warn!("Authentication failed: Invalid API key");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Invalid API key"
                })),
            )
                .into_response());
        }

        let role = role.unwrap();
        let required_perm = Self::required_permission(method, path);

        // Check if role has required permission
        if !role.has_permission(required_perm) {
            warn!(
                "Authorization failed: Role '{}' lacks {:?} permission for {} {}",
                role.name, required_perm, method, path
            );
            return Err((
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "error": format!("Insufficient permissions. Required: {:?}", required_perm)
                })),
            )
                .into_response());
        }

        debug!(
            "Authentication successful: Role '{}' granted access to {} {}",
            role.name, method, path
        );
        Ok(())
    }
}

/// Axum middleware function for authentication
pub async fn auth_middleware(
    _headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Skip authentication for health endpoint
    if request.uri().path() == "/health" {
        return Ok(next.run(request).await);
    }

    // Extract auth middleware from request extensions if available
    // For now, skip authentication in middleware (will be applied in handlers if needed)
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_enum() {
        assert_eq!(Permission::Read, Permission::Read);
        assert_ne!(Permission::Read, Permission::Write);
    }

    #[test]
    fn test_role_read_only() {
        let role = Role::read_only();
        assert_eq!(role.name, "read_only");
        assert!(role.has_permission(Permission::Read));
        assert!(!role.has_permission(Permission::Write));
        assert!(!role.has_permission(Permission::Delete));
        assert!(!role.has_permission(Permission::Admin));
    }

    #[test]
    fn test_role_read_write() {
        let role = Role::read_write();
        assert_eq!(role.name, "read_write");
        assert!(role.has_permission(Permission::Read));
        assert!(role.has_permission(Permission::Write));
        assert!(!role.has_permission(Permission::Delete));
        assert!(!role.has_permission(Permission::Admin));
    }

    #[test]
    fn test_role_admin() {
        let role = Role::admin();
        assert_eq!(role.name, "admin");
        assert!(role.has_permission(Permission::Read));
        assert!(role.has_permission(Permission::Write));
        assert!(role.has_permission(Permission::Delete));
        assert!(role.has_permission(Permission::Admin));
    }

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert!(!config.enabled);
        assert!(config.api_keys.is_empty());
    }

    #[test]
    fn test_auth_config_add_api_key() {
        let mut config = AuthConfig::new(true);
        config.add_api_key("test-key".to_string(), Role::admin());
        assert_eq!(config.api_keys.len(), 1);
        assert!(config.get_role("test-key").is_some());
        assert_eq!(config.get_role("test-key").unwrap().name, "admin");
    }

    #[test]
    fn test_auth_config_validate_disabled() {
        let config = AuthConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_auth_config_validate_enabled_no_keys() {
        let config = AuthConfig::new(true);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_auth_config_validate_enabled_with_keys() {
        let mut config = AuthConfig::new(true);
        config.add_api_key("test-key".to_string(), Role::admin());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_auth_middleware_extract_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer test-token".parse().unwrap());
        let api_key = AuthMiddleware::extract_api_key(&headers);
        assert_eq!(api_key, Some("test-token".to_string()));
    }

    #[test]
    fn test_auth_middleware_extract_api_key() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "test-key".parse().unwrap());
        let api_key = AuthMiddleware::extract_api_key(&headers);
        assert_eq!(api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_auth_middleware_required_permission() {
        assert_eq!(
            AuthMiddleware::required_permission("GET", "/test"),
            Permission::Read
        );
        assert_eq!(
            AuthMiddleware::required_permission("PUT", "/test"),
            Permission::Write
        );
        assert_eq!(
            AuthMiddleware::required_permission("DELETE", "/test"),
            Permission::Delete
        );
        assert_eq!(
            AuthMiddleware::required_permission("GET", "/metrics"),
            Permission::Admin
        );
        assert_eq!(
            AuthMiddleware::required_permission("GET", "/cluster/info"),
            Permission::Admin
        );
    }

    #[tokio::test]
    async fn test_auth_middleware_disabled() {
        let config = AuthConfig::default();
        let middleware = AuthMiddleware::new(config);
        let headers = HeaderMap::new();
        let result = middleware.authenticate(&headers, "GET", "/test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_auth_middleware_missing_key() {
        let config = AuthConfig::new(true);
        let middleware = AuthMiddleware::new(config);
        let headers = HeaderMap::new();
        let result = middleware.authenticate(&headers, "GET", "/test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_auth_middleware_invalid_key() {
        let mut config = AuthConfig::new(true);
        config.add_api_key("valid-key".to_string(), Role::admin());
        let middleware = AuthMiddleware::new(config);
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "invalid-key".parse().unwrap());
        let result = middleware.authenticate(&headers, "GET", "/test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_auth_middleware_valid_key_sufficient_permission() {
        let mut config = AuthConfig::new(true);
        config.add_api_key("admin-key".to_string(), Role::admin());
        let middleware = AuthMiddleware::new(config);
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "admin-key".parse().unwrap());
        let result = middleware.authenticate(&headers, "GET", "/test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_auth_middleware_valid_key_insufficient_permission() {
        let mut config = AuthConfig::new(true);
        config.add_api_key("read-key".to_string(), Role::read_only());
        let middleware = AuthMiddleware::new(config);
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "read-key".parse().unwrap());
        let result = middleware.authenticate(&headers, "PUT", "/test").await;
        assert!(result.is_err());
    }
}
