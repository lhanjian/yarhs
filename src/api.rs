use std::convert::Infallible;
use std::sync::Arc;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Request, Response, Method, StatusCode};
use crate::config::{AppState, DynamicServerConfig};
use crate::logger;

pub async fn handle_api_config(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = req.uri().path();
    
    // Validate path matches /api/config
    if path != "/api/config" {
        logger::log_api_request(req.method().as_str(), path, 404);
        return Ok(not_found());
    }
    
    match *req.method() {
        Method::GET => handle_get_config(state).await,
        Method::PUT => handle_put_config(req, state).await,
        _ => Ok(method_not_allowed()),
    }
}

async fn handle_get_config(state: Arc<AppState>) -> Result<Response<Full<Bytes>>, Infallible> {
    let dynamic_config = state.dynamic_config.read().await;
    
    let full_config = serde_json::json!({
        "server": {
            "host": dynamic_config.server.host,
            "port": dynamic_config.server.port,
            "api_host": dynamic_config.server.api_host,
            "api_port": dynamic_config.server.api_port
        },
        "logging": dynamic_config.logging,
        "http": &*dynamic_config.http,
        "routes": &*dynamic_config.routes,
        "performance": dynamic_config.performance
    });
    
    let json = match serde_json::to_string_pretty(&full_config) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("[ERROR] Failed to serialize config: {}", e);
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(r#"{"error":"Internal server error"}"#)))
                .unwrap_or_else(|_| Response::new(Full::new(Bytes::from("Error")))));
        }
    };
    
    logger::log_api_request("GET", "/api/config", 200);
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap_or_else(|e| {
            eprintln!("[ERROR] Failed to build response: {}", e);
            Response::new(Full::new(Bytes::from("Error")))
        }))
}

