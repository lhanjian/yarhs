use http_body_util::Full;
use hyper::body::Bytes;
use hyper::Response;
use tokio::fs;
use std::path::Path;
use std::sync::Arc;
use pulldown_cmark::{Parser, Options, html};
use crate::config::{HttpConfig, AppState};

const FAVICON_PATH: &str = "static/favicon.svg";
const API_DOC_PATH: &str = "API.md";

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

fn create_fallback_html() -> String {
    String::from(
        r#"<!DOCTYPE html>
<html>
<head><title>Server Running</title></head>
<body><h1>Server is running</h1></body>
</html>"#
    )
}

pub async fn load_and_render_markdown(state: &Arc<AppState>) -> String {
    // Try cache first
    {
        let cache = state.markdown_cache.read().await;
        if let Some(cached) = cache.as_ref() {
            return cached.clone();
        }
    }
    
    // Generate markdown HTML with async I/O
    let html = match fs::read_to_string(API_DOC_PATH).await {
        Ok(markdown_content) => {
            let html_output = render_markdown(&markdown_content);
            
            // Wrap in HTML document with styling
            format!(
                r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>API Documentation</title>
    <link rel="icon" type="image/svg+xml" href="/favicon.svg">
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            line-height: 1.6;
            max-width: 900px;
            margin: 0 auto;
            padding: 20px;
            background: #f5f5f5;
            color: #333;
        }}
        pre {{
            background: #2d2d2d;
            color: #f8f8f2;
            padding: 15px;
            border-radius: 5px;
            overflow-x: auto;
        }}
        code {{
            background: #e8e8e8;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: "Courier New", monospace;
            font-size: 0.9em;
        }}
        pre code {{
            background: transparent;
            padding: 0;
        }}
        h1, h2, h3 {{
            color: #667eea;
            border-bottom: 2px solid #667eea;
            padding-bottom: 5px;
        }}
        h1 {{ font-size: 2em; }}
        h2 {{ font-size: 1.5em; margin-top: 30px; }}
        h3 {{ font-size: 1.2em; }}
        table {{
            width: 100%;
            border-collapse: collapse;
            margin: 20px 0;
            background: white;
        }}
        th, td {{
            border: 1px solid #ddd;
            padding: 12px;
            text-align: left;
        }}
        th {{
            background: #667eea;
            color: white;
            font-weight: bold;
        }}
        tr:nth-child(even) {{
            background: #f9f9f9;
        }}
        a {{
            color: #667eea;
            text-decoration: none;
        }}
        a:hover {{
            text-decoration: underline;
        }}
        blockquote {{
            border-left: 4px solid #667eea;
            margin: 20px 0;
            padding-left: 20px;
            color: #666;
        }}
        hr {{
            border: none;
            border-top: 2px solid #ddd;
            margin: 30px 0;
        }}
    </style>
</head>
<body>
{}
</body>
</html>"#,
                html_output
            )
        }
        Err(_) => {
            eprintln!("[Warning] Failed to load API.md, using fallback");
            create_fallback_html()
        }
    };
    
    // Cache the result
    {
        let mut cache = state.markdown_cache.write().await;
        *cache = Some(html.clone());
    }
    
    html
}

pub fn build_html_response(html: String, http_config: &HttpConfig) -> Response<Full<Bytes>> {
    let mut builder = Response::builder()
        .status(200)
        .header("Content-Type", &http_config.default_content_type)
        .header("Server", &http_config.server_name);
    
    if http_config.enable_cors {
        builder = builder.header("Access-Control-Allow-Origin", "*");
    }
    
    builder
        .body(Full::new(Bytes::from(html)))
        .expect("Failed to build response")
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
        .expect("Failed to build favicon response")
}

pub fn build_404_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(404)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from("Not Found")))
        .expect("Failed to build 404 response")
}

pub fn build_413_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(413)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from("Request Entity Too Large")))
        .expect("Failed to build 413 response")
}

pub fn build_static_file_response(data: Vec<u8>, content_type: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(200)
        .header("Content-Type", content_type)
        .header("Cache-Control", "public, max-age=3600")
        .body(Full::new(Bytes::from(data)))
        .expect("Failed to build static file response")
}

pub fn build_redirect_response(target: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(302)
        .header("Location", target)
        .body(Full::new(Bytes::from("")))
        .expect("Failed to build redirect response")
}

// Make render_markdown public for custom routes
pub fn render_markdown(md_content: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(md_content, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}
