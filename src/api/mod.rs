// API module entry
// xDS-style configuration management API

mod dashboard;
mod handlers;
mod response;
mod types;
mod updaters;

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Method, Request, Response};
use std::convert::Infallible;
use std::sync::Arc;

use crate::config::{AppState, ResourceType};
use crate::logger;

// Re-export public types
pub use response::*;

/// API route handler
///
/// Dispatches to handler functions based on request path and method
pub async fn handle_api_config(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = req.uri().path();
    let method = req.method().clone();

    // xDS style routes
    match (method, path) {
        // Dashboard - Web UI
        (Method::GET, "/" | "/dashboard") => Ok(dashboard::serve_dashboard()),
        // Get all resources snapshot
        (Method::GET, "/v1/discovery") => handlers::handle_snapshot(state).await,
        // Discover specific resource type (Listener)
        (Method::GET, "/v1/discovery:listeners") => {
            handlers::handle_discovery_get(state, ResourceType::Listener).await
        }
        (Method::POST, "/v1/discovery:listeners") => {
            handlers::handle_discovery_post(req, state, ResourceType::Listener).await
        }
        // Discover route resources
        (Method::GET, "/v1/discovery:routes") => {
            handlers::handle_discovery_get(state, ResourceType::Route).await
        }
        (Method::POST, "/v1/discovery:routes") => {
            handlers::handle_discovery_post(req, state, ResourceType::Route).await
        }
        // Discover HTTP configuration
        (Method::GET, "/v1/discovery:http") => {
            handlers::handle_discovery_get(state, ResourceType::Http).await
        }
        (Method::POST, "/v1/discovery:http") => {
            handlers::handle_discovery_post(req, state, ResourceType::Http).await
        }
        // Discover logging configuration
        (Method::GET, "/v1/discovery:logging") => {
            handlers::handle_discovery_get(state, ResourceType::Logging).await
        }
        (Method::POST, "/v1/discovery:logging") => {
            handlers::handle_discovery_post(req, state, ResourceType::Logging).await
        }
        // Discover performance configuration
        (Method::GET, "/v1/discovery:performance") => {
            handlers::handle_discovery_get(state, ResourceType::Performance).await
        }
        (Method::POST, "/v1/discovery:performance") => {
            handlers::handle_discovery_post(req, state, ResourceType::Performance).await
        }
        // Discover virtual_hosts configuration
        (Method::GET, "/v1/discovery:vhosts") => {
            handlers::handle_discovery_get(state, ResourceType::VirtualHost).await
        }
        (Method::POST, "/v1/discovery:vhosts") => {
            handlers::handle_discovery_post(req, state, ResourceType::VirtualHost).await
        }
        // State persistence management
        (Method::GET, "/v1/state") => handlers::handle_state_get(state).await,
        (Method::DELETE, "/v1/state") => handlers::handle_state_clear(state).await,
        // Unknown route
        _ => {
            logger::log_api_request(req.method().as_str(), path, 404);
            Ok(not_found())
        }
    }
}
