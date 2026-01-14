// 配置类型定义模块
// 定义所有配置相关的数据结构

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// 主配置结构
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub performance: PerformanceConfig,
    pub http: HttpConfig,
    pub routes: RoutesConfig,
}

/// 动态配置 - 可在运行时修改
#[derive(Debug, Clone)]
pub struct DynamicConfig {
    pub server: DynamicServerConfig,
    pub logging: LoggingConfig,
    pub http: Arc<HttpConfig>,
    pub routes: Arc<RoutesConfig>,
    pub performance: DynamicPerformanceConfig,
}

/// 动态性能配置
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DynamicPerformanceConfig {
    pub keep_alive_timeout: u64,
    pub read_timeout: u64,
    pub write_timeout: u64,
    pub max_connections: Option<u64>,
}

/// 路由配置
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RoutesConfig {
    pub favicon_paths: Vec<String>,
    pub index_files: Vec<String>,
    pub custom_routes: HashMap<String, RouteHandler>,
}

/// 路由处理器类型
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
        }
    }
}

/// 动态服务器配置 - 可触发重启
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct DynamicServerConfig {
    pub host: String,
    pub port: u16,
    pub api_host: String,
    pub api_port: u16,
}

/// 服务器配置
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub api_host: String,
    pub api_port: u16,
    pub workers: Option<usize>,
}

/// 日志配置
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub access_log: bool,
    pub show_headers: bool,
}

/// 性能配置
#[derive(Debug, Deserialize, Clone)]
pub struct PerformanceConfig {
    pub keep_alive_timeout: u64,
    pub read_timeout: u64,
    pub write_timeout: u64,
    pub max_connections: Option<u64>,
}

/// HTTP 配置
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HttpConfig {
    pub default_content_type: String,
    pub server_name: String,
    pub enable_cors: bool,
    pub max_body_size: u64,
}
