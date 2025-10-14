/// Advanced logging configuration with structured logging, log rotation, and correlation IDs
///
/// This module provides production-ready logging capabilities using the tracing framework,
/// including structured logging, log levels, log rotation, and request correlation IDs.
use std::io;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// Log output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable console output
    Console,
    /// JSON structured logs
    Json,
}

/// Log configuration
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Log level (debug, info, warn, error)
    pub level: Level,
    /// Output format
    pub format: LogFormat,
    /// Enable file logging
    pub enable_file: bool,
    /// Log file directory (if file logging enabled)
    pub log_dir: Option<String>,
    /// Log file prefix
    pub log_file_prefix: String,
    /// Enable console logging
    pub enable_console: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: Level::INFO,
            format: LogFormat::Console,
            enable_file: false,
            log_dir: None,
            log_file_prefix: "scribe-ledger".to_string(),
            enable_console: true,
        }
    }
}

impl LogConfig {
    /// Create a new log configuration with custom settings
    pub fn new(level: Level, format: LogFormat) -> Self {
        Self {
            level,
            format,
            ..Default::default()
        }
    }

    /// Enable file logging with rotation
    pub fn with_file_logging(mut self, log_dir: &str) -> Self {
        self.enable_file = true;
        self.log_dir = Some(log_dir.to_string());
        self
    }

    /// Set log file prefix
    pub fn with_file_prefix(mut self, prefix: &str) -> Self {
        self.log_file_prefix = prefix.to_string();
        self
    }

    /// Disable console logging
    pub fn without_console(mut self) -> Self {
        self.enable_console = false;
        self
    }
}

/// Initialize logging system with the given configuration
///
/// Returns a WorkerGuard that must be kept alive for the duration of the program.
/// Dropping the guard will stop log writing to files.
pub fn init_logging(config: LogConfig) -> Option<WorkerGuard> {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(config.level.as_str()));

    let mut guard = None;

    // Build the subscriber with layers
    let registry = tracing_subscriber::registry().with(env_filter);

    if config.enable_file && config.log_dir.is_some() {
        let log_dir = config.log_dir.as_ref().unwrap();

        // Create log directory if it doesn't exist
        std::fs::create_dir_all(log_dir).expect("Failed to create log directory");

        // Set up file appender with rotation
        let file_appender = tracing_appender::rolling::daily(log_dir, &config.log_file_prefix);
        let (non_blocking, worker_guard) = tracing_appender::non_blocking(file_appender);

        // Create file layer
        let file_layer = match config.format {
            LogFormat::Console => fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_span_events(FmtSpan::CLOSE)
                .boxed(),
            LogFormat::Json => fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_span_events(FmtSpan::CLOSE)
                .boxed(),
        };

        guard = Some(worker_guard);

        if config.enable_console {
            // Both console and file
            let console_layer = match config.format {
                LogFormat::Console => fmt::layer()
                    .with_writer(io::stdout)
                    .with_span_events(FmtSpan::CLOSE)
                    .boxed(),
                LogFormat::Json => fmt::layer()
                    .json()
                    .with_writer(io::stdout)
                    .with_span_events(FmtSpan::CLOSE)
                    .boxed(),
            };

            registry.with(file_layer).with(console_layer).init();
        } else {
            // File only
            registry.with(file_layer).init();
        }
    } else if config.enable_console {
        // Console only
        let console_layer = match config.format {
            LogFormat::Console => fmt::layer()
                .with_writer(io::stdout)
                .with_span_events(FmtSpan::CLOSE)
                .boxed(),
            LogFormat::Json => fmt::layer()
                .json()
                .with_writer(io::stdout)
                .with_span_events(FmtSpan::CLOSE)
                .boxed(),
        };

        registry.with(console_layer).init();
    }

    guard
}

/// Generate a correlation ID for request tracing
pub fn generate_correlation_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_micros();

    // Generate a simple correlation ID with timestamp and random component
    format!("{:x}-{:x}", timestamp, fastrand::u64(..))
}

// Re-export fastrand for correlation ID generation
use fastrand;

/// Audit event types for security logging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEvent {
    /// Authentication attempt
    AuthAttempt,
    /// Authentication success
    AuthSuccess,
    /// Authentication failure
    AuthFailure,
    /// Authorization check
    AuthzCheck,
    /// Authorization denied
    AuthzDenied,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Data access (read)
    DataRead,
    /// Data modification (write)
    DataWrite,
    /// Data deletion
    DataDelete,
    /// Configuration change
    ConfigChange,
    /// System event
    SystemEvent,
}

