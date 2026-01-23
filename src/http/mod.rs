//! HTTP protocol layer module
//!
//! Provides HTTP protocol-related base functionality, decoupled from specific business logic.
//! Can be shared between static file serving and reverse proxy in the future.

pub mod cache;
pub mod mime;
pub mod range;
pub mod response;

// Re-export commonly used types
pub use range::parse_range_header;
pub use response::{
    build_304_response, build_404_response, build_405_response, build_413_response,
    build_416_response, build_direct_response, build_health_response, build_options_response,
    build_redirect_response, build_redirect_response_with_code,
};
