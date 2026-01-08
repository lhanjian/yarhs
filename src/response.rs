use http_body_util::Full;
use hyper::body::Bytes;
use hyper::Response;
use tokio::fs;
use std::path::Path;
use std::sync::Arc;
use crate::config::HttpConfig;

const FAVICON_PATH: &str = "static/favicon.svg";

// Serve static files from static_dir
pub async fn load_static_file(static_dir: &str, path: &str, route_prefix: &str) -> Option<(Vec<u8>, &'static str)> {
    // Remove leading slash and prevent directory traversal
    let clean_path = path.trim_start_matches('/').replace("..", "");
    
    // Remove route prefix from path (e.g., "static" from "/static/file.css")
    let relative_path = clean_path.strip_prefix(&format!("{}/", route_prefix.trim_matches('/')))
        .unwrap_or(&clean_path);
    let file_path = Path::new(static_dir).join(relative_path);
    
    // Security: ensure file_path is within static_dir
    let static_dir_canonical = Path::new(static_dir).canonicalize().ok()?;
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
        <p>高性能异步 Web 服务器</p>
        
        <ul class="features">
            <li>动态路由配置</li>
            <li>零停机热重启</li>
            <li>高性能异步 I/O</li>
            <li>智能缓存系统</li>
        </ul>
        
        <div class="footer">
            <p>Powered by <a href="https://www.rust-lang.org/" target="_blank">Rust</a> + <a href="https://tokio.rs/" target="_blank">Tokio</a> + <a href="https://hyper.rs/" target="_blank">Hyper</a></p>
        </div>
    </div>
</body>
</html>"#
    )
}

pub fn build_html_response(html: String, http_config: Arc<HttpConfig>) -> Response<Full<Bytes>> {
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
            crate::logger::log_error(&format!("Failed to build HTML response: {}", e));
            Response::new(Full::new(Bytes::from("Internal Server Error")))
        })
}

pub async fn load_favicon() -> Option<Vec<u8>> {
    fs::read(FAVICON_PATH).await.ok()
}

pub fn build_favicon_response(data: Vec<u8>) -> Response<Full<Bytes>> {
    Response::builder()
        .status(200)
        .header("Content-Type", "image/svg+xml")
        .header("Cache-Control", "public, max-age=86400")
        .body(Full::new(Bytes::from(data)))
        .unwrap_or_else(|e| {
            crate::logger::log_error(&format!("Failed to build favicon response: {}", e));
            Response::new(Full::new(Bytes::new()))
        })
}

pub fn build_404_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(404)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from("Not Found")))
        .unwrap_or_else(|e| {
            crate::logger::log_error(&format!("Failed to build 404 response: {}", e));
            Response::new(Full::new(Bytes::from("Not Found")))
        })
}

pub fn build_413_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(413)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from("Request Entity Too Large")))
        .unwrap_or_else(|e| {
            crate::logger::log_error(&format!("Failed to build 413 response: {}", e));
            Response::new(Full::new(Bytes::from("Request Entity Too Large")))
        })
}

pub fn build_static_file_response(data: Vec<u8>, content_type: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(200)
        .header("Content-Type", content_type)
        .header("Cache-Control", "public, max-age=3600")
        .body(Full::new(Bytes::from(data)))
        .unwrap_or_else(|e| {
            crate::logger::log_error(&format!("Failed to build static file response: {}", e));
            Response::new(Full::new(Bytes::new()))
        })
}

pub fn build_redirect_response(target: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(302)
        .header("Location", target)
        .body(Full::new(Bytes::from("")))
        .unwrap_or_else(|e| {
            crate::logger::log_error(&format!("Failed to build redirect response: {}", e));
            Response::new(Full::new(Bytes::new()))
        })
}
