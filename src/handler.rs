use std::convert::Infallible;
use std::sync::Arc;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Request, Response};
use crate::config::{AppState, RouteHandler};
use crate::logger;
use crate::response;

pub async fn handle_request(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let method = req.method();
    let uri = req.uri();
    let version = req.version();
    let path = uri.path();
    
    // Use cached access_log flag to avoid lock
    let access_log = state.cached_access_log.load(std::sync::atomic::Ordering::Relaxed);
    
    if access_log {
        logger::log_request(method, uri, version);
    }
    
    // Read lightweight logging config only (avoid cloning heavy http_config)
    let show_headers = {
        let config = state.dynamic_config.read().await;
        config.logging.show_headers
    };
    
    logger::log_headers_count(req.headers().len(), show_headers);
    
    // Check request body size limit
    let max_body_size = state.config.http.max_body_size;
    if let Some(content_length) = req.headers().get("content-length") {
        if let Ok(size_str) = content_length.to_str() {
            if let Ok(size) = size_str.parse::<u64>() {
                if size > max_body_size {
                    eprintln!("[ERROR] Request body too large: {} bytes (max: {})", size, max_body_size);
                    return Ok(response::build_413_response());
                }
            }
        }
    }
    
    // Get routes configuration (Arc reference, no clone)
    let routes = {
        let config = state.dynamic_config.read().await;
        Arc::clone(&config.routes)
    };
    
    // Route handling with dynamic configuration
    // Note: API routes are handled by separate API server on port 8000
    let response = 
        // 1. Favicon routes
        if routes.favicon_paths.iter().any(|p| path == p) {
            if let Some(favicon_data) = response::load_favicon().await {
                let size = favicon_data.len();
                if access_log {
                    logger::log_response(size);
                }
                response::build_favicon_response(favicon_data)
            } else {
                response::build_404_response()
            }
        }
        // 3. Custom routes (exact match)
        else if let Some(handler) = routes.custom_routes.get(path) {
            handle_custom_route(path, handler, &state, access_log, path).await
        }
        // 4. Check for prefix-based custom routes (e.g., /static/*)
        else if let Some((prefix, handler)) = routes.custom_routes.iter()
            .find(|(prefix, _)| path.starts_with(prefix.as_str())) 
        {
            handle_custom_route(path, handler, &state, access_log, prefix).await
        }
        // 5. Default: Simple homepage
        else {
            let http_config = {
                let config = state.dynamic_config.read().await;
                Arc::clone(&config.http)
            };
            
            let html = response::get_default_homepage();
            let html_len = html.len();
            let resp = response::build_html_response(html, http_config);
            
            if access_log {
                logger::log_response(html_len);
            }
            resp
        };
    
    Ok(response)
}

async fn handle_custom_route(
    path: &str,
    handler: &RouteHandler,
    state: &Arc<AppState>,
    access_log: bool,
    route_prefix: &str,  // Add route prefix parameter
) -> Response<Full<Bytes>> {
    match handler {
        RouteHandler::Static { dir } => {
            // Serve file from custom static directory
            if let Some((content, content_type)) = response::load_static_file(dir, path, route_prefix).await {
                let size = content.len();
                if access_log {
                    logger::log_response(size);
                }
                response::build_static_file_response(content, content_type)
            } else {
                response::build_404_response()
            }
        }
        RouteHandler::Template { file } => {
            // Load and serve HTML template
            if let Ok(content) = tokio::fs::read_to_string(file).await {
                let http_config = {
                    let config = state.dynamic_config.read().await;
                    Arc::clone(&config.http)
                };
                let size = content.len();
                if access_log {
                    logger::log_response(size);
                }
                response::build_html_response(content, http_config)
            } else {
                response::build_404_response()
            }
        }
        RouteHandler::Redirect { target } => {
            // Build redirect response
            response::build_redirect_response(target)
        }
    }
}
