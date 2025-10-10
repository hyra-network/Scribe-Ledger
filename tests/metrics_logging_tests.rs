/// Integration tests for metrics and logging features (Task 11.1 & 11.2)
///
/// This test suite validates:
/// - Prometheus metrics collection and reporting
/// - Structured logging with tracing
/// - Request correlation IDs
/// - Metrics endpoint functionality
use hyra_scribe_ledger::{logging, metrics};

#[test]
fn test_metrics_initialization() {
    // Initialize metrics system
    metrics::init_metrics();

    // Verify metrics can be gathered
    let metrics_output = metrics::get_metrics();

    // Check for key metrics
    assert!(metrics_output.contains("scribe_ledger_get_requests_total"));
    assert!(metrics_output.contains("scribe_ledger_put_requests_total"));
    assert!(metrics_output.contains("scribe_ledger_delete_requests_total"));
    assert!(metrics_output.contains("scribe_ledger_node_health"));
    assert!(metrics_output.contains("scribe_ledger_operations_total"));
}

#[test]
fn test_request_counter_metrics() {
    metrics::init_metrics();

    // Track some requests
    let initial_gets = metrics::GET_REQUESTS.get();
    metrics::GET_REQUESTS.inc();
    assert_eq!(metrics::GET_REQUESTS.get(), initial_gets + 1);

    let initial_puts = metrics::PUT_REQUESTS.get();
    metrics::PUT_REQUESTS.inc();
    metrics::PUT_REQUESTS.inc();
    assert_eq!(metrics::PUT_REQUESTS.get(), initial_puts + 2);

    let initial_deletes = metrics::DELETE_REQUESTS.get();
    metrics::DELETE_REQUESTS.inc();
    assert_eq!(metrics::DELETE_REQUESTS.get(), initial_deletes + 1);
}

#[test]
fn test_latency_metrics() {
    metrics::init_metrics();

    // Observe some latencies
    metrics::GET_LATENCY.observe(0.001); // 1ms
    metrics::GET_LATENCY.observe(0.010); // 10ms
    metrics::GET_LATENCY.observe(0.050); // 50ms

    metrics::PUT_LATENCY.observe(0.005); // 5ms
    metrics::PUT_LATENCY.observe(0.025); // 25ms

    metrics::DELETE_LATENCY.observe(0.001); // 1ms

    // Verify histograms are in the output
    let metrics_output = metrics::get_metrics();
    assert!(metrics_output.contains("scribe_ledger_get_latency_seconds"));
    assert!(metrics_output.contains("scribe_ledger_put_latency_seconds"));
    assert!(metrics_output.contains("scribe_ledger_delete_latency_seconds"));
}

#[test]
fn test_storage_metrics() {
    metrics::init_metrics();

    // Update storage metrics
    metrics::update_storage_metrics(100, 1024 * 1024); // 100 keys, 1MB

    assert_eq!(metrics::STORAGE_KEYS.get(), 100);
    assert_eq!(metrics::STORAGE_SIZE.get(), 1024 * 1024);

    // Update again
    metrics::update_storage_metrics(200, 2 * 1024 * 1024); // 200 keys, 2MB

    assert_eq!(metrics::STORAGE_KEYS.get(), 200);
    assert_eq!(metrics::STORAGE_SIZE.get(), 2 * 1024 * 1024);
}

#[test]
fn test_raft_metrics() {
    metrics::init_metrics();

    // Update Raft metrics
    metrics::update_raft_metrics(5, 100, 95);

    assert_eq!(metrics::RAFT_TERM.get(), 5);
    assert_eq!(metrics::RAFT_COMMIT_INDEX.get(), 100);
    assert_eq!(metrics::RAFT_LAST_APPLIED.get(), 95);

    // Update again
    metrics::update_raft_metrics(6, 150, 148);

    assert_eq!(metrics::RAFT_TERM.get(), 6);
    assert_eq!(metrics::RAFT_COMMIT_INDEX.get(), 150);
    assert_eq!(metrics::RAFT_LAST_APPLIED.get(), 148);
}

#[test]
fn test_node_health_metric() {
    metrics::init_metrics();

    // Initial state should be healthy
    assert_eq!(metrics::NODE_HEALTH.get(), 1);

    // Set unhealthy
    metrics::NODE_HEALTH.set(0);
    assert_eq!(metrics::NODE_HEALTH.get(), 0);

    // Set healthy again
    metrics::NODE_HEALTH.set(1);
    assert_eq!(metrics::NODE_HEALTH.get(), 1);
}

#[test]
fn test_error_and_ops_counters() {
    metrics::init_metrics();

    let initial_errors = metrics::ERRORS_TOTAL.get();
    let initial_ops = metrics::OPS_TOTAL.get();

    // Track some errors and operations
    metrics::ERRORS_TOTAL.inc();
    metrics::ERRORS_TOTAL.inc();
    metrics::OPS_TOTAL.inc();
    metrics::OPS_TOTAL.inc();
    metrics::OPS_TOTAL.inc();

    assert_eq!(metrics::ERRORS_TOTAL.get(), initial_errors + 2);
    assert_eq!(metrics::OPS_TOTAL.get(), initial_ops + 3);
}

