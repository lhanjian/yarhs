// API response utility functions module

use crate::logger;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};
use serde::Serialize;
use std::convert::Infallible;

/// Build JSON response
#[allow(clippy::unnecessary_wraps)]
pub fn json_response<T: Serialize>(
    status: StatusCode,
    body: &T,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let json = match serde_json::to_string_pretty(body) {
        Ok(j) => j,
        Err(e) => {
            logger::log_error(&format!("Failed to serialize response: {e}"));
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error":"Internal server error"}"#,
                )))
                .unwrap_or_else(|_| Response::new(Full::new(Bytes::from("Error")))));
        }
    };

    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap_or_else(|e| {
            logger::log_error(&format!("Failed to build response: {e}"));
            Response::new(Full::new(Bytes::from("Error")))
        }))
}

/// 404 Not Found response
pub fn not_found() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(r#"{"error":"Not Found","available_endpoints":["/v1/discovery","/v1/discovery:listeners","/v1/discovery:routes","/v1/discovery:http","/v1/discovery:logging","/v1/discovery:performance"]}"#)))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::from("Not Found"))))
}

/// 400 Bad Request response
pub fn bad_request(message: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "status": "NACK",
        "error_detail": {
            "code": 400,
            "message": message
        }
    });
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::from("Bad Request"))))
}

/// 409 Conflict response
pub fn conflict_response(message: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "status": "NACK",
        "error_detail": {
            "code": 409,
            "message": message
        }
    });
    Response::builder()
        .status(StatusCode::CONFLICT)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::from("Conflict"))))
}
