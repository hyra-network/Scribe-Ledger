//! Rate limiting module for preventing abuse
//!
//! This module implements token bucket rate limiting for API requests.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::warn;

/// Rate limiter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Maximum requests per time window
    pub max_requests: usize,
    /// Time window duration in seconds
    pub window_secs: u64,
    /// Burst capacity (allows short bursts above average rate)
    pub burst_size: usize,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_requests: 100,
            window_secs: 60,
            burst_size: 10,
        }
    }
}

impl RateLimiterConfig {
    /// Create a new rate limiter configuration
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            enabled: true,
            max_requests,
            window_secs,
            burst_size: max_requests / 10, // Default burst is 10% of max
        }
    }

    /// Set burst size
    pub fn with_burst_size(mut self, burst_size: usize) -> Self {
        self.burst_size = burst_size;
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled {
            if self.max_requests == 0 {
                return Err("max_requests must be greater than 0".to_string());
            }
            if self.window_secs == 0 {
                return Err("window_secs must be greater than 0".to_string());
            }
            if self.burst_size > self.max_requests {
                return Err("burst_size cannot exceed max_requests".to_string());
            }
        }
        Ok(())
    }
}

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Available tokens
    tokens: f64,
    /// Maximum tokens (burst capacity)
    capacity: f64,
    /// Token refill rate per second
    refill_rate: f64,
    /// Last refill time
    last_refill: Instant,
}

impl TokenBucket {
    /// Create a new token bucket
    fn new(capacity: usize, refill_rate: f64) -> Self {
        Self {
            tokens: capacity as f64,
            capacity: capacity as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }

    /// Try to consume a token
    fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Get available tokens
    fn available(&mut self) -> usize {
        self.refill();
        self.tokens.floor() as usize
    }
}

/// Rate limiter with per-client tracking
pub struct RateLimiter {
    config: RateLimiterConfig,
    /// Per-client token buckets (key: client ID, typically IP address or API key)
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimiterConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            buckets: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Check if a request is allowed for a client
    pub async fn check_rate_limit(&self, client_id: &str) -> bool {
        if !self.config.enabled {
            return true;
        }

        let mut buckets = self.buckets.write().await;

        // Get or create bucket for client
        let bucket = buckets.entry(client_id.to_string()).or_insert_with(|| {
            let refill_rate = self.config.max_requests as f64 / self.config.window_secs as f64;
            TokenBucket::new(
                self.config.max_requests + self.config.burst_size,
                refill_rate,
            )
        });

        let allowed = bucket.try_consume();
        if !allowed {
            warn!(
                "Rate limit exceeded for client: {} (available: {})",
                client_id,
                bucket.available()
            );
        }
        allowed
    }

    /// Get available tokens for a client
    pub async fn get_available_tokens(&self, client_id: &str) -> Option<usize> {
        if !self.config.enabled {
            return None;
        }

        let mut buckets = self.buckets.write().await;
        buckets.get_mut(client_id).map(|bucket| bucket.available())
    }

    /// Clean up old buckets (call periodically to prevent memory growth)
    pub async fn cleanup_old_buckets(&self) {
        let mut buckets = self.buckets.write().await;
        buckets.retain(|_, bucket| {
            // Keep buckets that have been used recently (within 2x window)
            let elapsed = Instant::now().duration_since(bucket.last_refill).as_secs();
            elapsed < self.config.window_secs * 2
        });
    }

    /// Get configuration
    pub fn config(&self) -> &RateLimiterConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[test]
    fn test_rate_limiter_config_default() {
        let config = RateLimiterConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.max_requests, 100);
        assert_eq!(config.window_secs, 60);
        assert_eq!(config.burst_size, 10);
    }

    #[test]
    fn test_rate_limiter_config_new() {
        let config = RateLimiterConfig::new(50, 30);
        assert!(config.enabled);
        assert_eq!(config.max_requests, 50);
        assert_eq!(config.window_secs, 30);
        assert_eq!(config.burst_size, 5);
    }

    #[test]
    fn test_rate_limiter_config_with_burst_size() {
        let config = RateLimiterConfig::new(100, 60).with_burst_size(20);
        assert_eq!(config.burst_size, 20);
    }

