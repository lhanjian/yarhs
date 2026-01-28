// Application state module
// Manages runtime state and configuration cache

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};

use super::persist::SharedStateManager;
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

    // State persistence manager
    pub state_manager: SharedStateManager,
}

impl AppState {
    /// Create `AppState` with persisted state applied
    /// This loads state.toml and merges it with config.toml
    pub async fn new(config: &Config, state_manager: SharedStateManager) -> Self {
        // Get persisted state and merge with base config
        let persisted_state = state_manager.get_state().await;
        let dynamic = config.to_dynamic_with_state(&persisted_state);
        
        // Update cached values based on merged config
        let cached_access_log = Arc::new(AtomicBool::new(dynamic.logging.access_log));

        Self {
            config: config.clone(),
            dynamic_config: RwLock::new(dynamic),
            restart_signal: Arc::new(Notify::new()),
            new_server_config: Arc::new(RwLock::new(None)),
            api_restart_signal: Arc::new(Notify::new()),
            cached_access_log,
            xds_versions: XdsVersionManager::new(),
            state_manager,
        }
    }

    /// Update cached configuration values
    pub fn update_cache(&self, new_config: &DynamicConfig) {
        self.cached_access_log
            .store(new_config.logging.access_log, Ordering::Relaxed);
    }
}
