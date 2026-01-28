// API Dashboard - Web UI for configuration management

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};

/// Serve the dashboard HTML page
pub fn serve_dashboard() -> Response<Full<Bytes>> {
    let html = include_str!("dashboard.html");

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-cache")
        .body(Full::new(Bytes::from(html.to_string())))
        .unwrap()
}
