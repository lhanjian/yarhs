//! HTTP response building module
//!
//! Provides builders for various HTTP status code responses, decoupled from specific business logic.

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::Response;

/// Build 304 Not Modified response
pub fn build_304_response(etag: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(304)
        .header("ETag", etag)
        .header("Cache-Control", "public, max-age=3600")
        .body(Full::new(Bytes::new()))
        .unwrap_or_else(|e| {
            log_build_error("304", &e);
            Response::new(Full::new(Bytes::new()))
        })
}

/// Build 404 Not Found response
pub fn build_404_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(404)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from("404 Not Found")))
        .unwrap_or_else(|e| {
            log_build_error("404", &e);
            Response::new(Full::new(Bytes::from("404 Not Found")))
        })
}

/// Build 405 Method Not Allowed response
pub fn build_405_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(405)
        .header("Content-Type", "text/plain")
        .header("Allow", "GET, HEAD, OPTIONS")
        .body(Full::new(Bytes::from("405 Method Not Allowed")))
        .unwrap_or_else(|e| {
            log_build_error("405", &e);
            Response::new(Full::new(Bytes::from("405 Method Not Allowed")))
        })
}

/// Build OPTIONS response (preflight request)
pub fn build_options_response(enable_cors: bool) -> Response<Full<Bytes>> {
    let mut builder = Response::builder()
        .status(204)
        .header("Allow", "GET, HEAD, OPTIONS");

    if enable_cors {
        builder = builder
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET, HEAD, OPTIONS")
            .header("Access-Control-Allow-Headers", "Content-Type, Range")
            .header("Access-Control-Max-Age", "86400");
    }

    builder.body(Full::new(Bytes::new())).unwrap_or_else(|e| {
        log_build_error("OPTIONS", &e);
        Response::new(Full::new(Bytes::new()))
    })
}

/// Build 413 Payload Too Large response
pub fn build_413_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(413)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from("413 Payload Too Large")))
        .unwrap_or_else(|e| {
            log_build_error("413", &e);
            Response::new(Full::new(Bytes::from("413 Payload Too Large")))
        })
}

/// Build 416 Range Not Satisfiable response
pub fn build_416_response(file_size: usize) -> Response<Full<Bytes>> {
    Response::builder()
        .status(416)
        .header("Content-Type", "text/plain")
        .header("Content-Range", format!("bytes */{file_size}"))
        .body(Full::new(Bytes::from("Range Not Satisfiable")))
        .unwrap_or_else(|e| {
            log_build_error("416", &e);
            Response::new(Full::new(Bytes::from("Range Not Satisfiable")))
        })
}

/// Build 302 redirect response
pub fn build_redirect_response(target: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(302)
        .header("Location", target)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from("Redirecting...")))
        .unwrap_or_else(|e| {
            log_build_error("302", &e);
            Response::new(Full::new(Bytes::from("Redirecting...")))
        })
}

/// Build generic HTML response
pub fn build_html_response(content: String, is_head: bool) -> Response<Full<Bytes>> {
    let content_length = content.len();
    let body = if is_head {
        Bytes::new()
    } else {
        Bytes::from(content)
    };

    Response::builder()
        .status(200)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Content-Length", content_length)
        .body(Full::new(body))
        .unwrap_or_else(|e| {
            log_build_error("HTML", &e);
            Response::new(Full::new(Bytes::new()))
        })
}

/// Build success response with cache control
pub fn build_cached_response(
    data: Bytes,
    content_type: &str,
    etag: &str,
    is_head: bool,
) -> Response<Full<Bytes>> {
    let content_length = data.len();
    let body = if is_head { Bytes::new() } else { data };

    Response::builder()
        .status(200)
        .header("Content-Type", content_type)
        .header("Content-Length", content_length)
        .header("Accept-Ranges", "bytes")
        .header("ETag", etag)
        .header("Cache-Control", "public, max-age=3600")
        .body(Full::new(body))
        .unwrap_or_else(|e| {
            log_build_error("200", &e);
            Response::new(Full::new(Bytes::new()))
        })
}

/// Build 206 Partial Content response
pub fn build_partial_response(
    data: Bytes,
    content_type: &str,
    etag: &str,
    start: usize,
    end: usize,
    total_size: usize,
    is_head: bool,
) -> Response<Full<Bytes>> {
    let content_length = end - start + 1;
    let body = if is_head { Bytes::new() } else { data };

    Response::builder()
        .status(206)
        .header("Content-Type", content_type)
        .header("Content-Length", content_length)
        .header("Content-Range", format!("bytes {start}-{end}/{total_size}"))
        .header("Accept-Ranges", "bytes")
        .header("ETag", etag)
        .header("Cache-Control", "public, max-age=3600")
        .body(Full::new(body))
        .unwrap_or_else(|e| {
            log_build_error("206", &e);
            Response::new(Full::new(Bytes::new()))
        })
}

/// Log response build error
fn log_build_error(status: &str, error: &hyper::http::Error) {
    crate::logger::log_error(&format!("Failed to build {status} response: {error}"));
}
