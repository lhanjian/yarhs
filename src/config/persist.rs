// Configuration persistence module
// Saves dynamic configuration changes to state.toml

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::{
    DynamicPerformanceConfig, DynamicServerConfig, HealthConfig, HttpConfig, LoggingConfig,
    RouteHandler, RoutesConfig, VirtualHost,
};

/// Persistent state - serialized to state.toml
/// Only includes fields that can be modified at runtime via API
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct PersistentState {
    /// Server endpoint configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<DynamicServerConfig>,

    /// Logging configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingConfig>,

    /// HTTP configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpConfig>,

    /// Performance configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub performance: Option<DynamicPerformanceConfig>,

    /// Routes configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routes: Option<PersistentRoutesConfig>,

    /// Virtual hosts configuration
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub virtual_hosts: Vec<VirtualHost>,
}

/// Routes config for persistence (same structure, explicit for serialization)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersistentRoutesConfig {
    #[serde(default)]
    pub index_files: Vec<String>,
    #[serde(default)]
    pub custom_routes: HashMap<String, RouteHandler>,
    #[serde(default)]
    pub health: HealthConfig,
}

impl From<&RoutesConfig> for PersistentRoutesConfig {
    fn from(routes: &RoutesConfig) -> Self {
        Self {
            index_files: routes.index_files.clone(),
            custom_routes: routes.custom_routes.clone(),
            health: routes.health.clone(),
        }
    }
}

impl From<PersistentRoutesConfig> for RoutesConfig {
    fn from(routes: PersistentRoutesConfig) -> Self {
        Self {
            index_files: routes.index_files,
            custom_routes: routes.custom_routes,
            health: routes.health,
        }
    }
}

/// State file manager
pub struct StateManager {
    /// Path to state file
    state_path: PathBuf,
    /// Current state (cached in memory)
    state: RwLock<PersistentState>,
    /// Whether persistence is enabled
    enabled: bool,
}

impl StateManager {
    /// Create a new state manager
    ///
    /// # Arguments
    /// * `config_path` - Path to config.toml (state.toml will be in same directory)
    /// * `enabled` - Whether persistence is enabled
    pub fn new(config_path: &str, enabled: bool) -> Self {
        let config_dir = Path::new(config_path)
            .parent()
            .unwrap_or_else(|| Path::new("."));

        let state_path = config_dir.join("state.toml");

        // Only load existing state if persistence is enabled
        let state = if enabled {
            Self::load_state(&state_path).unwrap_or_default()
        } else {
            PersistentState::default()
        };

        Self {
            state_path,
            state: RwLock::new(state),
            enabled,
        }
    }

    /// Load state from file
    fn load_state(path: &Path) -> Option<PersistentState> {
        if !path.exists() {
            return None;
        }

        match fs::read_to_string(path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(state) => {
                    crate::logger::write_info(&format!(
                        "Loaded persistent state from {}",
                        path.display()
                    ));
                    Some(state)
                }
                Err(e) => {
                    crate::logger::write_error(&format!(
                        "Failed to parse state file {}: {}",
                        path.display(),
                        e
                    ));
                    None
                }
            },
            Err(e) => {
                crate::logger::write_error(&format!(
                    "Failed to read state file {}: {}",
                    path.display(),
                    e
                ));
                None
            }
        }
    }

    /// Save state to file
    async fn save_state(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        let content = {
            let state = self.state.read().await;
            toml::to_string_pretty(&*state)
                .map_err(|e| format!("Failed to serialize state: {e}"))?
        };

        fs::write(&self.state_path, content)
            .map_err(|e| format!("Failed to write state file: {e}"))?;

        Ok(())
    }

    /// Get the current persistent state
    pub async fn get_state(&self) -> PersistentState {
        self.state.read().await.clone()
    }

    /// Update server configuration
    pub async fn update_server(&self, server: &DynamicServerConfig) -> Result<(), String> {
        {
            let mut state = self.state.write().await;
            state.server = Some(server.clone());
        }
        self.save_state().await
    }

    /// Update logging configuration
    pub async fn update_logging(&self, logging: &LoggingConfig) -> Result<(), String> {
        {
            let mut state = self.state.write().await;
            state.logging = Some(logging.clone());
        }
        self.save_state().await
    }

    /// Update HTTP configuration
    pub async fn update_http(&self, http: &HttpConfig) -> Result<(), String> {
        {
            let mut state = self.state.write().await;
            state.http = Some(http.clone());
        }
        self.save_state().await
    }

    /// Update performance configuration
    pub async fn update_performance(&self, perf: &DynamicPerformanceConfig) -> Result<(), String> {
        {
            let mut state = self.state.write().await;
            state.performance = Some(perf.clone());
        }
        self.save_state().await
    }

    /// Update routes configuration
    pub async fn update_routes(&self, routes: &RoutesConfig) -> Result<(), String> {
        {
            let mut state = self.state.write().await;
            state.routes = Some(PersistentRoutesConfig::from(routes));
        }
        self.save_state().await
    }

    /// Update virtual hosts configuration
    pub async fn update_virtual_hosts(&self, vhosts: &[VirtualHost]) -> Result<(), String> {
        {
            let mut state = self.state.write().await;
            state.virtual_hosts = vhosts.to_vec();
        }
        self.save_state().await
    }

    /// Clear all persisted state (reset to config.toml defaults)
    pub async fn clear(&self) -> Result<(), String> {
        {
            let mut state = self.state.write().await;
            *state = PersistentState::default();
        }

        if self.enabled && self.state_path.exists() {
            fs::remove_file(&self.state_path)
                .map_err(|e| format!("Failed to remove state file: {e}"))?;
        }

        Ok(())
    }

    /// Get state file path
    #[allow(clippy::missing_const_for_fn)]
    pub fn state_path(&self) -> &Path {
        &self.state_path
    }

    /// Check if persistence is enabled
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Wrapper for Arc<StateManager>
pub type SharedStateManager = Arc<StateManager>;

/// Create a shared state manager
pub fn create_state_manager(config_path: &str, enabled: bool) -> SharedStateManager {
    Arc::new(StateManager::new(config_path, enabled))
}