async fn handle_put_config(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    use http_body_util::BodyExt;
    
    // Read request body
    let whole_body = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => {
            logger::log_api_request("PUT", "/api/config", 400);
            return Ok(bad_request("Failed to read request body"));
        }
    };
    
    // Try to parse with server config first
    #[derive(serde::Deserialize)]
    struct FullConfig {
        server: Option<DynamicServerConfig>,
        logging: crate::config::LoggingConfig,
        http: crate::config::HttpConfig,
        routes: crate::config::RoutesConfig,
        performance: crate::config::DynamicPerformanceConfig,
        #[serde(default)]
        force_restart: bool,
    }
    
    let full_config: FullConfig = match serde_json::from_slice(&whole_body) {
        Ok(config) => config,
        Err(e) => {
            logger::log_api_request("PUT", "/api/config", 400);
            return Ok(bad_request(&format!("Invalid JSON: {}", e)));
        }
    };
    
    // Update dynamic configuration
    {
        let mut config = state.dynamic_config.write().await;
        config.server = full_config.server.clone().unwrap_or(config.server.clone());
        config.logging = full_config.logging.clone();
        config.http = Arc::new(full_config.http.clone());
        config.routes = Arc::new(full_config.routes.clone());
        config.performance = full_config.performance.clone();
    }
    
    // Update cached config values
    state.update_cache(&crate::config::DynamicConfig {
        server: full_config.server.clone().unwrap_or_else(|| {
            let current = state.dynamic_config.blocking_read();
            current.server.clone()
        }),
        logging: full_config.logging,
        http: Arc::new(full_config.http),
        routes: Arc::new(full_config.routes),
        performance: full_config.performance,
    });
    
    // Check if server config changed or force_restart is true
    if let Some(new_server_config) = full_config.server {
        let (port_changed, api_port_changed) = {
            let dynamic = state.dynamic_config.read().await;
            let current_config = &dynamic.server;
            
            println!("[CONFIG] ========== Configuration Change Detection ==========");
            println!("[CONFIG] Current config: host={}, port={}, api_host={}, api_port={}", 
                     current_config.host, current_config.port, current_config.api_host, current_config.api_port);
            println!("[CONFIG] New config: host={}, port={}, api_host={}, api_port={}", 
                     new_server_config.host, new_server_config.port, new_server_config.api_host, new_server_config.api_port);
            
            // Check what changed
            let port_changed = current_config.port != new_server_config.port || 
                              current_config.host != new_server_config.host;
            let api_port_changed = current_config.api_port != new_server_config.api_port ||
                                  current_config.api_host != new_server_config.api_host;
            
            println!("[CONFIG] Change detection:");
            println!("[CONFIG]   - Main server (host/port) changed: {}", port_changed);
            println!("[CONFIG]   - API server (api_host/api_port) changed: {}", api_port_changed);
            println!("[CONFIG]   - Force restart: {}", full_config.force_restart);
            
            (port_changed, api_port_changed)
        }; // Release read lock here
        
        // Trigger restart if config changed OR force_restart is true
        if port_changed || api_port_changed || full_config.force_restart {
            logger::log_api_request("PUT", "/api/config", 200);
            
            {
                let dynamic = state.dynamic_config.read().await;
                if full_config.force_restart && dynamic.server == new_server_config {
                    println!("[RESTART] Force restart requested for same address");
                } else {
                    logger::log_server_config_change(&dynamic.server, &new_server_config);
                }
            }
            
            // Store new server config and trigger restart(s)
            {
                let mut cfg = state.new_server_config.write().await;
                *cfg = Some(new_server_config.clone());
                println!("[CONFIG] New server config stored for restart");
            }
            
            // Note: dynamic_config.server is already updated above, no need to update again
            
            println!("[CONFIG] All locks released, sending restart signals");
            
            // Trigger main server restart if port/host changed
            if port_changed || full_config.force_restart {
                println!("[CONFIG] Triggering main server restart signal");
                state.restart_signal.notify_one();
            }
            
            // Trigger API server restart if api_port changed
            if api_port_changed || full_config.force_restart {
                println!("[CONFIG] Triggering API server restart signal");
                state.api_restart_signal.notify_one();
            }
            
            println!("[CONFIG] ========== Restart Signals Sent ==========");
            
            let mut changes = Vec::new();
            if port_changed {
                changes.push(format!("Main server: {}:{}", new_server_config.host, new_server_config.port));
            }
            if api_port_changed {
                changes.push(format!("API server: {}:{}", new_server_config.api_host, new_server_config.api_port));
            }
            
            let message = if changes.is_empty() {
                "Configuration updated. Servers will restart (force restart).".to_string()
            } else {
                format!("Configuration updated. Restarting: {}", changes.join(", "))
            };
            
            let response_body = serde_json::json!({
                "status": "ok",
                "message": message,
                "main_address": format!("{}:{}", new_server_config.host, new_server_config.port),
                "api_address": format!("{}:{}", new_server_config.api_host, new_server_config.api_port)
            });
            
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(response_body.to_string())))
                .unwrap_or_else(|e| {
                    eprintln!("[ERROR] Failed to build restart response: {}", e);
                    Response::new(Full::new(Bytes::from("OK")))
                }));
        }
    }
    
    logger::log_api_request("PUT", "/api/config", 200);
    logger::log_config_updated();
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(r#"{"status":"ok","message":"Configuration updated"}"#)))
        .unwrap_or_else(|e| {
            eprintln!("[ERROR] Failed to build response: {}", e);
            Response::new(Full::new(Bytes::from("OK")))
        }))
}

fn method_not_allowed() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::METHOD_NOT_ALLOWED)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from("Method Not Allowed")))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::from("Method Not Allowed"))))
}

fn not_found() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(r#"{"error":"Not Found","message":"Only /api/config is supported"}"#)))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::from("Not Found"))))
}

fn bad_request(message: &str) -> Response<Full<Bytes>> {
    let body = format!(r#"{{"error":"{}"}}"#, message);
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::from("Bad Request"))))
}
