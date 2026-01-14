// xDS version management module
// Manages version numbers and nonces for configuration resources

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// xDS resource type definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResourceType {
    Listener,
    Route,
    Http,
    Logging,
    Performance,
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Listener => write!(f, "LISTENER"),
            Self::Route => write!(f, "ROUTE"),
            Self::Http => write!(f, "HTTP"),
            Self::Logging => write!(f, "LOGGING"),
            Self::Performance => write!(f, "PERFORMANCE"),
        }
    }
}

/// Versioned resource state
#[derive(Debug)]
pub struct VersionedResource {
    pub version: AtomicU64,
    pub nonce: AtomicU64,
}

impl VersionedResource {
    pub fn new() -> Self {
        let ts = u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        )
        .unwrap_or_default();
        Self {
            version: AtomicU64::new(ts),
            nonce: AtomicU64::new(1),
        }
    }

    pub fn increment(&self) -> (u64, u64) {
        let new_version = u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        )
        .unwrap_or_default();
        self.version.store(new_version, Ordering::SeqCst);
        let new_nonce = self.nonce.fetch_add(1, Ordering::SeqCst) + 1;
        (new_version, new_nonce)
    }

    pub fn get(&self) -> (u64, u64) {
        (
            self.version.load(Ordering::SeqCst),
            self.nonce.load(Ordering::SeqCst),
        )
    }
}

impl Default for VersionedResource {
    fn default() -> Self {
        Self::new()
    }
}

/// xDS resource version manager
pub struct XdsVersionManager {
    pub listener: VersionedResource,
    pub route: VersionedResource,
    pub http: VersionedResource,
    pub logging: VersionedResource,
    pub performance: VersionedResource,
}

impl XdsVersionManager {
    pub fn new() -> Self {
        Self {
            listener: VersionedResource::new(),
            route: VersionedResource::new(),
            http: VersionedResource::new(),
            logging: VersionedResource::new(),
            performance: VersionedResource::new(),
        }
    }

    pub const fn get_resource(&self, resource_type: ResourceType) -> &VersionedResource {
        match resource_type {
            ResourceType::Listener => &self.listener,
            ResourceType::Route => &self.route,
            ResourceType::Http => &self.http,
            ResourceType::Logging => &self.logging,
            ResourceType::Performance => &self.performance,
        }
    }

    pub fn increment(&self, resource_type: ResourceType) -> (u64, u64) {
        self.get_resource(resource_type).increment()
    }

    pub fn get_version(&self, resource_type: ResourceType) -> (u64, u64) {
        self.get_resource(resource_type).get()
    }
}

impl Default for XdsVersionManager {
    fn default() -> Self {
        Self::new()
    }
}
