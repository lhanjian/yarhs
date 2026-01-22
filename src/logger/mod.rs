//! Logger module
//!
//! Provides logging utilities for the HTTP server including:
//! - Server lifecycle logging
//! - Access logging with multiple formats
//! - Error and warning logging
//! - File-based logging support

mod format;
pub mod writer;

pub use format::AccessLogEntry;

use crate::config::Config;
use std::net::SocketAddr;

/// Initialize the logger with configuration
///
/// Should be called once at application startup.
pub fn init(config: &Config) -> std::io::Result<()> {
    writer::init(
        config.logging.access_log_file.as_deref(),
        config.logging.error_log_file.as_deref(),
    )
}

/// Write to info/access log
fn write_info(message: &str) {
    if writer::is_initialized() {
        writer::get().write_info(message);
    } else {
        println!("{message}");
    }
}

/// Write to error log
fn write_error(message: &str) {
    if writer::is_initialized() {
        writer::get().write_error(message);
    } else {
        eprintln!("{message}");
    }
}

/// Write to access log specifically
fn write_access(message: &str) {
    if writer::is_initialized() {
        writer::get().write_access(message);
    } else {
        println!("{message}");
    }
}

pub fn log_server_start(addr: &SocketAddr, config: &Config) {
    write_info("======================================");
    write_info("Async server started successfully");
    write_info(&format!("Listening on: http://{addr}"));
    write_info(&format!("Log level: {}", config.logging.level));
    if let Some(workers) = config.server.workers {
        write_info(&format!("Worker threads: {workers}"));
    }
    if let Some(ref path) = config.logging.access_log_file {
        write_info(&format!("Access log: {path}"));
    }
    if let Some(ref path) = config.logging.error_log_file {
        write_info(&format!("Error log: {path}"));
    }
    write_info("Using Tokio runtime for concurrency");
    write_info("======================================\n");
}

pub fn log_connection_accepted(peer_addr: &SocketAddr) {
    write_info(&format!("[Connection] Accepted from: {peer_addr}"));
}

pub fn log_connection_error(err: &impl std::fmt::Debug) {
    write_error(&format!("[ERROR] Failed to serve connection: {err:?}"));
}

pub fn log_error(message: &str) {
    write_error(&format!("[ERROR] {message}"));
}

pub fn log_api_error(message: &str) {
    write_error(&format!("[API ERROR] {message}"));
}

pub fn log_old_listener_error(message: &str) {
    write_error(&format!("[OLD] {message}"));
}

pub fn log_warning(message: &str) {
    write_error(&format!("[WARN] {message}"));
}

pub fn log_headers_count(count: usize, show: bool) {
    if show {
        write_info(&format!("[Headers] Count: {count}"));
    }
}

/// Log formatted access log entry
pub fn log_access(entry: &AccessLogEntry, format: &str) {
    write_access(&entry.format(format));
}

pub fn log_api_request(method: &str, path: &str, status: u16) {
    write_info(&format!("[API] {method} {path} - {status}"));
}

pub fn log_restart_triggered() {
    write_info("\n[Restart] Server restart triggered");
}

pub fn log_binding_new_address(addr: &std::net::SocketAddr) {
    write_info(&format!("[Step 1] Binding new address: {addr}"));
}

pub fn log_new_listener_bound(addr: &std::net::SocketAddr) {
    write_info(&format!("[Step 1] ✓ New listener bound successfully on {addr}"));
}

pub fn log_bind_failed(addr: &std::net::SocketAddr, err: &std::io::Error) {
    log_error(&format!("[Step 1] ✗ Failed to bind {addr}: {err}"));
    write_error("         Continuing with current configuration");
}
