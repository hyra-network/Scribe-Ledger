/// Prometheus metrics collection for the Scribe Ledger
///
/// This module provides comprehensive metrics tracking for monitoring system performance,
/// including request latency, throughput, storage metrics, and Raft consensus metrics.
use lazy_static::lazy_static;
use prometheus::{Histogram, HistogramOpts, IntCounter, IntGauge, Registry};
use std::sync::Once;

lazy_static! {
    /// Global metrics registry
    pub static ref REGISTRY: Registry = Registry::new();

    // Request counters
    /// Total number of GET requests
    pub static ref GET_REQUESTS: IntCounter = IntCounter::new(
        "scribe_ledger_get_requests_total",
        "Total number of GET requests"
    ).unwrap();

    /// Total number of PUT requests
    pub static ref PUT_REQUESTS: IntCounter = IntCounter::new(
        "scribe_ledger_put_requests_total",
        "Total number of PUT requests"
    ).unwrap();

    /// Total number of DELETE requests
    pub static ref DELETE_REQUESTS: IntCounter = IntCounter::new(
        "scribe_ledger_delete_requests_total",
        "Total number of DELETE requests"
    ).unwrap();

    // Request latency histograms
    /// GET request latency in seconds
    pub static ref GET_LATENCY: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "scribe_ledger_get_latency_seconds",
            "GET request latency in seconds"
        )
        .buckets(vec![0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0])
    ).unwrap();

    /// PUT request latency in seconds
    pub static ref PUT_LATENCY: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "scribe_ledger_put_latency_seconds",
            "PUT request latency in seconds"
        )
        .buckets(vec![0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0])
    ).unwrap();

    /// DELETE request latency in seconds
    pub static ref DELETE_LATENCY: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "scribe_ledger_delete_latency_seconds",
            "DELETE request latency in seconds"
        )
        .buckets(vec![0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0])
    ).unwrap();

    // Storage metrics
    /// Total number of keys stored
    pub static ref STORAGE_KEYS: IntGauge = IntGauge::new(
        "scribe_ledger_storage_keys_total",
        "Total number of keys in storage"
    ).unwrap();

    /// Storage size in bytes
    pub static ref STORAGE_SIZE: IntGauge = IntGauge::new(
        "scribe_ledger_storage_size_bytes",
        "Storage size in bytes"
    ).unwrap();

    // Raft consensus metrics
    /// Current Raft term
    pub static ref RAFT_TERM: IntGauge = IntGauge::new(
        "scribe_ledger_raft_term",
        "Current Raft term"
    ).unwrap();

    /// Raft commit index
    pub static ref RAFT_COMMIT_INDEX: IntGauge = IntGauge::new(
        "scribe_ledger_raft_commit_index",
        "Current Raft commit index"
    ).unwrap();

    /// Raft last applied index
    pub static ref RAFT_LAST_APPLIED: IntGauge = IntGauge::new(
        "scribe_ledger_raft_last_applied",
        "Last applied Raft log index"
    ).unwrap();

    /// Node health status (1 = healthy, 0 = unhealthy)
    pub static ref NODE_HEALTH: IntGauge = IntGauge::new(
        "scribe_ledger_node_health",
        "Node health status (1 = healthy, 0 = unhealthy)"
    ).unwrap();

    // Throughput metrics
    /// Operations per second counter
    pub static ref OPS_TOTAL: IntCounter = IntCounter::new(
        "scribe_ledger_operations_total",
        "Total number of operations processed"
    ).unwrap();

    // Error metrics
    /// Total number of errors
    pub static ref ERRORS_TOTAL: IntCounter = IntCounter::new(
        "scribe_ledger_errors_total",
        "Total number of errors"
    ).unwrap();
}

static INIT: Once = Once::new();

