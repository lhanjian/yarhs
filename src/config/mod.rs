// Configuration module entry point
// Manages application configuration, runtime state, and version control

mod state;
mod types;
mod version;

use std::net::SocketAddr;
use std::sync::Arc;

// Re-export public types
pub use state::AppState;
pub use types::{
    Config, DynamicConfig, DynamicPerformanceConfig, DynamicServerConfig, HealthConfig, HttpConfig,
    LoggingConfig, RouteHandler, RoutesConfig,
};
pub use version::ResourceType;

impl Config {
    /// Load configuration from specified file path (without extension)
    /// Default config file is "config.toml" when no path specified
    pub fn load_from(config_path: &str) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name(config_path).required(false))
            .add_source(config::Environment::with_prefix("SERVER"))
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.port", 8080)?
            .set_default("server.api_host", "127.0.0.1")?
            .set_default("server.api_port", 8000)?
            .set_default("logging.level", "info")?
            .set_default("logging.access_log", true)?
            .set_default("logging.show_headers", false)?
            .set_default("performance.keep_alive_timeout", 75)?
            .set_default("performance.read_timeout", 30)?
            .set_default("performance.write_timeout", 30)?
            .set_default("http.default_content_type", "text/html; charset=utf-8")?
            .set_default("http.server_name", "Tokio-Hyper/1.0")?
            .set_default("http.enable_cors", false)?
            .set_default("http.max_body_size", 10_485_760)? // 10MB
            .build()?;

        settings.try_deserialize()
    }

    pub fn get_socket_addr(&self) -> Result<SocketAddr, String> {
        format!("{}:{}", self.server.host, self.server.port)
            .parse()
            .map_err(|e| format!("Invalid address: {e}"))
    }

    pub fn get_api_socket_addr(&self) -> Result<SocketAddr, String> {
        format!("{}:{}", self.server.api_host, self.server.api_port)
            .parse()
            .map_err(|e| format!("Invalid API address: {e}"))
    }

    pub fn to_dynamic(&self) -> DynamicConfig {
        DynamicConfig {
            server: DynamicServerConfig {
                host: self.server.host.clone(),
                port: self.server.port,
                api_host: self.server.api_host.clone(),
                api_port: self.server.api_port,
            },
            logging: self.logging.clone(),
            http: Arc::new(self.http.clone()),
            routes: Arc::new(self.routes.clone()),
            performance: DynamicPerformanceConfig {
                keep_alive_timeout: self.performance.keep_alive_timeout,
                read_timeout: self.performance.read_timeout,
                write_timeout: self.performance.write_timeout,
                max_connections: self.performance.max_connections,
            },
        }
    }
}