#[test]
fn test_metrics_prometheus_format() {
    metrics::init_metrics();

    // Increment some counters
    metrics::GET_REQUESTS.inc();
    metrics::PUT_REQUESTS.inc();
    metrics::OPS_TOTAL.inc();

    let metrics_output = metrics::get_metrics();

    // Verify Prometheus format
    assert!(metrics_output.contains("# HELP"));
    assert!(metrics_output.contains("# TYPE"));

    // Check for counter types
    assert!(metrics_output.contains("TYPE scribe_ledger_get_requests_total counter"));
    assert!(metrics_output.contains("TYPE scribe_ledger_put_requests_total counter"));

    // Check for histogram types
    assert!(metrics_output.contains("TYPE scribe_ledger_get_latency_seconds histogram"));

    // Check for gauge types
    assert!(metrics_output.contains("TYPE scribe_ledger_storage_keys_total gauge"));
    assert!(metrics_output.contains("TYPE scribe_ledger_node_health gauge"));
}

#[test]
fn test_logging_config_default() {
    let config = logging::LogConfig::default();

    assert_eq!(config.level, tracing::Level::INFO);
    assert_eq!(config.format, logging::LogFormat::Console);
    assert!(!config.enable_file);
    assert!(config.enable_console);
    assert_eq!(config.log_file_prefix, "scribe-ledger");
}

#[test]
fn test_logging_config_custom() {
    let config = logging::LogConfig::new(tracing::Level::DEBUG, logging::LogFormat::Json)
        .with_file_logging("/tmp/test_logs")
        .with_file_prefix("test_app")
        .without_console();

    assert_eq!(config.level, tracing::Level::DEBUG);
    assert_eq!(config.format, logging::LogFormat::Json);
    assert!(config.enable_file);
    assert_eq!(config.log_dir, Some("/tmp/test_logs".to_string()));
    assert_eq!(config.log_file_prefix, "test_app");
    assert!(!config.enable_console);
}

#[test]
fn test_correlation_id_generation() {
    // Generate multiple correlation IDs
    let id1 = logging::generate_correlation_id();
    let id2 = logging::generate_correlation_id();
    let id3 = logging::generate_correlation_id();

    // All should be unique
    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
    assert_ne!(id1, id3);

    // All should be non-empty
    assert!(!id1.is_empty());
    assert!(!id2.is_empty());
    assert!(!id3.is_empty());

    // All should contain hyphen separator
    assert!(id1.contains('-'));
    assert!(id2.contains('-'));
    assert!(id3.contains('-'));
}

#[test]
fn test_correlation_id_format() {
    let id = logging::generate_correlation_id();
    let parts: Vec<&str> = id.split('-').collect();

    // Should have exactly two parts
    assert_eq!(parts.len(), 2);

    // Both parts should be valid hex strings
    assert!(
        u128::from_str_radix(parts[0], 16).is_ok(),
        "First part should be valid hex"
    );
    assert!(
        u64::from_str_radix(parts[1], 16).is_ok(),
        "Second part should be valid hex"
    );
}

#[test]
fn test_latency_percentiles() {
    metrics::init_metrics();

    // Observe latencies that cover different buckets
    let latencies = [
        0.001, 0.002, 0.003, 0.005, 0.007, 0.010, 0.015, 0.020, 0.025, 0.030, 0.040, 0.050, 0.075,
        0.100, 0.150, 0.200, 0.250, 0.300, 0.400, 0.500,
    ];

    for latency in latencies.iter() {
        metrics::GET_LATENCY.observe(*latency);
    }

    let metrics_output = metrics::get_metrics();

    // Verify histogram buckets are present
    assert!(metrics_output.contains("le=\"0.001\""));
    assert!(metrics_output.contains("le=\"0.01\""));
    assert!(metrics_output.contains("le=\"0.05\""));
    assert!(metrics_output.contains("le=\"0.1\""));
    assert!(metrics_output.contains("le=\"0.5\""));
    assert!(metrics_output.contains("le=\"+Inf\""));
}

#[test]
fn test_metrics_idempotent_initialization() {
    // Initialize multiple times should not panic
    metrics::init_metrics();
    metrics::init_metrics();
    metrics::init_metrics();

    // Metrics should still work
    metrics::GET_REQUESTS.inc();
    assert!(metrics::GET_REQUESTS.get() > 0);
}

#[test]
fn test_concurrent_metric_updates() {
    use std::thread;

    metrics::init_metrics();

    let handles: Vec<_> = (0..10)
        .map(|_| {
            thread::spawn(|| {
                for _ in 0..100 {
                    metrics::GET_REQUESTS.inc();
                    metrics::PUT_REQUESTS.inc();
                    metrics::OPS_TOTAL.inc();
                    metrics::GET_LATENCY.observe(0.001);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify metrics were updated (should be at least 1000 each)
    assert!(metrics::GET_REQUESTS.get() >= 1000);
    assert!(metrics::PUT_REQUESTS.get() >= 1000);
    assert!(metrics::OPS_TOTAL.get() >= 1000);
}

#[test]
fn test_log_format_variants() {
    assert_eq!(logging::LogFormat::Console, logging::LogFormat::Console);
    assert_eq!(logging::LogFormat::Json, logging::LogFormat::Json);
    assert_ne!(logging::LogFormat::Console, logging::LogFormat::Json);
}

#[test]
fn test_all_log_levels() {
    let levels = [
        tracing::Level::TRACE,
        tracing::Level::DEBUG,
        tracing::Level::INFO,
        tracing::Level::WARN,
        tracing::Level::ERROR,
    ];

    for level in levels.iter() {
        let config = logging::LogConfig::new(*level, logging::LogFormat::Console);
        assert_eq!(config.level, *level);
    }
}
