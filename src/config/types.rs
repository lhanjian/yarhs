// Configuration types module
// Defines all configuration-related data structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Main configuration structure
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub performance: PerformanceConfig,
    pub http: HttpConfig,
    pub routes: RoutesConfig,
}

/// Dynamic configuration - can be modified at runtime
#[derive(Debug, Clone)]
pub struct DynamicConfig {
    pub server: DynamicServerConfig,
    pub logging: LoggingConfig,
    pub http: Arc<HttpConfig>,
    pub routes: Arc<RoutesConfig>,
    pub performance: DynamicPerformanceConfig,
}

/// Dynamic performance configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DynamicPerformanceConfig {
    pub keep_alive_timeout: u64,
    pub read_timeout: u64,
    pub write_timeout: u64,
    pub max_connections: Option<u64>,
}

/// Routes configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RoutesConfig {
    pub favicon_paths: Vec<String>,
    pub index_files: Vec<String>,
    pub custom_routes: HashMap<String, RouteHandler>,
    /// Health check configuration
    #[serde(default)]
    pub health: HealthConfig,
}

/// Health check configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HealthConfig {
    /// Enable health check endpoints
    #[serde(default = "default_health_enabled")]
    pub enabled: bool,
    /// Liveness probe path (default: /healthz)
    #[serde(default = "default_healthz_path")]
    pub liveness_path: String,
    /// Readiness probe path (default: /readyz)
    #[serde(default = "default_readyz_path")]
    pub readiness_path: String,
}

#[allow(clippy::missing_const_for_fn)]
fn default_health_enabled() -> bool {
    true
}

#[allow(clippy::missing_const_for_fn)]
fn default_healthz_path() -> String {
    "/healthz".to_string()
}

#[allow(clippy::missing_const_for_fn)]
fn default_readyz_path() -> String {
    "/readyz".to_string()
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            enabled: default_health_enabled(),
            liveness_path: default_healthz_path(),
            readiness_path: default_readyz_path(),
        }
    }
}

/// Route handler types
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RouteHandler {
    Dir { path: String },
    File { path: String },
    Redirect { target: String },
}

impl Default for RoutesConfig {
    fn default() -> Self {
        Self {
            favicon_paths: vec!["/favicon.ico".to_string(), "/favicon.svg".to_string()],
            index_files: vec!["index.html".to_string(), "index.htm".to_string()],
            custom_routes: HashMap::new(),
            health: HealthConfig::default(),
        }
    }
}

/// Dynamic server configuration - may trigger restart
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct DynamicServerConfig {
    pub host: String,
    pub port: u16,
    pub api_host: String,
    pub api_port: u16,
}

/// Server configuration
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub api_host: String,
    pub api_port: u16,
    pub workers: Option<usize>,
}

/// Logging configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub access_log: bool,
    pub show_headers: bool,
    /// Access log format (combined, common, json, or custom pattern)
    #[serde(default = "default_access_log_format")]
    pub access_log_format: String,
    /// Access log file path (optional, stdout if not set)
    #[serde(default)]
    pub access_log_file: Option<String>,
    /// Error log file path (optional, stderr if not set)
    #[serde(default)]
    pub error_log_file: Option<String>,
}

#[allow(clippy::missing_const_for_fn)]
fn default_access_log_format() -> String {
    "combined".to_string()
}

/// Performance configuration
#[derive(Debug, Deserialize, Clone)]
pub struct PerformanceConfig {
    pub keep_alive_timeout: u64,
    pub read_timeout: u64,
    pub write_timeout: u64,
    pub max_connections: Option<u64>,
}

/// HTTP configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HttpConfig {
    pub default_content_type: String,
    pub server_name: String,
    pub enable_cors: bool,
    pub max_body_size: u64,
}
