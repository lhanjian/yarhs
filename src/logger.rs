use std::net::SocketAddr;
use hyper::{Method, Uri, Version};
use crate::config::Config;

pub fn log_server_start(addr: &SocketAddr, config: &Config) {
    println!("======================================");
    println!("Async server started successfully");
    println!("Listening on: http://{}", addr);
    println!("Log level: {}", config.logging.level);
    println!("Template directory: {}", config.resources.template_dir);
    if let Some(workers) = config.server.workers {
        println!("Worker threads: {}", workers);
    }
    println!("Using Tokio runtime for concurrency");
    println!("======================================\n");
}

pub fn log_connection_accepted(peer_addr: &SocketAddr) {
    println!("[Connection] Accepted from: {}", peer_addr);
}

pub fn log_connection_error(err: &impl std::fmt::Debug) {
    eprintln!("[Error] Failed to serve connection: {:?}", err);
}

pub fn log_request(method: &Method, uri: &Uri, version: Version) {
    println!("[Request] {} {} {:?}", method, uri, version);
}

pub fn log_headers_count(count: usize, show: bool) {
    if show {
        println!("[Headers] Count: {}", count);
    }
}

pub fn log_response(size: usize) {
    println!("[Response] Sent 200 OK ({} bytes)\n", size);
}

pub fn log_api_request(method: &str, path: &str, status: u16) {
    println!("[API] {} {} - {}", method, path, status);
}

pub fn log_config_updated() {
    println!("[Config] Dynamic configuration updated");
}

pub fn log_restart_triggered() {
    println!("\n[Restart] Server restart triggered");
}

pub fn log_binding_new_address(addr: &std::net::SocketAddr) {
    println!("[Step 1] Binding new address: {}", addr);
}

pub fn log_new_listener_bound(addr: &std::net::SocketAddr) {
    println!("[Step 1] ✓ New listener bound successfully on {}", addr);
}

pub fn log_bind_failed(addr: &std::net::SocketAddr, err: &std::io::Error) {
    eprintln!("[Step 1] ✗ Failed to bind {}: {}", addr, err);
    eprintln!("         Continuing with current configuration");
}

pub fn log_server_config_change(old: &crate::config::DynamicServerConfig, new: &crate::config::DynamicServerConfig) {
    println!("[Config] Server configuration change detected:");
    println!("  Old: {}:{}", old.host, old.port);
    println!("  New: {}:{}", new.host, new.port);
}
