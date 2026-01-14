// 连接处理模块
// 处理单个 TCP 连接的接受和服务

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;

use crate::config;
use crate::logger;
use crate::api;
use crate::handler;

/// Accept and process a connection, checking limits and logging.
/// 
/// # Arguments
/// 
/// * `stream` - The TCP stream to handle
/// * `peer_addr` - The peer's socket address
/// * `state` - Shared application state
/// * `conn_counter` - Active connection counter
/// * `check_limits` - Whether to check max connection limits
/// * `log_prefix` - Prefix for log messages (e.g., "OLD" for old listener)
/// * `is_api_server` - Whether this is the API management server
pub fn accept_connection(
    stream: tokio::net::TcpStream,
    peer_addr: std::net::SocketAddr,
    state: &Arc<config::AppState>,
    conn_counter: &Arc<AtomicUsize>,
    check_limits: bool,
    log_prefix: &str,
    is_api_server: bool,
) {
    // Increment counter first, then check limit (prevents race condition)
    let prev_count = conn_counter.fetch_add(1, Ordering::SeqCst);
    
    // Check connection limit if requested
    if check_limits {
        if let Some(max_conn) = state.config.performance.max_connections {
            if prev_count >= usize::try_from(max_conn).unwrap_or(usize::MAX) {
                // Exceeded limit: rollback counter and reject
                conn_counter.fetch_sub(1, Ordering::SeqCst);
                logger::log_warning(&format!(
                    "Max connections reached: {prev_count}/{max_conn}. Connection rejected."
                ));
                drop(stream);
                return;
            }
        }
    }
    
    // Check if access logging is enabled (lock-free)
    let access_log = state.cached_access_log.load(Ordering::Relaxed);
    if access_log {
        if log_prefix.is_empty() {
            logger::log_connection_accepted(&peer_addr);
        } else {
            println!("[{log_prefix}] Accepting connection from {peer_addr}");
        }
    }
    
    // Handle the connection in a spawned task
    handle_connection(
        stream,
        Arc::clone(state),
        Arc::clone(conn_counter),
        is_api_server,
    );
}

/// Handle a single connection in a spawned task.
/// 
/// This function:
/// 1. Wraps the TCP stream in `TokioIo`
/// 2. Configures HTTP/1.1 connection settings (keep-alive, timeouts)
/// 3. Serves the connection with the request handler
/// 4. Applies timeout to the connection
/// 5. Decrements connection counter when done
/// 
/// # Arguments
/// 
/// * `stream` - The TCP stream to handle
/// * `state` - Shared application state
/// * `conn_counter` - Active connection counter to decrement when done
/// * `is_api_server` - Whether this is handling API management requests
fn handle_connection(
    stream: tokio::net::TcpStream,
    state: Arc<config::AppState>,
    conn_counter: Arc<AtomicUsize>,
    is_api_server: bool,
) {
    tokio::task::spawn_local(async move {
        let io = TokioIo::new(stream);
        
        // Read performance configuration
        let keep_alive_timeout = state.config.performance.keep_alive_timeout;
        let timeout_duration = std::time::Duration::from_secs(
            std::cmp::max(
                state.config.performance.read_timeout,
                state.config.performance.write_timeout
            )
        );
        
        // Build HTTP/1 connection with keep-alive support
        let mut builder = http1::Builder::new();
        if keep_alive_timeout > 0 {
            builder.keep_alive(true);
        }
        
        // Serve connection
        let conn = builder.serve_connection(io, service_fn(move |req| {
            let state_clone = Arc::clone(&state);
            async move {
                if is_api_server {
                    // API server只处理API请求
                    api::handle_api_config(req, state_clone).await
                } else {
                    // 应用服务器处理所有非API请求
                    handler::handle_request(req, state_clone).await
                }
            }
        }));
        
        // Apply timeout and handle result
        match tokio::time::timeout(timeout_duration, conn).await {
            Ok(Ok(())) => {},
            Ok(Err(err)) => logger::log_connection_error(&err),
            Err(_) => {
                logger::log_warning(&format!(
                    "Connection timeout after {} seconds, api_server: {}",
                    timeout_duration.as_secs(),
                    is_api_server
                ));
            }
        }
        
        // Decrement active connection counter
        conn_counter.fetch_sub(1, Ordering::SeqCst);
    });
}
