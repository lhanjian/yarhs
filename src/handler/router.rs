//! Request routing dispatch module
//!
//! Entry point for HTTP request processing, responsible for method validation, route matching, and dispatching.

use crate::config::{AppState, RouteAction, RouteHandler, RoutesConfig, VirtualHost};
use crate::handler::static_files;
use crate::http;
use crate::logger;
use crate::routing;
use http_body_util::Full;
use hyper::body::{Body, Bytes};
use hyper::{Method, Request, Response};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

/// Get elapsed time in microseconds, saturating to `u64::MAX` if overflow
#[inline]
#[allow(clippy::cast_possible_truncation)]
fn elapsed_micros(start: Instant) -> u64 {
    start.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

/// Request context encapsulating information needed for request processing
pub struct RequestContext<'a> {
    pub path: &'a str,
    pub is_head: bool,
    pub if_none_match: Option<String>,
    pub if_modified_since: Option<String>,
    pub range_header: Option<String>,
}

/// Main entry point for HTTP request handling
#[allow(clippy::too_many_lines)]
pub async fn handle_request(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
    remote_addr: SocketAddr,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let start_time = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path().to_string();
    let query = uri.query().map(ToString::to_string);
    let http_version = format!("{:?}", req.version()).replace("HTTP/", "");
    let is_head = method == Method::HEAD;

    // Extract headers for logging
    let referer = req
        .headers()
        .get("referer")
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);
    
    // Extract Host header for virtual host routing
    let host = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost")
        .to_string();

    let access_log = state
        .cached_access_log
        .load(std::sync::atomic::Ordering::Relaxed);

    // Get log format early
    let log_format = {
        let config = state.dynamic_config.read().await;
        config.logging.access_log_format.clone()
    };

    // 1. Check HTTP method
    if let Some(resp) = check_http_method(&method, state.config.http.enable_cors) {
        if access_log {
            logger::log_access_request(
                &remote_addr,
                method.as_str(),
                &path,
                query.as_deref(),
                &http_version,
                resp.status().as_u16(),
                0,
                referer.as_deref(),
                user_agent.as_deref(),
                elapsed_micros(start_time),
                &log_format,
            );
        }
        return Ok(resp);
    }

    // 2. Check body size
    if let Some(resp) = check_body_size(&req, state.config.http.max_body_size) {
        if access_log {
            logger::log_access_request(
                &remote_addr,
                method.as_str(),
                &path,
                query.as_deref(),
                &http_version,
                resp.status().as_u16(),
                0,
                referer.as_deref(),
                user_agent.as_deref(),
                elapsed_micros(start_time),
                &log_format,
            );
        }
        return Ok(resp);
    }

    // 3. Log headers if enabled
    let show_headers = state.dynamic_config.read().await.logging.show_headers;
    logger::log_headers_count(req.headers().len(), show_headers);

    // 4. Extract headers for caching and range requests
    let ctx = RequestContext {
        path: &path,
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
    };

    // 5. Get config and dispatch based on virtual hosts or legacy routes
    let (virtual_hosts, routes) = {
        let config = state.dynamic_config.read().await;
        (Arc::clone(&config.virtual_hosts), Arc::clone(&config.routes))
    };

    let response = if virtual_hosts.is_empty() {
        // Fallback to legacy route configuration
        route_request(&ctx, &routes, &state).await
    } else {
        // Use xDS-style virtual host routing
        route_with_vhosts(&ctx, &host, &virtual_hosts, &routes, &state).await
    };

    // Log access after response is built
    if access_log {
        #[allow(clippy::cast_possible_truncation)]
        let body_bytes = response
            .body()
            .size_hint()
            .exact()
            .unwrap_or(0) as usize;
        logger::log_access_request(
            &remote_addr,
            method.as_str(),
            &path,
            query.as_deref(),
            &http_version,
            response.status().as_u16(),
            body_bytes,
            referer.as_deref(),
            user_agent.as_deref(),
            elapsed_micros(start_time),
            &log_format,
        );
    }

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

/// Route request using xDS-style virtual hosts
async fn route_with_vhosts(
    ctx: &RequestContext<'_>,
    host: &str,
    virtual_hosts: &[VirtualHost],
    legacy_routes: &Arc<RoutesConfig>,
    state: &Arc<AppState>,
) -> Response<Full<Bytes>> {
    // 0. Health check endpoints (global, highest priority)
    if legacy_routes.health.enabled {
        if ctx.path == legacy_routes.health.liveness_path {
            return http::build_health_response("ok");
        }
        if ctx.path == legacy_routes.health.readiness_path {
            return http::build_health_response("ok");
        }
    }

    // 1. Find matching virtual host
    let Some(vhost) = routing::resolve_virtual_host(host, virtual_hosts) else {
        // No matching virtual host, fall back to legacy routes
        return route_request(ctx, legacy_routes, state).await;
    };

    // 2. Get index files (use vhost override or legacy default)
    let index_files = vhost
        .index_files
        .as_ref()
        .unwrap_or(&legacy_routes.index_files);

    // 3. Find matching route within virtual host
    if let Some(route) = routing::match_route(ctx.path, None, &vhost.routes) {
        return dispatch_route_action(ctx, &route.action, ctx.path, index_files).await;
    }

    // 4. No route matched, return 404
    http::build_404_response()
}

/// Route request based on path and configuration (legacy mode)
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

/// Dispatch to xDS `RouteAction`
async fn dispatch_route_action(
    ctx: &RequestContext<'_>,
    action: &RouteAction,
    route_prefix: &str,
    index_files: &[String],
) -> Response<Full<Bytes>> {
    match action {
        RouteAction::Dir { path: dir } => {
            static_files::serve_directory(ctx, dir, route_prefix, index_files).await
        }
        RouteAction::File { path: file_path } => {
            static_files::serve_file(ctx, file_path).await
        }
        RouteAction::Redirect { target, code } => {
            http::build_redirect_response_with_code(target, *code)
        }
        RouteAction::Direct { status, body, content_type } => {
            http::build_direct_response(*status, body.as_deref(), content_type.as_deref())
        }
    }
}

/// Dispatch to specific route handler (legacy mode)
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

    http::response::build_html_response(html, ctx.is_head)
}
