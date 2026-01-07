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
    pub resources: ResourcesConfig,
    pub performance: PerformanceConfig,
    pub http: HttpConfig,
}

// Dynamic configuration that can be modified at runtime
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DynamicConfig {
    pub logging: LoggingConfig,
    pub http: HttpConfig,
    pub resources: DynamicResourcesConfig,
    pub routes: RoutesConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DynamicResourcesConfig {
    pub template_dir: String,
}

// Routes configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RoutesConfig {
    pub api_prefix: String,              // API路由前缀，如 "/api"
    pub static_prefix: String,           // 静态文件路由前缀，如 "/static"
    pub favicon_paths: Vec<String>,      // Favicon路径列表
    pub custom_routes: HashMap<String, RouteHandler>,  // 自定义路由映射
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RouteHandler {
    Static { dir: String },              // 静态文件目录
    Template { file: String },           // 模板文件
    Markdown { file: String },           // Markdown文件
    Redirect { target: String },         // 重定向
}

impl Default for RoutesConfig {
    fn default() -> Self {
        Self {
            api_prefix: "/api".to_string(),
            static_prefix: "/static".to_string(),
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
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub access_log: bool,
    pub show_headers: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ResourcesConfig {
    pub template_dir: String,
    pub static_dir: Option<String>,
    pub max_body_size: u64,
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
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name("config").required(false))
            .add_source(config::Environment::with_prefix("SERVER"))
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.port", 8080)?
            .set_default("logging.level", "info")?
            .set_default("logging.access_log", true)?
            .set_default("logging.show_headers", false)?
            .set_default("resources.template_dir", "templates")?
            .set_default("resources.max_body_size", 10485760)?  // 10MB
            .set_default("performance.keep_alive_timeout", 75)?
            .set_default("performance.read_timeout", 30)?
            .set_default("performance.write_timeout", 30)?
            .set_default("http.default_content_type", "text/html; charset=utf-8")?
            .set_default("http.server_name", "Tokio-Hyper/1.0")?
            .set_default("http.enable_cors", false)?
            .build()?;

        settings.try_deserialize()
    }

    pub fn get_socket_addr(&self) -> Result<SocketAddr, String> {
        format!("{}:{}", self.server.host, self.server.port)
            .parse()
            .map_err(|e| format!("Invalid address: {}", e))
    }

    pub fn to_dynamic(&self) -> DynamicConfig {
        DynamicConfig {
            logging: self.logging.clone(),
            http: self.http.clone(),
            resources: DynamicResourcesConfig {
                template_dir: self.resources.template_dir.clone(),
            },
            routes: RoutesConfig::default(),
        }
    }
    
    pub fn get_dynamic_server_config(&self) -> DynamicServerConfig {
        DynamicServerConfig {
            host: self.server.host.clone(),
            port: self.server.port,
        }
    }
}

pub struct AppState {
    pub config: Config,
    pub current_server_config: Arc<RwLock<DynamicServerConfig>>,
    pub dynamic_config: RwLock<DynamicConfig>,
    pub restart_signal: Arc<Notify>,
    pub new_server_config: Arc<RwLock<Option<DynamicServerConfig>>>,
    pub markdown_cache: RwLock<Option<String>>,
    
    // Cached config values for fast access without locks
    pub cached_access_log: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(config: &Config) -> Self {
        let dynamic = config.to_dynamic();
        let server_config = config.get_dynamic_server_config();
        
        Self {
            config: config.clone(),
            current_server_config: Arc::new(RwLock::new(server_config)),
            dynamic_config: RwLock::new(dynamic),
            restart_signal: Arc::new(Notify::new()),
            new_server_config: Arc::new(RwLock::new(None)),
            markdown_cache: RwLock::new(None),
            cached_access_log: Arc::new(AtomicBool::new(config.logging.access_log)),
        }
    }
    
    /// Update cached configuration values
    pub fn update_cache(&self, new_config: &DynamicConfig) {
        self.cached_access_log.store(new_config.logging.access_log, Ordering::Relaxed);
    }
}
