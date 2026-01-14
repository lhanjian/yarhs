use crate::config::HttpConfig;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::Response;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

const FAVICON_PATH: &str = "static/favicon.svg";

/// Generate `ETag` from content using fast hash
pub fn generate_etag(content: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    let v = hasher.finish();
    format!("\"{v:x}\"")
}

/// Check if client's `If-None-Match` header matches our `ETag`
pub fn check_etag_match(if_none_match: Option<&str>, etag: &str) -> bool {
    if_none_match.is_some_and(|client_etag| {
        // Handle multiple ETags separated by comma
        client_etag
            .split(',')
            .any(|e| e.trim() == etag || e.trim() == "*")
    })
}

/// Build 304 Not Modified response
pub fn build_304_response(etag: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(304)
        .header("ETag", etag)
        .header("Cache-Control", "public, max-age=3600")
        .body(Full::new(Bytes::new()))
        .unwrap_or_else(|e| {
            crate::logger::log_error(&format!("Failed to build 304 response: {e}"));
            Response::new(Full::new(Bytes::new()))
        })
}

// Serve static files from static_dir with index file support
pub async fn load_static_file(
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
    let static_dir_canonical = Path::new(static_dir).canonicalize().ok()?;

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

    let file_path_canonical = file_path.canonicalize().ok()?;
    if !file_path_canonical.starts_with(&static_dir_canonical) {
        return None;
    }

    let content = fs::read(&file_path).await.ok()?;

    // Determine content type from extension
    let content_type = match file_path.extension()?.to_str()? {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    };

    Some((content, content_type))
}

// Load a single file and determine its content type
pub async fn load_single_file(file_path: &str) -> Option<(Vec<u8>, &'static str)> {
    let path = Path::new(file_path);
    let content = fs::read(path).await.ok()?;

    // Determine content type from extension
    let content_type = match path.extension().and_then(|e| e.to_str()) {
        Some("html" | "htm") => "text/html; charset=utf-8",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("txt" | "md") => "text/plain; charset=utf-8",
        Some("xml") => "application/xml",
        Some("pdf") => "application/pdf",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("ico") => "image/x-icon",
        Some("webp") => "image/webp",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        _ => "application/octet-stream",
    };

    Some((content, content_type))
}

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

pub fn build_html_response(html: String, http_config: &Arc<HttpConfig>) -> Response<Full<Bytes>> {
    let mut builder = Response::builder()
        .status(200)
        .header("Content-Type", &http_config.default_content_type)
        .header("Server", &http_config.server_name);

    if http_config.enable_cors {
        builder = builder.header("Access-Control-Allow-Origin", "*");
    }

    builder
        .body(Full::new(Bytes::from(html)))
        .unwrap_or_else(|e| {
            crate::logger::log_error(&format!("Failed to build HTML response: {e}"));
            Response::new(Full::new(Bytes::from("Internal Server Error")))
        })
}

pub async fn load_favicon() -> Option<Vec<u8>> {
    fs::read(FAVICON_PATH).await.ok()
}

/// Build favicon response with `ETag` conditional check
pub fn build_favicon_response(data: Vec<u8>, if_none_match: Option<&str>) -> Response<Full<Bytes>> {
    let etag = generate_etag(&data);

    if check_etag_match(if_none_match, &etag) {
        return build_304_response(&etag);
    }

    Response::builder()
        .status(200)
        .header("Content-Type", "image/svg+xml")
        .header("ETag", etag)
        .header("Cache-Control", "public, max-age=86400")
        .body(Full::new(Bytes::from(data)))
        .unwrap_or_else(|e| {
            crate::logger::log_error(&format!("Failed to build favicon response: {e}"));
            Response::new(Full::new(Bytes::new()))
        })
}

pub fn build_404_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(404)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from("Not Found")))
        .unwrap_or_else(|e| {
            crate::logger::log_error(&format!("Failed to build 404 response: {e}"));
            Response::new(Full::new(Bytes::from("Not Found")))
        })
}

pub fn build_413_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(413)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from("Request Entity Too Large")))
        .unwrap_or_else(|e| {
            crate::logger::log_error(&format!("Failed to build 413 response: {e}"));
            Response::new(Full::new(Bytes::from("Request Entity Too Large")))
        })
}

/// Build static file response with `ETag` support and conditional check
pub fn build_static_file_response(
    data: Vec<u8>,
    content_type: &str,
    if_none_match: Option<&str>,
) -> Response<Full<Bytes>> {
    let etag = generate_etag(&data);

    // Check if client has cached version
    if check_etag_match(if_none_match, &etag) {
        return build_304_response(&etag);
    }

    Response::builder()
        .status(200)
        .header("Content-Type", content_type)
        .header("ETag", etag)
        .header("Cache-Control", "public, max-age=3600")
        .body(Full::new(Bytes::from(data)))
        .unwrap_or_else(|e| {
            crate::logger::log_error(&format!("Failed to build static file response: {e}"));
            Response::new(Full::new(Bytes::new()))
        })
}

pub fn build_redirect_response(target: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(302)
        .header("Location", target)
        .body(Full::new(Bytes::from("")))
        .unwrap_or_else(|e| {
            crate::logger::log_error(&format!("Failed to build redirect response: {e}"));
            Response::new(Full::new(Bytes::new()))
        })
}
