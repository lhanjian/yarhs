use crate::config::{AppState, RouteHandler};
use crate::logger;
use crate::response;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Request, Response};
use std::convert::Infallible;
use std::sync::Arc;

pub async fn handle_request(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let method = req.method();
    let uri = req.uri();
    let version = req.version();
    let path = uri.path();

    // Use cached access_log flag to avoid lock
    let access_log = state
        .cached_access_log
        .load(std::sync::atomic::Ordering::Relaxed);

    if access_log {
        logger::log_request(method, uri, version);
    }

    // Extract If-None-Match header for ETag validation
    let if_none_match = req
        .headers()
        .get("if-none-match")
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);

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
                    logger::log_error(&format!(
                        "Request body too large: {size} bytes (max: {max_body_size})"
                    ));
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
    let response = if routes.favicon_paths.iter().any(|p| path == p) {
        // 1. Favicon routes
        response::load_favicon()
            .await
            .map_or_else(response::build_404_response, |favicon_data| {
                let size = favicon_data.len();
                if access_log {
                    logger::log_response(size);
                }
                response::build_favicon_response(favicon_data, if_none_match.as_deref())
            })
    } else if let Some(handler) = routes.custom_routes.get(path) {
        // 2. Custom routes (exact match)
        handle_custom_route(
            path,
            handler,
            &state,
            access_log,
            path,
            &routes.index_files,
            if_none_match.as_deref(),
        )
        .await
    } else if let Some((prefix, handler)) = routes
        .custom_routes
        .iter()
        .find(|(prefix, _)| path.starts_with(prefix.as_str()))
    {
        // 3. Custom routes (prefix match, e.g., /static/*)
        handle_custom_route(
            path,
            handler,
            &state,
            access_log,
            prefix,
            &routes.index_files,
            if_none_match.as_deref(),
        )
        .await
    } else {
        // 4. Default: homepage
        let http_config = {
            let config = state.dynamic_config.read().await;
            Arc::clone(&config.http)
        };

        let html = response::get_default_homepage();
        let html_len = html.len();
        let resp = response::build_html_response(html, &http_config);

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
    _state: &Arc<AppState>,
    access_log: bool,
    route_prefix: &str,
    index_files: &[String],
    if_none_match: Option<&str>,
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
                response::build_static_file_response(content, content_type, if_none_match)
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
                response::build_static_file_response(content, content_type, if_none_match)
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
