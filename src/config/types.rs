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
    /// Virtual hosts configuration (xDS-compatible)
    pub virtual_hosts: Arc<Vec<VirtualHost>>,
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

// ============================================
// xDS-compatible Virtual Host types
// ============================================

/// Virtual host configuration - routes requests based on Host header
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VirtualHost {
    /// Unique name for this virtual host
    pub name: String,
    /// Domain patterns to match (e.g., "api.example.com", "*.example.com", "*")
    pub domains: Vec<String>,
    /// Routes within this virtual host (matched in order)
    #[serde(default)]
    pub routes: Vec<Route>,
    /// Default index files for this virtual host
    #[serde(default)]
    pub index_files: Option<Vec<String>>,
}

/// xDS Route - matches requests and dispatches to actions
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Route {
    /// Optional route name for identification
    #[serde(default)]
    pub name: Option<String>,
    /// Match conditions (prefix, path, headers)
    #[serde(rename = "match")]
    pub match_rule: RouteMatch,
    /// Action to take when matched
    #[serde(flatten)]
    pub action: RouteAction,
}

/// Route matching conditions
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct RouteMatch {
    /// Path prefix match (e.g., "/api" matches "/api/users")
    #[serde(default)]
    pub prefix: Option<String>,
    /// Exact path match
    #[serde(default)]
    pub path: Option<String>,
    /// Header matchers (optional)
    #[serde(default)]
    pub headers: Option<Vec<HeaderMatcher>>,
}

/// Header matching condition
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HeaderMatcher {
    /// Header name
    pub name: String,
    /// Expected value (exact match)
    #[serde(default)]
    pub exact: Option<String>,
    /// Prefix match
    #[serde(default)]
    pub prefix: Option<String>,
    /// Check if header is present
    #[serde(default)]
    pub present: Option<bool>,
}

/// Route action - what to do when a route matches
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RouteAction {
    /// Serve files from a directory
    Dir { path: String },
    /// Serve a specific file
    File { path: String },
    /// HTTP redirect
    Redirect {
        target: String,
        #[serde(default = "default_redirect_code")]
        code: u16,
    },
    /// Direct response (e.g., for health checks, errors)
    Direct {
        status: u16,
        #[serde(default)]
        body: Option<String>,
        #[serde(default)]
        content_type: Option<String>,
    },
}

#[allow(clippy::missing_const_for_fn)]
fn default_redirect_code() -> u16 {
    302
}

impl RouteAction {
    /// Convert legacy `RouteHandler` to `RouteAction`
    #[allow(dead_code)]
    pub fn from_handler(handler: &RouteHandler) -> Self {
        match handler {
            RouteHandler::Dir { path } => Self::Dir { path: path.clone() },
            RouteHandler::File { path } => Self::File { path: path.clone() },
            RouteHandler::Redirect { target } => Self::Redirect {
                target: target.clone(),
                code: 302,
            },
        }
    }
}
