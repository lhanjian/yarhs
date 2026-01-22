//! Request routing dispatch module
//!
//! Entry point for HTTP request processing, responsible for method validation, route matching, and dispatching.

use crate::config::{AppState, RouteHandler, RoutesConfig};
use crate::handler::static_files;
use crate::http;
use crate::logger;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Method, Request, Response};
use std::convert::Infallible;
use std::sync::Arc;

/// Request context encapsulating information needed for request processing
pub struct RequestContext<'a> {
    pub path: &'a str,
    pub is_head: bool,
    pub if_none_match: Option<String>,
    pub if_modified_since: Option<String>,
    pub range_header: Option<String>,
    pub access_log: bool,
}

/// Main entry point for HTTP request handling
pub async fn handle_request(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let method = req.method();
    let uri = req.uri();
    let path = uri.path();
    let is_head = *method == Method::HEAD;

    let access_log = state
        .cached_access_log
        .load(std::sync::atomic::Ordering::Relaxed);
    if access_log {
        logger::log_request(method, uri, req.version());
    }

    // 1. Check HTTP method
    if let Some(resp) = check_http_method(method, state.config.http.enable_cors) {
        return Ok(resp);
    }

    // 2. Check body size
    if let Some(resp) = check_body_size(&req, state.config.http.max_body_size) {
        return Ok(resp);
    }

    // 3. Log headers if enabled
    let show_headers = state.dynamic_config.read().await.logging.show_headers;
    logger::log_headers_count(req.headers().len(), show_headers);

    // 4. Extract headers for caching and range requests
    let ctx = RequestContext {
        path,
        is_head,
        if_none_match: req
            .headers()
            .get("if-none-match")
            .and_then(|v| v.to_str().ok())
            .map(ToString::to_string),
        if_modified_since: req
            .headers()
            .get("if-modified-since")
            .and_then(|v| v.to_str().ok())
            .map(ToString::to_string),
        range_header: req
            .headers()
            .get("range")
            .and_then(|v| v.to_str().ok())
            .map(ToString::to_string),
        access_log,
    };

    // 5. Get routes config and dispatch
    let routes = {
        let config = state.dynamic_config.read().await;
        Arc::clone(&config.routes)
    };

    let response = route_request(&ctx, &routes, &state).await;
    Ok(response)
}

/// Check HTTP method and return appropriate response for non-GET/HEAD methods
fn check_http_method(method: &Method, enable_cors: bool) -> Option<Response<Full<Bytes>>> {
    match method {
        &Method::GET | &Method::HEAD => None,
        &Method::OPTIONS => Some(http::build_options_response(enable_cors)),
        _ => {
            logger::log_warning(&format!("Method not allowed: {method}"));
            Some(http::build_405_response())
        }
    }
}

/// Validate Content-Length header and return 413 if exceeded
fn check_body_size(
    req: &Request<hyper::body::Incoming>,
    max_body_size: u64,
) -> Option<Response<Full<Bytes>>> {
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
                Some(http::build_413_response())
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

/// Route request based on path and configuration
async fn route_request(
    ctx: &RequestContext<'_>,
    routes: &Arc<RoutesConfig>,
    state: &Arc<AppState>,
) -> Response<Full<Bytes>> {
    // 0. Health check endpoints (highest priority, always fast)
    if routes.health.enabled {
        if ctx.path == routes.health.liveness_path {
            return http::build_health_response("ok");
        }
        if ctx.path == routes.health.readiness_path {
            // Readiness can include additional checks in the future
            return http::build_health_response("ok");
        }
    }

    // 1. Favicon routes
    if routes.favicon_paths.iter().any(|p| ctx.path == p) {
        return static_files::serve_favicon(ctx).await;
    }

    // 2. Custom routes (exact match)
    if let Some(handler) = routes.custom_routes.get(ctx.path) {
        return dispatch_route_handler(ctx, handler, ctx.path, &routes.index_files).await;
    }

    // 3. Custom routes (prefix match)
    if let Some((prefix, handler)) = routes
        .custom_routes
        .iter()
        .find(|(p, _)| ctx.path.starts_with(p.as_str()))
    {
        return dispatch_route_handler(ctx, handler, prefix, &routes.index_files).await;
    }

    // 4. Default: homepage
    serve_default_homepage(ctx, state).await
}

/// Dispatch to specific route handler
async fn dispatch_route_handler(
    ctx: &RequestContext<'_>,
    handler: &RouteHandler,
    route_prefix: &str,
    index_files: &[String],
) -> Response<Full<Bytes>> {
    match handler {
        RouteHandler::Dir { path: dir } => {
            static_files::serve_directory(ctx, dir, route_prefix, index_files).await
        }
        RouteHandler::File { path: file_path } => static_files::serve_file(ctx, file_path).await,
        RouteHandler::Redirect { target } => http::build_redirect_response(target),
    }
}

/// Serve default homepage
async fn serve_default_homepage(
    ctx: &RequestContext<'_>,
    state: &Arc<AppState>,
) -> Response<Full<Bytes>> {
    let _http_config = {
        let config = state.dynamic_config.read().await;
        Arc::clone(&config.http)
    };

    let html = static_files::get_default_homepage();
    let html_len = html.len();

    if ctx.access_log {
        logger::log_response(html_len);
    }

    http::response::build_html_response(html, ctx.is_head)
}