/// Initialize and register all metrics (idempotent - can be called multiple times)
pub fn init_metrics() {
    INIT.call_once(|| {
        // Register request counters
        REGISTRY
            .register(Box::new(GET_REQUESTS.clone()))
            .expect("Failed to register GET_REQUESTS metric");
        REGISTRY
            .register(Box::new(PUT_REQUESTS.clone()))
            .expect("Failed to register PUT_REQUESTS metric");
        REGISTRY
            .register(Box::new(DELETE_REQUESTS.clone()))
            .expect("Failed to register DELETE_REQUESTS metric");

        // Register latency histograms
        REGISTRY
            .register(Box::new(GET_LATENCY.clone()))
            .expect("Failed to register GET_LATENCY metric");
        REGISTRY
            .register(Box::new(PUT_LATENCY.clone()))
            .expect("Failed to register PUT_LATENCY metric");
        REGISTRY
            .register(Box::new(DELETE_LATENCY.clone()))
            .expect("Failed to register DELETE_LATENCY metric");

        // Register storage metrics
        REGISTRY
            .register(Box::new(STORAGE_KEYS.clone()))
            .expect("Failed to register STORAGE_KEYS metric");
        REGISTRY
            .register(Box::new(STORAGE_SIZE.clone()))
            .expect("Failed to register STORAGE_SIZE metric");

        // Register Raft metrics
        REGISTRY
            .register(Box::new(RAFT_TERM.clone()))
            .expect("Failed to register RAFT_TERM metric");
        REGISTRY
            .register(Box::new(RAFT_COMMIT_INDEX.clone()))
            .expect("Failed to register RAFT_COMMIT_INDEX metric");
        REGISTRY
            .register(Box::new(RAFT_LAST_APPLIED.clone()))
            .expect("Failed to register RAFT_LAST_APPLIED metric");
        REGISTRY
            .register(Box::new(NODE_HEALTH.clone()))
            .expect("Failed to register NODE_HEALTH metric");

        // Register throughput metrics
        REGISTRY
            .register(Box::new(OPS_TOTAL.clone()))
            .expect("Failed to register OPS_TOTAL metric");

        // Register error metrics
        REGISTRY
            .register(Box::new(ERRORS_TOTAL.clone()))
            .expect("Failed to register ERRORS_TOTAL metric");

        // Set initial node health to healthy
        NODE_HEALTH.set(1);
    });
}

/// Get metrics in Prometheus text format
pub fn get_metrics() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .expect("Failed to encode metrics");
    String::from_utf8(buffer).expect("Failed to convert metrics to string")
}

/// Update storage metrics
pub fn update_storage_metrics(keys: usize, size: u64) {
    STORAGE_KEYS.set(keys as i64);
    STORAGE_SIZE.set(size as i64);
}

/// Update Raft metrics
pub fn update_raft_metrics(term: u64, commit_index: u64, last_applied: u64) {
    RAFT_TERM.set(term as i64);
    RAFT_COMMIT_INDEX.set(commit_index as i64);
    RAFT_LAST_APPLIED.set(last_applied as i64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_initialization() {
        init_metrics();
        // Verify that metrics can be gathered
        let metrics = get_metrics();
        assert!(metrics.contains("scribe_ledger_get_requests_total"));
        assert!(metrics.contains("scribe_ledger_put_requests_total"));
        assert!(metrics.contains("scribe_ledger_node_health"));
    }

    #[test]
    fn test_request_counters() {
        init_metrics();
        let initial_gets = GET_REQUESTS.get();
        GET_REQUESTS.inc();
        assert_eq!(GET_REQUESTS.get(), initial_gets + 1);

        let initial_puts = PUT_REQUESTS.get();
        PUT_REQUESTS.inc();
        assert_eq!(PUT_REQUESTS.get(), initial_puts + 1);
    }

    #[test]
    fn test_latency_histogram() {
        init_metrics();
        // Record some latencies
        GET_LATENCY.observe(0.001);
        GET_LATENCY.observe(0.010);
        GET_LATENCY.observe(0.100);

        // Verify histogram has recorded observations
        let metrics = get_metrics();
        assert!(metrics.contains("scribe_ledger_get_latency_seconds"));
    }

    #[test]
    fn test_storage_metrics_update() {
        init_metrics();
        update_storage_metrics(100, 1024);
        assert_eq!(STORAGE_KEYS.get(), 100);
        assert_eq!(STORAGE_SIZE.get(), 1024);
    }

    #[test]
    fn test_raft_metrics_update() {
        init_metrics();
        update_raft_metrics(5, 100, 95);
        assert_eq!(RAFT_TERM.get(), 5);
        assert_eq!(RAFT_COMMIT_INDEX.get(), 100);
        assert_eq!(RAFT_LAST_APPLIED.get(), 95);
    }

    #[test]
    fn test_node_health() {
        init_metrics();
        assert_eq!(NODE_HEALTH.get(), 1); // Initially healthy

        NODE_HEALTH.set(0); // Set unhealthy
        assert_eq!(NODE_HEALTH.get(), 0);

        NODE_HEALTH.set(1); // Set healthy again
        assert_eq!(NODE_HEALTH.get(), 1);
    }

    #[test]
    fn test_error_counter() {
        init_metrics();
        let initial_errors = ERRORS_TOTAL.get();
        ERRORS_TOTAL.inc();
        assert_eq!(ERRORS_TOTAL.get(), initial_errors + 1);
    }

    #[test]
    fn test_ops_counter() {
        init_metrics();
        let initial_ops = OPS_TOTAL.get();
        OPS_TOTAL.inc();
        assert_eq!(OPS_TOTAL.get(), initial_ops + 1);
    }
}
