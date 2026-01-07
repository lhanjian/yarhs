use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use socket2::{Socket, Domain, Type, Protocol};

mod api;
mod config;
mod handler;
mod logger;
mod response;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::Config::load()?;
    let addr = cfg.get_socket_addr()?;
    let listener = create_reusable_listener(addr)?;
    
    let state = Arc::new(config::AppState::new(&cfg));
    let active_connections = Arc::new(AtomicUsize::new(0));
    
    logger::log_server_start(&addr, &cfg);
    println!("[API] Dynamic configuration endpoint: http://{}/api/config", addr);
    println!("  - GET  /api/config  (view current config)");
    println!("  - PUT  /api/config  (update config)");
    println!("[INFO] Server supports graceful restart for host/port changes\n");

    // Use LocalSet for spawn_local support
    let local = tokio::task::LocalSet::new();
    local.run_until(start_server_loop(listener, state, active_connections)).await
}

async fn start_server_loop(
    mut listener: TcpListener,
    state: Arc<config::AppState>,
    active_connections: Arc<AtomicUsize>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, peer_addr)) => {
                        accept_connection(
                            stream,
                            peer_addr,
                            &state,
                            &active_connections,
                            true,  // check_limits
                            "",    // no prefix
                        );
                    }
                    Err(e) => {
                        eprintln!("[ERROR] Failed to accept connection: {}", e);
                    }
                }
            }
            
            _ = state.restart_signal.notified() => {
                logger::log_restart_triggered();
                
                let new_config = {
                    let config = state.new_server_config.read().await;
                    config.clone().unwrap()
                };
                
                let new_addr = format!("{}:{}", new_config.host, new_config.port)
                    .parse::<std::net::SocketAddr>()?;
                
                // Check if restarting on same address
                let old_addr = listener.local_addr()?;
                let same_addr = old_addr == new_addr;
                    
                // ====== Step 1: Bind new listener ======
                logger::log_binding_new_address(&new_addr);
                let new_listener = match create_reusable_listener(new_addr) {
                    Ok(l) => {
                        logger::log_new_listener_bound(&new_addr);
                        l
                    }
                    Err(e) => {
                        logger::log_bind_failed(&new_addr, &e);
                        {
                            let mut cfg = state.new_server_config.write().await;
                            *cfg = None;
                        }
                        continue;
                    }
                };
                
                // ====== Step 2: Immediately start new loop (zero downtime) ======
                println!("[RESTART] Starting new server loop on {}", new_addr);
                
                if same_addr {
                    println!("[RESTART] Force restart on same address: {}", new_addr);
                    println!("[RESTART] Both old and new listeners will run concurrently for 100ms");
                } else {
                    println!("[RESTART] Switching from {} to {}", old_addr, new_addr);
                    println!("[RESTART] Old listener will drain backlog for 100ms");
                }
                
                let old_listener = listener;
                let old_state = Arc::clone(&state);
                let old_counter = Arc::clone(&active_connections);
                
                // Spawn old loop to drain backlog for 100ms
                tokio::task::spawn_local(async move {
                    drain_old_listener(
                        old_listener,
                        old_state,
                        old_counter,
                    ).await;
                });
                
                // ====== Step 3: Switch to new listener immediately ======
                listener = new_listener;
                
                // Update current server config
                {
                    let mut current = state.current_server_config.write().await;
                    *current = new_config.clone();
                }
                
                println!("======================================");
                println!("Server successfully restarted!");
                println!("Listening on: http://{}", new_addr);
                println!("======================================");
                
                // Continue main loop with new listener
                continue;
            }
        }
    }
}

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
fn accept_connection(
    stream: tokio::net::TcpStream,
    peer_addr: std::net::SocketAddr,
    state: &Arc<config::AppState>,
    conn_counter: &Arc<AtomicUsize>,
    check_limits: bool,
    log_prefix: &str,
) {
    // Increment counter first, then check limit (prevents race condition)
    let prev_count = conn_counter.fetch_add(1, Ordering::SeqCst);
    eprintln!(
        "[DEBUG] prev_count connections: {}",
        prev_count
    );
    
    // Check connection limit if requested
    if check_limits {
        if let Some(max_conn) = state.config.performance.max_connections {
            if prev_count >= max_conn as usize {
                // Exceeded limit: rollback counter and reject
                conn_counter.fetch_sub(1, Ordering::SeqCst);
                eprintln!(
                    "[WARN] Max connections reached: {}/{}. Connection rejected.",
                    prev_count, max_conn
                );
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
            println!("[{}] Accepting connection from {}", log_prefix, peer_addr);
        }
    }
    
    // Handle the connection in a spawned task
    handle_connection(
        stream,
        Arc::clone(state),
        Arc::clone(conn_counter),
    );
}

/// Handle a single connection in a spawned task.
/// 
/// This function:
/// 1. Wraps the TCP stream in TokioIo
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
fn handle_connection(
    stream: tokio::net::TcpStream,
    state: Arc<config::AppState>,
    conn_counter: Arc<AtomicUsize>,
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
            handler::handle_request(req, Arc::clone(&state))
        }));
        
        // Apply timeout and handle result
        match tokio::time::timeout(timeout_duration, conn).await {
            Ok(Ok(_)) => {},
            Ok(Err(err)) => logger::log_connection_error(&err),
            Err(_) => {
                eprintln!(
                    "[WARN] Connection timeout after {} seconds",
                    timeout_duration.as_secs()
                );
            }
        }
        
        // Decrement active connection counter
        conn_counter.fetch_sub(1, Ordering::SeqCst);
    });
}

