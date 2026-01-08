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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::Config::load()?;
    
    // 创建 Tokio 运行时，根据 workers 配置设置线程数
    let mut runtime_builder = tokio::runtime::Builder::new_multi_thread();
    runtime_builder.enable_all();
    
    if let Some(workers) = cfg.server.workers {
        runtime_builder.worker_threads(workers);
        println!("[CONFIG] Using {} worker threads", workers);
    } else {
        println!("[CONFIG] Using default worker threads (CPU cores)");
    }
    
    let runtime = runtime_builder.build()?;
    
    runtime.block_on(async_main(cfg))
}

async fn async_main(cfg: config::Config) -> Result<(), Box<dyn std::error::Error>> {
    let app_addr = cfg.get_socket_addr()?;
    let api_addr = cfg.get_api_socket_addr()?;
    
    let app_listener = create_reusable_listener(app_addr)?;
    let api_listener = create_reusable_listener(api_addr)?;
    
    let state = Arc::new(config::AppState::new(&cfg));
    let app_connections = Arc::new(AtomicUsize::new(0));
    let api_connections = Arc::new(AtomicUsize::new(0));
    
    logger::log_server_start(&app_addr, &cfg);
    println!("[API] Management API running on: http://{}", api_addr);
    println!("  - GET  http://{}/api/config  (view current config)", api_addr);
    println!("  - PUT  http://{}/api/config  (update config)", api_addr);
    println!("[INFO] Application and API ports are separated");
    println!("[INFO] Server supports graceful restart for host/port changes");
    println!("[CONFIG] Loaded configuration:");
    println!("  - Main server: {}:{}", cfg.server.host, cfg.server.port);
    println!("  - API server: {}:{}", cfg.server.api_host, cfg.server.api_port);
    println!("  - Max body size: {} bytes", cfg.http.max_body_size);
    println!("  - Max connections: {:?}\n", cfg.performance.max_connections);

    // Use LocalSet for spawn_local support
    let local = tokio::task::LocalSet::new();
    local.run_until(run_dual_servers(
        app_listener,
        api_listener,
        state,
        app_connections,
        api_connections,
    )).await
}

async fn run_dual_servers(
    app_listener: TcpListener,
    api_listener: TcpListener,
    state: Arc<config::AppState>,
    app_connections: Arc<AtomicUsize>,
    api_connections: Arc<AtomicUsize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let state_clone = state.clone();
    let api_connections_clone = api_connections.clone();
    
    // Spawn API server task
    tokio::task::spawn_local(async move {
        if let Err(e) = run_api_server(api_listener, state_clone, api_connections_clone).await {
            eprintln!("[API ERROR] API server error: {}", e);
        }
    });
    
    // Run app server in main task
    let restart_signal = Arc::clone(&state.restart_signal);
    start_server_loop(
        app_listener,
        state,
        app_connections,
        false, // is_api_server
        true,  // check_connection_limits
        restart_signal,
        |config| format!("{}:{}", config.host, config.port),
        "",    // no log prefix
    ).await
}

async fn run_api_server(
    listener: TcpListener,
    state: Arc<config::AppState>,
    active_connections: Arc<AtomicUsize>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("[API] API server listening...");
    let api_restart_signal = Arc::clone(&state.api_restart_signal);
    start_server_loop(
        listener,
        state,
        active_connections,
        true,   // is_api_server
        false,  // check_connection_limits
        api_restart_signal,
        |config| format!("{}:{}", config.api_host, config.api_port),
        "[API]",
    ).await
}

