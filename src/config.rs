use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Notify, RwLock};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub performance: PerformanceConfig,
    pub http: HttpConfig,
    pub routes: RoutesConfig,
}

// Dynamic configuration that can be modified at runtime
#[derive(Debug, Clone)]
pub struct DynamicConfig {
    pub server: DynamicServerConfig,
    pub logging: LoggingConfig,
    pub http: Arc<HttpConfig>,
    pub routes: Arc<RoutesConfig>,
    pub performance: DynamicPerformanceConfig,
}

// Dynamic performance configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DynamicPerformanceConfig {
    pub keep_alive_timeout: u64,
    pub read_timeout: u64,
    pub write_timeout: u64,
    pub max_connections: Option<u64>,
}

// Routes configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RoutesConfig {
    pub favicon_paths: Vec<String>,      // Favicon路径列表
    pub custom_routes: HashMap<String, RouteHandler>,  // 自定义路由映射
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RouteHandler {
    Static { dir: String },              // 静态文件目录
    Template { file: String },           // 模板文件
    Redirect { target: String },         // 重定向
}

impl Default for RoutesConfig {
    fn default() -> Self {
        Self {
            favicon_paths: vec!["/favicon.ico".to_string(), "/favicon.svg".to_string()],
            custom_routes: HashMap::new(),
        }
    }
}

// Server configuration that can trigger restart
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct DynamicServerConfig {
    pub host: String,
    pub port: u16,
    pub api_host: String,
    pub api_port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub api_host: String,  // API服务器监听地址
    pub api_port: u16,     // API管理端口
    pub workers: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub access_log: bool,
    pub show_headers: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PerformanceConfig {
    pub keep_alive_timeout: u64,
    pub read_timeout: u64,
    pub write_timeout: u64,
    pub max_connections: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HttpConfig {
    pub default_content_type: String,
    pub server_name: String,
    pub enable_cors: bool,
    pub max_body_size: u64,
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name("config").required(false))
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
            .set_default("http.max_body_size", 10485760)?  // 10MB
            .build()?;

        settings.try_deserialize()
    }

    pub fn get_socket_addr(&self) -> Result<SocketAddr, String> {
        format!("{}:{}", self.server.host, self.server.port)
            .parse()
            .map_err(|e| format!("Invalid address: {}", e))
    }
    
    pub fn get_api_socket_addr(&self) -> Result<SocketAddr, String> {
        format!("{}:{}", self.server.api_host, self.server.api_port)
            .parse()
            .map_err(|e| format!("Invalid API address: {}", e))
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

pub struct AppState {
    pub config: Config,
    pub dynamic_config: RwLock<DynamicConfig>,
    pub restart_signal: Arc<Notify>,
    pub new_server_config: Arc<RwLock<Option<DynamicServerConfig>>>,
    pub api_restart_signal: Arc<Notify>,
    
    // Cached config values for fast access without locks
    pub cached_access_log: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(config: &Config) -> Self {
        let dynamic = config.to_dynamic();
        
        Self {
            config: config.clone(),
            dynamic_config: RwLock::new(dynamic),
            restart_signal: Arc::new(Notify::new()),
            new_server_config: Arc::new(RwLock::new(None)),
            api_restart_signal: Arc::new(Notify::new()),
            cached_access_log: Arc::new(AtomicBool::new(config.logging.access_log)),
        }
    }
    
    /// Update cached configuration values
    pub fn update_cache(&self, new_config: &DynamicConfig) {
        self.cached_access_log.store(new_config.logging.access_log, Ordering::Relaxed);
    }
}