impl AuditEvent {
    /// Get event name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditEvent::AuthAttempt => "auth_attempt",
            AuditEvent::AuthSuccess => "auth_success",
            AuditEvent::AuthFailure => "auth_failure",
            AuditEvent::AuthzCheck => "authz_check",
            AuditEvent::AuthzDenied => "authz_denied",
            AuditEvent::RateLimitExceeded => "rate_limit_exceeded",
            AuditEvent::DataRead => "data_read",
            AuditEvent::DataWrite => "data_write",
            AuditEvent::DataDelete => "data_delete",
            AuditEvent::ConfigChange => "config_change",
            AuditEvent::SystemEvent => "system_event",
        }
    }
}

/// Log an audit event with structured data
pub fn audit_log(
    event: AuditEvent,
    user: Option<&str>,
    action: &str,
    resource: Option<&str>,
    result: &str,
    details: Option<&str>,
) {
    use tracing::info;

    info!(
        audit_event = event.as_str(),
        user = user.unwrap_or("anonymous"),
        action = action,
        resource = resource.unwrap_or("none"),
        result = result,
        details = details.unwrap_or(""),
        "Audit event"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_log_config() {
        let config = LogConfig::default();
        assert_eq!(config.level, Level::INFO);
        assert_eq!(config.format, LogFormat::Console);
        assert!(!config.enable_file);
        assert!(config.enable_console);
    }

    #[test]
    fn test_custom_log_config() {
        let config = LogConfig::new(Level::DEBUG, LogFormat::Json)
            .with_file_logging("/tmp/test_logs")
            .with_file_prefix("test")
            .without_console();

        assert_eq!(config.level, Level::DEBUG);
        assert_eq!(config.format, LogFormat::Json);
        assert!(config.enable_file);
        assert_eq!(config.log_dir, Some("/tmp/test_logs".to_string()));
        assert_eq!(config.log_file_prefix, "test");
        assert!(!config.enable_console);
    }

    #[test]
    fn test_correlation_id_generation() {
        let id1 = generate_correlation_id();
        let id2 = generate_correlation_id();

        // IDs should be unique
        assert_ne!(id1, id2);

        // IDs should be non-empty
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());

        // IDs should contain a hyphen separator
        assert!(id1.contains('-'));
        assert!(id2.contains('-'));
    }

    #[test]
    fn test_correlation_id_format() {
        let id = generate_correlation_id();
        let parts: Vec<&str> = id.split('-').collect();

        // Should have two parts
        assert_eq!(parts.len(), 2);

        // Both parts should be valid hex strings
        assert!(u128::from_str_radix(parts[0], 16).is_ok());
        assert!(u64::from_str_radix(parts[1], 16).is_ok());
    }

    #[test]
    fn test_log_config_builder_pattern() {
        let config = LogConfig::default()
            .with_file_logging("/tmp/logs")
            .with_file_prefix("myapp");

        assert!(config.enable_file);
        assert_eq!(config.log_dir, Some("/tmp/logs".to_string()));
        assert_eq!(config.log_file_prefix, "myapp");
        assert!(config.enable_console); // Still enabled by default
    }

    #[test]
    fn test_log_format_variants() {
        assert_eq!(LogFormat::Console, LogFormat::Console);
        assert_eq!(LogFormat::Json, LogFormat::Json);
        assert_ne!(LogFormat::Console, LogFormat::Json);
    }

    #[test]
    fn test_audit_event_enum() {
        assert_eq!(AuditEvent::AuthSuccess.as_str(), "auth_success");
        assert_eq!(AuditEvent::AuthFailure.as_str(), "auth_failure");
        assert_eq!(AuditEvent::AuthzDenied.as_str(), "authz_denied");
        assert_eq!(AuditEvent::DataRead.as_str(), "data_read");
        assert_eq!(AuditEvent::DataWrite.as_str(), "data_write");
    }

    #[test]
    fn test_audit_log_function() {
        // Just verify the function can be called without panic
        audit_log(
            AuditEvent::AuthSuccess,
            Some("testuser"),
            "login",
            Some("/auth"),
            "success",
            Some("User logged in successfully"),
        );
    }
}
