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
    match req.method() {
        &Method::GET => handle_get_config(state).await,
        &Method::PUT => handle_put_config(req, state).await,
        _ => Ok(method_not_allowed()),
    }
}

async fn handle_get_config(state: Arc<AppState>) -> Result<Response<Full<Bytes>>, Infallible> {
    let dynamic_config = state.dynamic_config.read().await;
    let server_config = state.current_server_config.read().await;
    
    let full_config = serde_json::json!({
        "server": {
            "host": server_config.host,
            "port": server_config.port
        },
        "logging": dynamic_config.logging,
        "http": dynamic_config.http,
        "resources": dynamic_config.resources,
        "routes": dynamic_config.routes
    });
    
    let json = serde_json::to_string_pretty(&full_config).unwrap();
    
    logger::log_api_request("GET", "/api/config", 200);
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap())
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
        resources: crate::config::DynamicResourcesConfig,
        routes: crate::config::RoutesConfig,
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
        config.logging = full_config.logging.clone();
        config.http = full_config.http.clone();
        config.resources = full_config.resources.clone();
        config.routes = full_config.routes.clone();
    }
    
    // Update cached config values
    state.update_cache(&crate::config::DynamicConfig {
        logging: full_config.logging,
        http: full_config.http,
        resources: full_config.resources,
        routes: full_config.routes,
    });
    
    // Check if server config changed or force_restart is true
    if let Some(new_server_config) = full_config.server {
        let current_config = state.current_server_config.read().await;
        
        // Trigger restart if config changed OR force_restart is true
        if *current_config != new_server_config || full_config.force_restart {
            logger::log_api_request("PUT", "/api/config", 200);
            
            if full_config.force_restart && *current_config == new_server_config {
                println!("[RESTART] Force restart requested for same address: {}:{}", 
                         new_server_config.host, new_server_config.port);
            } else {
                logger::log_server_config_change(&current_config, &new_server_config);
            }
            
            // Store new server config and trigger restart
            {
                let mut cfg = state.new_server_config.write().await;
                *cfg = Some(new_server_config.clone());
            }
            state.restart_signal.notify_one();
            
            let message = if full_config.force_restart && *current_config == new_server_config {
                "Configuration updated. Server will restart on same address (force restart)."
            } else {
                "Configuration updated. Server will restart with new host/port."
            };
            
            let response_body = serde_json::json!({
                "status": "ok",
                "message": message,
                "new_address": format!("{}:{}", new_server_config.host, new_server_config.port)
            });
            
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(response_body.to_string())))
                .unwrap());
        }
    }
    
    logger::log_api_request("PUT", "/api/config", 200);
    logger::log_config_updated();
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(r#"{"status":"ok","message":"Configuration updated"}"#)))
        .unwrap())
}

fn method_not_allowed() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::METHOD_NOT_ALLOWED)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from("Method Not Allowed")))
        .unwrap()
}

fn bad_request(message: &str) -> Response<Full<Bytes>> {
    let body = format!(r#"{{"error":"{}"}}"#, message);
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}
