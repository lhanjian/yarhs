// Application state module
// Manages runtime state and configuration cache

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};

use super::types::{Config, DynamicConfig, DynamicServerConfig};
use super::version::XdsVersionManager;

/// Application state
pub struct AppState {
    pub config: Config,
    pub dynamic_config: RwLock<DynamicConfig>,
    pub restart_signal: Arc<Notify>,
    pub new_server_config: Arc<RwLock<Option<DynamicServerConfig>>>,
    pub api_restart_signal: Arc<Notify>,

    // Cached config values for fast access without locks
    pub cached_access_log: Arc<AtomicBool>,

    // xDS version management
    pub xds_versions: XdsVersionManager,
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
            xds_versions: XdsVersionManager::new(),
        }
    }

    /// Update cached configuration values
    pub fn update_cache(&self, new_config: &DynamicConfig) {
        self.cached_access_log
            .store(new_config.logging.access_log, Ordering::Relaxed);
    }
}
