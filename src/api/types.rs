// API types module
// Request/response types for xDS Discovery API

use crate::config::{
    DynamicPerformanceConfig, HealthConfig, HttpConfig, LoggingConfig, RouteHandler,
};
use serde::Serialize;
use std::collections::HashMap;

// ============== xDS API Types ==============

/// xDS Discovery Response - returned by server
#[derive(Debug, Serialize)]
pub struct DiscoveryResponse {
    /// Resource version
    pub version_info: String,
    /// Resource list
    pub resources: Vec<Resource>,
    /// Response nonce, client must return in next request
    pub nonce: String,
    /// Resource type URL
    pub type_url: String,
}

/// Generic resource wrapper
#[derive(Debug, Serialize)]
pub struct Resource {
    /// Resource type
    #[serde(rename = "@type")]
    pub type_url: String,
    /// Resource name
    pub name: String,
    /// Resource content
    #[serde(flatten)]
    pub value: serde_json::Value,
}

/// Snapshot response of all resources
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
    pub health: HealthConfig,
}