/// Drain old listener's backlog queue for 100ms then close it.
/// 
/// This function runs in a background task during hot restart to ensure:
/// 1. Connections in the old listener's backlog are not lost
/// 2. Old listener is closed promptly to release resources
/// 
/// # Process
/// 
/// 1. Accept connections from old listener for 100ms
/// 2. Close the old listener immediately
/// 3. Active connections finish gracefully in the background
/// 
/// # Why not wait for connections to close?
/// 
/// - Problem: conn_counter is shared between old and new connections
/// - If we wait for count==0, we'd wait for new connections too
/// - Solution: Let old connections finish naturally in background
/// - 100ms drain period is sufficient for most backlog connections
/// 
/// # Arguments
/// 
/// * `old_listener` - The listener being replaced
/// * `state` - Shared application state
/// * `_conn_counter` - Connection counter (unused, kept for signature compatibility)
async fn drain_old_listener(
    old_listener: TcpListener,
    state: Arc<config::AppState>,
    conn_counter: Arc<AtomicUsize>,
) {
    println!("[RESTART] Old loop draining backlog for 100ms...");
    
    let drain_deadline = tokio::time::Instant::now() 
        + std::time::Duration::from_millis(100);
    
    // Accept connections from old listener for 100ms
    loop {
        tokio::select! {
            accept_result = old_listener.accept() => {
                match accept_result {
                    Ok((stream, peer_addr)) => {
                        accept_connection(
                            stream,
                            peer_addr,
                            &state,
                            &conn_counter,
                            false,  // don't check limits for backlog connections
                            "OLD",  // log prefix
                        );
                    }
                    Err(e) => {
                        eprintln!("[OLD] Accept error: {}", e);
                        break;
                    }
                }
            }
            
            _ = tokio::time::sleep_until(drain_deadline) => {
                println!("[RESTART] Backlog drain completed (100ms elapsed)");
                break;
            }
        }
    }
    
    // Close old listener immediately
    // Note: Active connections will continue processing in background tasks
    println!("[RESTART] Closing old listener (active connections will finish naturally)");
    drop(old_listener);
    println!("[RESTART] ✓ Old listener closed and resources released");
}

/// Create a TcpListener with SO_REUSEPORT and SO_REUSEADDR enabled.
/// 
/// This allows multiple sockets to bind to the same address:port combination,
/// enabling zero-downtime restarts by binding a new listener before closing the old one.
/// 
/// # Arguments
/// 
/// * `addr` - The socket address to bind to
/// 
/// # Returns
/// 
/// * `Ok(TcpListener)` - Successfully created and bound listener
/// * `Err(std::io::Error)` - Failed to create or bind socket
fn create_reusable_listener(addr: std::net::SocketAddr) -> std::io::Result<TcpListener> {
    // Create socket with appropriate domain (IPv4 or IPv6)
    let domain = if addr.is_ipv4() { 
        Domain::IPV4 
    } else { 
        Domain::IPV6 
    };
    
    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;
    
    // Enable SO_REUSEPORT: allows multiple sockets to bind to the same port
    // This is the key feature for zero-downtime restarts
    socket.set_reuse_port(true)?;
    
    // Enable SO_REUSEADDR: allows binding to a port in TIME_WAIT state
    socket.set_reuse_address(true)?;
    
    // Set non-blocking mode for async compatibility
    socket.set_nonblocking(true)?;
    
    // Bind to the specified address
    socket.bind(&addr.into())?;
    
    // Start listening with a backlog queue size of 128
    socket.listen(128)?;
    
    // Convert socket2::Socket to std::net::TcpListener, then to tokio::net::TcpListener
    let std_listener: std::net::TcpListener = socket.into();
    TcpListener::from_std(std_listener)
}
