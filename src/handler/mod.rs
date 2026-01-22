//! Request handler module
//!
//! Responsible for request routing dispatch and business logic processing.
//! Currently supports static file serving, with future extensibility for reverse proxy and other features.

pub mod router;
pub mod static_files;

// Re-export main entry point
pub use router::handle_request;