    #[test]
    fn test_rate_limiter_config_validate() {
        let config = RateLimiterConfig::new(100, 60);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_rate_limiter_config_validate_zero_requests() {
        let mut config = RateLimiterConfig::new(100, 60);
        config.max_requests = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_rate_limiter_config_validate_zero_window() {
        let mut config = RateLimiterConfig::new(100, 60);
        config.window_secs = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_rate_limiter_config_validate_burst_too_large() {
        let mut config = RateLimiterConfig::new(100, 60);
        config.burst_size = 200;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_token_bucket_new() {
        let bucket = TokenBucket::new(100, 1.0);
        assert_eq!(bucket.capacity, 100.0);
        assert_eq!(bucket.tokens, 100.0);
        assert_eq!(bucket.refill_rate, 1.0);
    }

    #[test]
    fn test_token_bucket_consume() {
        let mut bucket = TokenBucket::new(10, 1.0);
        assert!(bucket.try_consume());
        assert_eq!(bucket.tokens.floor() as usize, 9);
    }

    #[test]
    fn test_token_bucket_consume_all() {
        let mut bucket = TokenBucket::new(3, 1.0);
        assert!(bucket.try_consume());
        assert!(bucket.try_consume());
        assert!(bucket.try_consume());
        assert!(!bucket.try_consume()); // Should fail when empty
    }

    #[tokio::test]
    async fn test_rate_limiter_disabled() {
        let config = RateLimiterConfig::default();
        let limiter = RateLimiter::new(config).unwrap();
        assert!(limiter.check_rate_limit("client1").await);
        assert!(limiter.check_rate_limit("client1").await);
        assert!(limiter.check_rate_limit("client1").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_enabled() {
        let config = RateLimiterConfig::new(2, 60);
        let limiter = RateLimiter::new(config).unwrap();

        // First two requests should succeed
        assert!(limiter.check_rate_limit("client1").await);
        assert!(limiter.check_rate_limit("client1").await);

        // Next requests should fail (exceeds burst capacity)
        for _ in 0..5 {
            assert!(!limiter.check_rate_limit("client1").await);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_per_client() {
        let config = RateLimiterConfig::new(2, 60);
        let limiter = RateLimiter::new(config).unwrap();

        // Client 1 consumes their quota
        assert!(limiter.check_rate_limit("client1").await);
        assert!(limiter.check_rate_limit("client1").await);
        assert!(!limiter.check_rate_limit("client1").await);

        // Client 2 still has quota
        assert!(limiter.check_rate_limit("client2").await);
        assert!(limiter.check_rate_limit("client2").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_refill() {
        let config = RateLimiterConfig::new(10, 1); // 10 requests per second
        let limiter = RateLimiter::new(config).unwrap();

        // Consume all tokens (max_requests + burst_size = 10 + 1 = 11)
        for _ in 0..11 {
            assert!(limiter.check_rate_limit("client1").await);
        }
        // Now we should be rate limited
        assert!(!limiter.check_rate_limit("client1").await);

        // Wait for refill (1.1 seconds should refill 11 tokens at 10 tokens/sec)
        sleep(Duration::from_millis(1100)).await;

        // Should have refilled tokens
        assert!(limiter.check_rate_limit("client1").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_get_available_tokens() {
        let config = RateLimiterConfig::new(5, 60);
        let limiter = RateLimiter::new(config).unwrap();

        // Consume some tokens
        assert!(limiter.check_rate_limit("client1").await);
        assert!(limiter.check_rate_limit("client1").await);

        // Check available tokens (should be 3 remaining + burst)
        let available = limiter.get_available_tokens("client1").await;
        assert!(available.is_some());
        assert!(available.unwrap() >= 3);
    }

    #[tokio::test]
    async fn test_rate_limiter_cleanup() {
        let config = RateLimiterConfig::new(100, 1);
        let limiter = RateLimiter::new(config).unwrap();

        // Create some buckets
        assert!(limiter.check_rate_limit("client1").await);
        assert!(limiter.check_rate_limit("client2").await);

        // Cleanup should keep recent buckets
        limiter.cleanup_old_buckets().await;

        // Buckets should still exist
        assert!(limiter.check_rate_limit("client1").await);
        assert!(limiter.check_rate_limit("client2").await);
    }
}
