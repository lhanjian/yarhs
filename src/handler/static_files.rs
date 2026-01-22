//! Static file serving module
//!
//! Handles static file loading, MIME type detection, and response building.

use crate::handler::router::RequestContext;
use crate::http::{self, cache, mime, range::RangeParseResult};
use crate::logger;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::Response;
use std::path::Path;
use tokio::fs;

const FAVICON_PATH: &str = "static/favicon.svg";

/// Serve favicon
pub async fn serve_favicon(ctx: &RequestContext<'_>) -> Response<Full<Bytes>> {
    match load_favicon().await {
        Some(data) => {
            if ctx.access_log {
                logger::log_response(data.len());
            }
            build_favicon_response(&data, ctx.if_none_match.as_deref(), ctx.is_head)
        }
        None => http::build_404_response(),
    }
}

/// Serve static files from a directory
pub async fn serve_directory(
    ctx: &RequestContext<'_>,
    dir: &str,
    route_prefix: &str,
    index_files: &[String],
) -> Response<Full<Bytes>> {
    match load_from_directory(dir, ctx.path, route_prefix, index_files).await {
        Some((content, content_type)) => {
            if ctx.access_log {
                logger::log_response(content.len());
            }
            build_static_file_response(
                &content,
                content_type,
                ctx.if_none_match.as_deref(),
                ctx.is_head,
                ctx.range_header.as_deref(),
            )
        }
        None => http::build_404_response(),
    }
}

/// Serve a single file
pub async fn serve_file(ctx: &RequestContext<'_>, file_path: &str) -> Response<Full<Bytes>> {
    match load_single_file(file_path).await {
        Some((content, content_type)) => {
            if ctx.access_log {
                logger::log_response(content.len());
            }
            build_static_file_response(
                &content,
                content_type,
                ctx.if_none_match.as_deref(),
                ctx.is_head,
                ctx.range_header.as_deref(),
            )
        }
        None => http::build_404_response(),
    }
}

/// Load static file from directory with index file support
pub async fn load_from_directory(
    static_dir: &str,
    path: &str,
    route_prefix: &str,
    index_files: &[String],
) -> Option<(Vec<u8>, &'static str)> {
    // Remove leading slash and prevent directory traversal
    let clean_path = path.trim_start_matches('/').replace("..", "");

    // Remove route prefix from path
    let prefix_clean = route_prefix.trim_matches('/');
    let relative_path = if prefix_clean.is_empty() {
        clean_path.as_str()
    } else {
        clean_path
            .strip_prefix(&format!("{prefix_clean}/"))
            .unwrap_or(&clean_path)
    };

    let mut file_path = Path::new(static_dir).join(relative_path);

    // Security: ensure file_path is within static_dir
    let static_dir_canonical = match Path::new(static_dir).canonicalize() {
        Ok(p) => p,
        Err(e) => {
            logger::log_warning(&format!(
                "Static directory not found or inaccessible '{static_dir}': {e}"
            ));
            return None;
        }
    };

    // Check if path is a directory, try index files
    if file_path.is_dir() || relative_path.is_empty() || relative_path.ends_with('/') {
        for index_file in index_files {
            let index_path = file_path.join(index_file);
            if index_path.exists() && index_path.is_file() {
                file_path = index_path;
                break;
            }
        }
    }

    // File not found is common (404), no need to log at warning level
    let Ok(file_path_canonical) = file_path.canonicalize() else {
        return None;
    };
    if !file_path_canonical.starts_with(&static_dir_canonical) {
        logger::log_warning(&format!(
            "Path traversal attempt blocked: {} -> {}",
            path,
            file_path_canonical.display()
        ));
        return None;
    }

    let content = match fs::read(&file_path).await {
        Ok(c) => c,
        Err(e) => {
            logger::log_error(&format!(
                "Failed to read file '{}': {}",
                file_path.display(),
                e
            ));
            return None;
        }
    };

    // Determine content type from extension
    let content_type = mime::get_content_type(file_path.extension().and_then(|e| e.to_str()));

    Some((content, content_type))
}

/// Load a single file
pub async fn load_single_file(file_path: &str) -> Option<(Vec<u8>, &'static str)> {
    let path = Path::new(file_path);
    let content = fs::read(path).await.ok()?;
    let content_type = mime::get_content_type(path.extension().and_then(|e| e.to_str()));
    Some((content, content_type))
}

/// Load favicon
pub async fn load_favicon() -> Option<Vec<u8>> {
    fs::read(FAVICON_PATH).await.ok()
}

