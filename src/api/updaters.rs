// 资源更新函数模块

use std::sync::Arc;
use crate::config::{
    AppState, HttpConfig, LoggingConfig, RoutesConfig, DynamicPerformanceConfig,
};
use serde::Deserialize;

/// 更新 Listener 配置
pub async fn update_listener(
    state: &Arc<AppState>,
    resource: &serde_json::Value,
    force_restart: bool,
) -> Result<String, String> {
    #[derive(Deserialize)]
    struct ListenerUpdate {
        main_server: Option<ServerEndpointUpdate>,
        api_server: Option<ServerEndpointUpdate>,
    }
    
    #[derive(Deserialize)]
    struct ServerEndpointUpdate {
        host: String,
        port: u16,
    }
    
    let update: ListenerUpdate = serde_json::from_value(resource.clone())
        .map_err(|e| format!("Invalid listener resource: {e}"))?;
    
    let (port_changed, api_port_changed) = {
        let mut config = state.dynamic_config.write().await;
        let mut port_changed = false;
        let mut api_port_changed = false;
        
        if let Some(main) = &update.main_server {
            if config.server.host != main.host || config.server.port != main.port {
                port_changed = true;
                config.server.host.clone_from(&main.host);
                config.server.port = main.port;
            }
        }
        
        if let Some(api) = &update.api_server {
            if config.server.api_host != api.host || config.server.api_port != api.port {
                api_port_changed = true;
                config.server.api_host.clone_from(&api.host);
                config.server.api_port = api.port;
            }
        }
        drop(config);
        
        (port_changed, api_port_changed)
    };
    
    // 触发重启
    if port_changed || api_port_changed || force_restart {
        let new_config = {
            let config = state.dynamic_config.read().await;
            config.server.clone()
        };
        
        {
            let mut cfg = state.new_server_config.write().await;
            *cfg = Some(new_config.clone());
        }
        
        if port_changed || force_restart {
            state.restart_signal.notify_one();
        }
        if api_port_changed || force_restart {
            state.api_restart_signal.notify_one();
        }
        
        let mut changes = Vec::new();
        if port_changed { changes.push("main_server"); }
        if api_port_changed { changes.push("api_server"); }
        if force_restart && changes.is_empty() { changes.push("forced"); }
        
        Ok(format!("Listener updated, restarting: {}", changes.join(", ")))
    } else {
        Ok("Listener config unchanged".to_string())
    }
}

/// 更新 Route 配置
pub async fn update_route(
    state: &Arc<AppState>,
    resource: &serde_json::Value,
) -> Result<String, String> {
    let routes: RoutesConfig = serde_json::from_value(resource.clone())
        .map_err(|e| format!("Invalid route resource: {e}"))?;
    
    {
        let mut config = state.dynamic_config.write().await;
        config.routes = Arc::new(routes);
    }
    
    Ok("Routes updated".to_string())
}

/// 更新 HTTP 配置
pub async fn update_http(
    state: &Arc<AppState>,
    resource: &serde_json::Value,
) -> Result<String, String> {
    let http: HttpConfig = serde_json::from_value(resource.clone())
        .map_err(|e| format!("Invalid HTTP resource: {e}"))?;
    
    {
        let mut config = state.dynamic_config.write().await;
        config.http = Arc::new(http);
    }
    
    Ok("HTTP config updated".to_string())
}

/// 更新 Logging 配置
pub async fn update_logging(
    state: &Arc<AppState>,
    resource: &serde_json::Value,
) -> Result<String, String> {
    let logging: LoggingConfig = serde_json::from_value(resource.clone())
        .map_err(|e| format!("Invalid logging resource: {e}"))?;
    
    {
        let mut config = state.dynamic_config.write().await;
        config.logging = logging.clone();
    }
    
    // Update cache
    state.update_cache(&*state.dynamic_config.read().await);
    
    Ok("Logging config updated".to_string())
}

/// 更新 Performance 配置
pub async fn update_performance(
    state: &Arc<AppState>,
    resource: &serde_json::Value,
) -> Result<String, String> {
    let performance: DynamicPerformanceConfig = serde_json::from_value(resource.clone())
        .map_err(|e| format!("Invalid performance resource: {e}"))?;
    
    {
        let mut config = state.dynamic_config.write().await;
        config.performance = performance;
    }
    
    Ok("Performance config updated".to_string())
}
