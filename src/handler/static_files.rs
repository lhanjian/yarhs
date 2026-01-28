//! Static file serving module
//!
//! Handles static file loading, MIME type detection, and response building.
//! Implements the "mtime-first" optimization for conditional requests.

use crate::handler::router::RequestContext;
use crate::http::{self, cache, mime, range::RangeParseResult};
use crate::logger;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::Response;
use std::path::Path;
use std::time::SystemTime;
use tokio::fs;

/// Serve static files from a directory
///
/// Implements the "mtime-first" optimization:
/// 1. Check file metadata (mtime) first - cheap I/O
/// 2. If If-Modified-Since matches, return 304 without reading file content
/// 3. Only read file content when necessary
pub async fn serve_directory(
    ctx: &RequestContext<'_>,
    dir: &str,
    route_prefix: &str,
    index_files: &[String],
) -> Response<Full<Bytes>> {
    match load_from_directory_optimized(
        dir,
        ctx.path,
        route_prefix,
        index_files,
        ctx.if_modified_since.as_deref(),
        ctx.if_none_match.as_deref(),
        ctx.is_head,
        ctx.range_header.as_deref(),
    )
    .await
    {
        Some(response) => response,
        None => http::build_404_response(),
    }
}

/// Serve a single file
///
/// Implements the "mtime-first" optimization for conditional requests.
pub async fn serve_file(ctx: &RequestContext<'_>, file_path: &str) -> Response<Full<Bytes>> {
    match load_single_file_optimized(
        file_path,
        ctx.if_modified_since.as_deref(),
        ctx.if_none_match.as_deref(),
        ctx.is_head,
        ctx.range_header.as_deref(),
    )
    .await
    {
        Some(response) => response,
        None => http::build_404_response(),
    }
}

/// Optimized directory loading with mtime-first check
///
/// This function checks file modification time before reading content,
/// allowing early 304 responses without file I/O.
#[allow(clippy::too_many_arguments)]
async fn load_from_directory_optimized(
    static_dir: &str,
    path: &str,
    route_prefix: &str,
    index_files: &[String],
    if_modified_since: Option<&str>,
    if_none_match: Option<&str>,
    is_head: bool,
    range_header: Option<&str>,
) -> Option<Response<Full<Bytes>>> {
    // Resolve file path (reuse existing logic)
    let file_path = resolve_file_path(static_dir, path, route_prefix, index_files)?;
    let content_type = mime::get_content_type(file_path.extension().and_then(|e| e.to_str()));

    // Step 1: Get file metadata (cheap I/O - only reads inode)
    let metadata = fs::metadata(&file_path).await.ok()?;
    let mtime = metadata.modified().ok()?;
    let last_modified = cache::format_http_date(mtime);

    // Step 2: Fast path - check If-Modified-Since first
    if cache::check_not_modified_since(if_modified_since, mtime) {
        // File hasn't changed, return 304 without reading content
        // Generate ETag from mtime for consistency
        let etag = format!("\"{}\"", mtime_to_etag(mtime));
        return Some(http::response::build_304_response_with_mtime(&etag, &last_modified));
    }

    // Step 3: Slow path - read file content
    let content = fs::read(&file_path).await.ok()?;

    // Generate content-based ETag for accuracy
    let etag = cache::generate_etag(&content);

    // Check ETag match (client might have used If-None-Match)
    if cache::check_etag_match(if_none_match, &etag) {
        return Some(http::response::build_304_response_with_mtime(&etag, &last_modified));
    }

    // Build full response with Last-Modified header
    Some(build_static_file_response_with_mtime(
        &content,
        content_type,
        &etag,
        &last_modified,
        is_head,
        range_header,
    ))
}

/// Optimized single file loading with mtime-first check
async fn load_single_file_optimized(
    file_path: &str,
    if_modified_since: Option<&str>,
    if_none_match: Option<&str>,
    is_head: bool,
    range_header: Option<&str>,
) -> Option<Response<Full<Bytes>>> {
    let path = Path::new(file_path);
    let content_type = mime::get_content_type(path.extension().and_then(|e| e.to_str()));

    // Step 1: Get file metadata
    let metadata = fs::metadata(path).await.ok()?;
    let mtime = metadata.modified().ok()?;
    let last_modified = cache::format_http_date(mtime);

    // Step 2: Fast path - check If-Modified-Since
    if cache::check_not_modified_since(if_modified_since, mtime) {
        let etag = format!("\"{}\"", mtime_to_etag(mtime));
        return Some(http::response::build_304_response_with_mtime(&etag, &last_modified));
    }

    // Step 3: Slow path - read content
    let content = fs::read(path).await.ok()?;

    let etag = cache::generate_etag(&content);

    if cache::check_etag_match(if_none_match, &etag) {
        return Some(http::response::build_304_response_with_mtime(&etag, &last_modified));
    }

    Some(build_static_file_response_with_mtime(
        &content,
        content_type,
        &etag,
        &last_modified,
        is_head,
        range_header,
    ))
}

/// Resolve file path from request, handling index files
fn resolve_file_path(
    static_dir: &str,
    path: &str,
    route_prefix: &str,
    index_files: &[String],
) -> Option<std::path::PathBuf> {
    let clean_path = path.trim_start_matches('/').replace("..", "");
    let prefix_clean = route_prefix.trim_matches('/');
    let relative_path = if prefix_clean.is_empty() {
        clean_path.as_str()
    } else {
        clean_path
            .strip_prefix(&format!("{prefix_clean}/"))
            .unwrap_or(&clean_path)
    };

    let mut file_path = Path::new(static_dir).join(relative_path);

    let static_dir_canonical = Path::new(static_dir).canonicalize().ok()?;

    if file_path.is_dir() || relative_path.is_empty() || relative_path.ends_with('/') {
        for index_file in index_files {
            let index_path = file_path.join(index_file);
            if index_path.exists() && index_path.is_file() {
                file_path = index_path;
                break;
            }
        }
    }

    let file_path_canonical = file_path.canonicalize().ok()?;
    if !file_path_canonical.starts_with(&static_dir_canonical) {
        logger::log_warning(&format!(
            "Path traversal attempt blocked: {} -> {}",
            path,
            file_path_canonical.display()
        ));
        return None;
    }

    Some(file_path_canonical)
}

/// Convert mtime to a simple `ETag` (for mtime-only 304 responses)
fn mtime_to_etag(mtime: SystemTime) -> String {
    let duration = mtime.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    format!("{:x}", duration.as_secs())
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

/// Build static file response with `Last-Modified` support (optimized path)
fn build_static_file_response_with_mtime(
    data: &[u8],
    content_type: &str,
    etag: &str,
    last_modified: &str,
    is_head: bool,
    range_header: Option<&str>,
) -> Response<Full<Bytes>> {
    let total_size = data.len();

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
                etag,
                Some(last_modified),
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

    http::response::build_cached_response(body, content_type, etag, Some(last_modified), is_head)
}