/// Unified server loop that handles both main and API servers
/// 
/// This function consolidates the common logic between main server and API server loops,
/// reducing code duplication and improving maintainability.
async fn start_server_loop(
    mut listener: TcpListener,
    state: Arc<config::AppState>,
    active_connections: Arc<AtomicUsize>,
    is_api_server: bool,
    check_connection_limits: bool,
    restart_signal: Arc<tokio::sync::Notify>,
    get_new_addr: impl Fn(&config::DynamicServerConfig) -> String,
    log_prefix: &str,
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
                            check_connection_limits,
                            if log_prefix.is_empty() { "" } else { log_prefix },
                            is_api_server,
                        );
                    }
                    Err(e) => {
                        eprintln!("[{}ERROR] Failed to accept connection: {}", 
                                 if log_prefix.is_empty() { "" } else { log_prefix },
                                 e);
                    }
                }
            }
            
            _ = restart_signal.notified() => {
                if !is_api_server {
                    logger::log_restart_triggered();
                } else {
                    println!("[API RESTART] ========== API Restart Signal Received ==========");
                }
                
                let new_config = {
                    let config = state.new_server_config.read().await;
                    match config.as_ref() {
                        Some(c) => c.clone(),
                        None => {
                            eprintln!("[ERROR] No new config available for restart");
                            continue;
                        }
                    }
                };
                
                let old_addr = listener.local_addr()?;
                let new_addr_str = get_new_addr(&new_config);
                let new_addr = new_addr_str.parse::<std::net::SocketAddr>()?;
                
                if is_api_server {
                    println!("[API RESTART] Current address: {}", old_addr);
                    println!("[API RESTART] New address: {}", new_addr);
                } else {
                    logger::log_binding_new_address(&new_addr);
                }
                
                let same_addr = old_addr == new_addr;
                    
                // Bind new listener
                let new_listener = match create_reusable_listener(new_addr) {
                    Ok(l) => {
                        if is_api_server {
                            println!("[API RESTART] ✓ New listener successfully bound on {}", new_addr);
                        } else {
                            logger::log_new_listener_bound(&new_addr);
                        }
                        l
                    }
                    Err(e) => {
                        if is_api_server {
                            eprintln!("[API ERROR] ✗ Failed to bind {}: {}", new_addr, e);
                            eprintln!("[API ERROR] API server will continue on old address: {}", old_addr);
                        } else {
                            logger::log_bind_failed(&new_addr, &e);
                            let mut cfg = state.new_server_config.write().await;
                            *cfg = None;
                        }
                        continue;
                    }
                };
                
                // Log restart info
                if is_api_server {
                    println!("[API RESTART] Starting new server loop on {}", new_addr);
                    if same_addr {
                        println!("[API RESTART] Force restart on same address: {}", new_addr);
                    } else {
                        println!("[API RESTART] Switching from {} to {}", old_addr, new_addr);
                    }
                } else {
                    println!("[RESTART] Starting new server loop on {}", new_addr);
                    if same_addr {
                        println!("[RESTART] Force restart on same address: {}", new_addr);
                        println!("[RESTART] Both old and new listeners will run concurrently for 100ms");
                    } else {
                        println!("[RESTART] Switching from {} to {}", old_addr, new_addr);
                        println!("[RESTART] Old listener will drain backlog for 100ms");
                    }
                }
                
                // Drain old listener
                let old_listener = listener;
                let old_state = Arc::clone(&state);
                let old_counter = Arc::clone(&active_connections);
                
                tokio::task::spawn_local(async move {
                    drain_old_listener(old_listener, old_state, old_counter).await;
                });
                
                // Switch to new listener
                listener = new_listener;
                
                // Log success
                if is_api_server {
                    println!("[API RESTART] ✓ Listener switched successfully");
                    println!("[API RESTART] ========== API Server Now Running on {} ==========", new_addr);
                    println!("[API RESTART] Old address {} is being drained and will close soon\n", old_addr);
                } else {
                    println!("======================================");
                    println!("Server successfully restarted!");
                    println!("Listening on: http://{}", new_addr);
                    println!("======================================");
                }
                
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
/// * `is_api_server` - Whether this is the API management server
fn accept_connection(
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
        is_api_server,
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
            Ok(Ok(_)) => {},
            Ok(Err(err)) => logger::log_connection_error(&err),
            Err(_) => {
                eprintln!(
                    "[WARN] Connection timeout after {} seconds, api_server: {}",
                    timeout_duration.as_secs(),
                    is_api_server
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
/// * `conn_counter` - Connection counter
async fn drain_old_listener(
    old_listener: TcpListener,
    state: Arc<config::AppState>,
    conn_counter: Arc<AtomicUsize>,
) {
    println!("[RESTART] Old loop draining backlog for 100ms...");
    
    let drain_deadline = tokio::time::Instant::now() 
        + std::time::Duration::from_millis(100);
    
    let mut iteration = 0;
    
    // Accept connections from old listener for 100ms
    loop {
        iteration += 1;
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
                            false,  // is_api_server
                        );
                    }
                    Err(e) => {
                        eprintln!("[OLD] Accept error: {}", e);
                        break;
                    }
                }
            }
            
            _ = tokio::time::sleep_until(drain_deadline) => {
                println!("[RESTART] Backlog drain completed (100ms elapsed, {} iterations)", iteration);
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
