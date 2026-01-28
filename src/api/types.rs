// API types module
// Request/response types for xDS Discovery API

use crate::config::{
    DynamicPerformanceConfig, HealthConfig, HttpConfig, LoggingConfig, RouteHandler, VirtualHost,
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
    pub virtual_hosts: VersionedValue<Vec<VirtualHost>>,
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
    /// Number of worker threads (read-only, set at startup). None means auto-detect.
    #[serde(serialize_with = "serialize_workers")]
    pub workers: Option<usize>,
}

/// Serialize workers field - None becomes "auto"
/// Note: serde's `serialize_with` requires `&Option<T>` signature, cannot change to `Option<&T>`
#[allow(clippy::ref_option, clippy::trivially_copy_pass_by_ref)]
fn serialize_workers<S>(workers: &Option<usize>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match workers {
        Some(n) => serializer.serialize_str(&n.to_string()),
        None => serializer.serialize_str("auto"),
    }
}

#[derive(Debug, Serialize)]
pub struct ServerEndpoint {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Serialize)]
pub struct RouteResource {
    pub index_files: Vec<String>,
    pub custom_routes: HashMap<String, RouteHandler>,
    pub health: HealthConfig,
}