/// Get default homepage HTML
#[allow(clippy::too_many_lines)]
pub fn get_default_homepage() -> String {
    String::from(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>YARHS - Rust Web Server</title>
    <link rel="icon" type="image/svg+xml" href="/favicon.svg">
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            line-height: 1.6;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            color: white;
        }
        .container {
            text-align: center;
            padding: 40px;
            background: rgba(255, 255, 255, 0.1);
            border-radius: 20px;
            backdrop-filter: blur(10px);
            box-shadow: 0 8px 32px 0 rgba(31, 38, 135, 0.37);
            border: 1px solid rgba(255, 255, 255, 0.18);
            max-width: 600px;
        }
        h1 {
            font-size: 3em;
            margin-bottom: 20px;
            font-weight: 700;
        }
        .emoji {
            font-size: 4em;
            margin-bottom: 20px;
            animation: bounce 2s infinite;
        }
        @keyframes bounce {
            0%, 100% { transform: translateY(0); }
            50% { transform: translateY(-20px); }
        }
        p {
            font-size: 1.2em;
            margin: 15px 0;
            opacity: 0.9;
        }
        .features {
            margin-top: 30px;
            text-align: left;
            display: inline-block;
        }
        .features li {
            margin: 10px 0;
            list-style: none;
            padding-left: 30px;
            position: relative;
        }
        .features li:before {
            content: "\2713";
            position: absolute;
            left: 0;
            color: #4ade80;
            font-weight: bold;
            font-size: 1.2em;
        }
        .footer {
            margin-top: 30px;
            font-size: 0.9em;
            opacity: 0.7;
        }
        a {
            color: #4ade80;
            text-decoration: none;
            font-weight: 600;
        }
        a:hover {
            text-decoration: underline;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="emoji">🚀</div>
        <h1>YARHS</h1>
        <p><strong>Yet Another Rust HTTP Server</strong></p>
        <p>High-performance asynchronous Web server</p>
        
        <ul class="features">
            <li>Dynamic route configuration</li>
            <li>Zero-downtime hot restart</li>
            <li>High-performance async I/O</li>
            <li>Smart caching system</li>
        </ul>
        
        <div class="footer">
            <p>Powered by <a href="https://www.rust-lang.org/" target="_blank">Rust</a> + <a href="https://tokio.rs/" target="_blank">Tokio</a> + <a href="https://hyper.rs/" target="_blank">Hyper</a></p>
        </div>
    </div>
</body>
</html>"#,
    )
}

/// Build favicon response
fn build_favicon_response(
    data: &[u8],
    if_none_match: Option<&str>,
    is_head: bool,
) -> Response<Full<Bytes>> {
    let etag = cache::generate_etag(data);

    if cache::check_etag_match(if_none_match, &etag) {
        return http::build_304_response(&etag);
    }

    let body = if is_head {
        Bytes::new()
    } else {
        Bytes::from(data.to_owned())
    };

    Response::builder()
        .status(200)
        .header("Content-Type", "image/svg+xml")
        .header("Content-Length", data.len())
        .header("ETag", etag)
        .header("Cache-Control", "public, max-age=86400")
        .body(Full::new(body))
        .unwrap_or_else(|e| {
            logger::log_error(&format!("Failed to build favicon response: {e}"));
            Response::new(Full::new(Bytes::new()))
        })
}

/// Build static file response with `ETag` and Range support
fn build_static_file_response(
    data: &[u8],
    content_type: &str,
    if_none_match: Option<&str>,
    is_head: bool,
    range_header: Option<&str>,
) -> Response<Full<Bytes>> {
    let etag = cache::generate_etag(data);
    let total_size = data.len();

    // Check if client has cached version
    if cache::check_etag_match(if_none_match, &etag) {
        return http::build_304_response(&etag);
    }

    // Check for Range request
    match http::parse_range_header(range_header, total_size) {
        RangeParseResult::Valid(range) => {
            let start = range.start;
            let end = range.end_position(total_size);

            let body = if is_head {
                Bytes::new()
            } else {
                Bytes::from(data[start..=end].to_vec())
            };

            return http::response::build_partial_response(
                body,
                content_type,
                &etag,
                start,
                end,
                total_size,
                is_head,
            );
        }
        RangeParseResult::NotSatisfiable => {
            return http::build_416_response(total_size);
        }
        RangeParseResult::None => {
            // No Range header or malformed, return full content
        }
    }

    // Full response
    let body = if is_head {
        Bytes::new()
    } else {
        Bytes::from(data.to_owned())
    };

    http::response::build_cached_response(body, content_type, &etag, is_head)
}
