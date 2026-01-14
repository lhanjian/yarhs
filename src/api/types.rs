// API 类型定义模块
// xDS Discovery API 的请求/响应类型

use serde::{Deserialize, Serialize};
use crate::config::{HttpConfig, LoggingConfig, DynamicPerformanceConfig, RouteHandler};
use std::collections::HashMap;

// ============== xDS API Types ==============

/// xDS Discovery Request - 客户端发送的请求
/// (预留用于未来的流式订阅功能)
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct DiscoveryRequest {
    /// 客户端已知的版本号，空表示首次请求
    #[serde(default)]
    pub version_info: String,
    /// 上次响应的 nonce，用于 ACK/NACK
    #[serde(default)]
    pub response_nonce: String,
    /// 请求的资源类型
    #[serde(default)]
    pub type_url: String,
    /// 请求的具体资源名称列表（空表示所有）
    #[serde(default)]
    pub resource_names: Vec<String>,
    /// 错误详情（用于 NACK）
    #[serde(default)]
    pub error_detail: Option<ErrorDetail>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorDetail {
    pub code: i32,
    pub message: String,
}

/// xDS Discovery Response - 服务端返回的响应
#[derive(Debug, Serialize)]
pub struct DiscoveryResponse {
    /// 资源版本号
    pub version_info: String,
    /// 资源列表
    pub resources: Vec<Resource>,
    /// 响应 nonce，客户端需要在下次请求中回传
    pub nonce: String,
    /// 资源类型 URL
    pub type_url: String,
}

/// 通用资源包装器
#[derive(Debug, Serialize)]
pub struct Resource {
    /// 资源类型
    #[serde(rename = "@type")]
    pub type_url: String,
    /// 资源名称
    pub name: String,
    /// 资源内容
    #[serde(flatten)]
    pub value: serde_json::Value,
}

/// 所有资源的快照响应
#[derive(Debug, Serialize)]
pub struct SnapshotResponse {
    pub version_info: String,
    pub resources: ResourceSnapshot,
}

#[derive(Debug, Serialize)]
pub struct ResourceSnapshot {
    pub listener: VersionedValue<ListenerResource>,
    pub route: VersionedValue<RouteResource>,
    pub http: VersionedValue<HttpConfig>,
    pub logging: VersionedValue<LoggingConfig>,
    pub performance: VersionedValue<DynamicPerformanceConfig>,
}

#[derive(Debug, Serialize)]
pub struct VersionedValue<T> {
    pub version_info: String,
    pub nonce: String,
    pub value: T,
}

#[derive(Debug, Serialize)]
pub struct ListenerResource {
    pub main_server: ServerEndpoint,
    pub api_server: ServerEndpoint,
}

#[derive(Debug, Serialize)]
pub struct ServerEndpoint {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Serialize)]
pub struct RouteResource {
    pub favicon_paths: Vec<String>,
    pub index_files: Vec<String>,
    pub custom_routes: HashMap<String, RouteHandler>,
}
