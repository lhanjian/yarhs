use crate::config::{AppState, RouteHandler, RoutesConfig};
use crate::logger;
use crate::response;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Method, Request, Response};
use std::convert::Infallible;
use std::sync::Arc;

/// Check HTTP method and return early response if not GET/HEAD
/// Returns Some(response) for OPTIONS/405, None to continue processing
fn check_http_method(method: &Method, enable_cors: bool) -> Option<Response<Full<Bytes>>> {
    match method {
        &Method::GET | &Method::HEAD => None,
        &Method::OPTIONS => Some(response::build_options_response(enable_cors)),
        _ => {
            logger::log_warning(&format!("Method not allowed: {method}"));
            Some(response::build_405_response())
        }
    }
}

/// Validate Content-Length header against max body size
/// Returns Some(413 response) if too large, None otherwise
fn check_body_size(req: &Request<hyper::body::Incoming>, max_body_size: u64) -> Option<Response<Full<Bytes>>> {
    let content_length = req.headers().get("content-length")?;
    content_length.to_str().map_or_else(
        |_| {
            logger::log_warning("Content-Length header contains non-ASCII characters");
            None
        },
        |size_str| match size_str.parse::<u64>() {
            Ok(size) if size > max_body_size => {
                logger::log_error(&format!(
                    "Request body too large: {size} bytes (max: {max_body_size})"
                ));
                Some(response::build_413_response())
            }
            Err(_) => {
                logger::log_warning(&format!(
                    "Invalid Content-Length value: '{size_str}', skipping size check"
                ));
                None
            }
            _ => None,
        },
    )
}

/// Route the request based on path and routes configuration
async fn route_request(
    path: &str,
    routes: &Arc<RoutesConfig>,
    state: &Arc<AppState>,
    access_log: bool,
    if_none_match: Option<&str>,
    range_header: Option<&str>,
    is_head: bool,
) -> Response<Full<Bytes>> {
    // 1. Favicon routes
    if routes.favicon_paths.iter().any(|p| path == p) {
        return response::load_favicon()
            .await
            .map_or_else(response::build_404_response, |favicon_data| {
                let size = favicon_data.len();
                if access_log {
                    logger::log_response(size);
                }
                response::build_favicon_response(&favicon_data, if_none_match, is_head)
            });
    }

    // 2. Custom routes (exact match)
    if let Some(handler) = routes.custom_routes.get(path) {
        return handle_custom_route(path, handler, access_log, path, &routes.index_files, if_none_match, range_header, is_head).await;
    }

    // 3. Custom routes (prefix match)
    if let Some((prefix, handler)) = routes.custom_routes.iter().find(|(p, _)| path.starts_with(p.as_str())) {
        return handle_custom_route(path, handler, access_log, prefix, &routes.index_files, if_none_match, range_header, is_head).await;
    }

    // 4. Default: homepage
    let http_config = {
        let config = state.dynamic_config.read().await;
        Arc::clone(&config.http)
    };
    let html = response::get_default_homepage();
    let html_len = html.len();
    if access_log {
        logger::log_response(html_len);
    }
    response::build_html_response(html, &http_config, is_head)
}

pub async fn handle_request(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let method = req.method();
    let uri = req.uri();
    let path = uri.path();
    let is_head = *method == Method::HEAD;

    let access_log = state.cached_access_log.load(std::sync::atomic::Ordering::Relaxed);
    if access_log {
        logger::log_request(method, uri, req.version());
    }

    // Check HTTP method
    if let Some(resp) = check_http_method(method, state.config.http.enable_cors) {
        return Ok(resp);
    }

    // Check body size
    if let Some(resp) = check_body_size(&req, state.config.http.max_body_size) {
        return Ok(resp);
    }

    // Log headers
    let show_headers = state.dynamic_config.read().await.logging.show_headers;
    logger::log_headers_count(req.headers().len(), show_headers);

    // Extract If-None-Match header
    let if_none_match = req.headers().get("if-none-match")
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);

    // Extract Range header
    let range_header = req.headers().get("range")
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);

    // Get routes and dispatch
    let routes = {
        let config = state.dynamic_config.read().await;
        Arc::clone(&config.routes)
    };

    let response = route_request(path, &routes, &state, access_log, if_none_match.as_deref(), range_header.as_deref(), is_head).await;
    Ok(response)
}

#[allow(clippy::too_many_arguments)]
async fn handle_custom_route(
    path: &str,
    handler: &RouteHandler,
    access_log: bool,
    route_prefix: &str,
    index_files: &[String],
    if_none_match: Option<&str>,
    range_header: Option<&str>,
    is_head: bool,
) -> Response<Full<Bytes>> {
    match handler {
        RouteHandler::Dir { path: dir } => {
            // Serve file from directory with index file support
            if let Some((content, content_type)) =
                response::load_static_file(dir, path, route_prefix, index_files).await
            {
                let size = content.len();
                if access_log {
                    logger::log_response(size);
                }
                response::build_static_file_response(&content, content_type, if_none_match, is_head, range_header)
            } else {
                response::build_404_response()
            }
        }
        RouteHandler::File { path: file_path } => {
            // Load and serve any file with appropriate content type
            if let Some((content, content_type)) = response::load_single_file(file_path).await {
                let size = content.len();
                if access_log {
                    logger::log_response(size);
                }
                response::build_static_file_response(&content, content_type, if_none_match, is_head, range_header)
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
