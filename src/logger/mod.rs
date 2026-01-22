//! Logger module
//!
//! Provides logging utilities for the HTTP server including:
//! - Server lifecycle logging
//! - Access logging with multiple formats
//! - Error and warning logging

mod format;

pub use format::AccessLogEntry;

use crate::config::Config;
use std::net::SocketAddr;

pub fn log_server_start(addr: &SocketAddr, config: &Config) {
    println!("======================================");
    println!("Async server started successfully");
    println!("Listening on: http://{addr}");
    println!("Log level: {}", config.logging.level);
    if let Some(workers) = config.server.workers {
        println!("Worker threads: {workers}");
    }
    println!("Using Tokio runtime for concurrency");
    println!("======================================\n");
}

pub fn log_connection_accepted(peer_addr: &SocketAddr) {
    println!("[Connection] Accepted from: {peer_addr}");
}

pub fn log_connection_error(err: &impl std::fmt::Debug) {
    eprintln!("[ERROR] Failed to serve connection: {err:?}");
}

pub fn log_error(message: &str) {
    eprintln!("[ERROR] {message}");
}

pub fn log_api_error(message: &str) {
    eprintln!("[API ERROR] {message}");
}

pub fn log_old_listener_error(message: &str) {
    eprintln!("[OLD] {message}");
}

pub fn log_warning(message: &str) {
    eprintln!("[WARN] {message}");
}

pub fn log_headers_count(count: usize, show: bool) {
    if show {
        println!("[Headers] Count: {count}");
    }
}

/// Log formatted access log entry
pub fn log_access(entry: &AccessLogEntry, format: &str) {
    println!("{}", entry.format(format));
}

pub fn log_api_request(method: &str, path: &str, status: u16) {
    println!("[API] {method} {path} - {status}");
}

pub fn log_restart_triggered() {
    println!("\n[Restart] Server restart triggered");
}

pub fn log_binding_new_address(addr: &std::net::SocketAddr) {
    println!("[Step 1] Binding new address: {addr}");
}

pub fn log_new_listener_bound(addr: &std::net::SocketAddr) {
    println!("[Step 1] ✓ New listener bound successfully on {addr}");
}

pub fn log_bind_failed(addr: &std::net::SocketAddr, err: &std::io::Error) {
    log_error(&format!("[Step 1] ✗ Failed to bind {addr}: {err}"));
    eprintln!("         Continuing with current configuration");
}
