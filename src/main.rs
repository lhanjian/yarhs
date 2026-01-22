use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::net::TcpListener;

mod api;
mod config;
mod handler;
mod http;
mod logger;
mod server;

fn parse_args() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    
    // Check for help first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Usage: {} [OPTIONS]", args[0]);
        println!();
        println!("Options:");
        println!("  -c, --config <PATH>  Path to config file (without .toml extension)");
        println!("                       Default: config");
        println!("  -h, --help           Show this help message");
        std::process::exit(0);
    }
    
    // Look for config option
    for (i, arg) in args.iter().enumerate() {
        if (arg == "--config" || arg == "-c") && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
        if let Some(value) = arg.strip_prefix("--config=") {
            return Some(value.to_string());
        }
        if let Some(value) = arg.strip_prefix("-c=") {
            return Some(value.to_string());
        }
    }
    
    // Check for unknown arguments (skip program name)
    for arg in args.iter().skip(1) {
        if arg.starts_with('-') && arg != "--config" && arg != "-c" {
            eprintln!("Unknown argument: {arg}");
            eprintln!("Use --help for usage information");
            std::process::exit(1);
        }
    }
    
    None
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = parse_args().unwrap_or_else(|| "config".to_string());
    let cfg = config::Config::load_from(&config_path)?;

    // Initialize logger with file configuration
    if let Err(e) = logger::init(&cfg) {
        eprintln!("[ERROR] Failed to initialize logger: {e}");
        return Err(e.into());
    }

    // Create Tokio runtime, set thread count based on workers configuration
    let mut runtime_builder = tokio::runtime::Builder::new_multi_thread();
    runtime_builder.enable_all();

    if let Some(workers) = cfg.server.workers {
        runtime_builder.worker_threads(workers);
        println!("[CONFIG] Using {workers} worker threads");
    } else {
        println!("[CONFIG] Using default worker threads (CPU cores)");
    }

    let runtime = runtime_builder.build()?;

    runtime.block_on(async_main(cfg))
}

// Allow `future_not_send`: This function uses `LocalSet::run_until()` which doesn't
// require Send futures. Clippy warns because the Future holds non-Send types across
// await points, but since we run via `block_on()` (not `spawn()`), Send is not required.
#[allow(clippy::similar_names, clippy::future_not_send)]
async fn async_main(cfg: config::Config) -> Result<(), Box<dyn std::error::Error>> {
    let app_addr = cfg.get_socket_addr()?;
    let api_addr = cfg.get_api_socket_addr()?;

    let app_listener = server::create_reusable_listener(app_addr)?;
    let api_listener = server::create_reusable_listener(api_addr)?;

    let state = Arc::new(config::AppState::new(&cfg));
    let app_connections = Arc::new(AtomicUsize::new(0));
    let api_connections = Arc::new(AtomicUsize::new(0));

    // Initialize signal handler (nginx-style)
    let signal_handler = Arc::new(server::SignalHandler::new());
    server::start_signal_handler(
        Arc::clone(&signal_handler),
        Arc::clone(&state.restart_signal),
        Arc::clone(&state.api_restart_signal),
    );

    logger::log_server_start(&app_addr, &cfg);
    println!("[API] Management API running on: http://{api_addr}");
    println!("  - GET  http://{api_addr}/v1/discovery  (view current snapshot)");
    println!("  - POST http://{api_addr}/v1/discovery:routes  (update routes)");
    println!("[INFO] Application and API ports are separated");
    println!("[INFO] Server supports graceful restart for host/port changes");
    println!("[CONFIG] Loaded configuration:");
    println!("  - Main server: {}:{}", cfg.server.host, cfg.server.port);
    println!(
        "  - API server: {}:{}",
        cfg.server.api_host, cfg.server.api_port
    );
    println!("  - Max body size: {} bytes", cfg.http.max_body_size);
    println!(
        "  - Max connections: {:?}\n",
        cfg.performance.max_connections
    );

    // Use LocalSet for spawn_local support
    let local = tokio::task::LocalSet::new();
    local
        .run_until(run_dual_servers(
            app_listener,
            api_listener,
            state,
            app_connections,
            api_connections,
            signal_handler,
        ))
        .await
}

// Allow `future_not_send`: This function is called within `LocalSet::run_until()`,
// which executes futures on the current thread without requiring Send. The internal
// `spawn_local()` also doesn't require Send. Clippy's warning is a false positive here.
#[allow(clippy::similar_names, clippy::future_not_send)]
async fn run_dual_servers(
    app_listener: TcpListener,
    api_listener: TcpListener,
    state: Arc<config::AppState>,
    app_connections: Arc<AtomicUsize>,
    api_connections: Arc<AtomicUsize>,
    signal_handler: Arc<server::SignalHandler>,
) -> Result<(), Box<dyn std::error::Error>> {
    let state_clone = state.clone();
    let api_connections_clone = api_connections.clone();
    let shutdown_clone = Arc::clone(&signal_handler.shutdown);

    // Spawn API server task
    tokio::task::spawn_local(async move {
        if let Err(e) = run_api_server(api_listener, state_clone, api_connections_clone).await {
            logger::log_api_error(&format!("API server error: {e}"));
        }
    });

    // Run app server in main task with shutdown support
    let restart_signal = Arc::clone(&state.restart_signal);
    let config = server::ServerLoopConfig {
        is_api_server: false,
        check_connection_limits: true,
        restart_signal,
        get_new_addr: |config| format!("{}:{}", config.host, config.port),
        log_prefix: "",
    };

    // Race between server loop and shutdown signal
    tokio::select! {
        result = server::start_server_loop(app_listener, state.clone(), app_connections.clone(), config) => {
            result
        }
        () = shutdown_clone.notified() => {
            println!("[SHUTDOWN] Main server received shutdown signal");
            graceful_shutdown(state, app_connections).await;
            Ok(())
        }
    }
}

/// Graceful shutdown - wait for active connections to complete
async fn graceful_shutdown(state: Arc<config::AppState>, conn_counter: Arc<AtomicUsize>) {
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    println!("[SHUTDOWN] ========== Graceful Shutdown Started ==========");

    let timeout = Duration::from_secs(
        state
            .config
            .performance
            .read_timeout
            .max(state.config.performance.write_timeout),
    );
    let start = std::time::Instant::now();

    // Wait for connections to drain (with timeout)
    loop {
        let active = conn_counter.load(Ordering::Relaxed);
        if active == 0 {
            println!("[SHUTDOWN] All connections closed");
            break;
        }

        if start.elapsed() > timeout {
            println!("[SHUTDOWN] Timeout reached, {active} connections still active");
            println!("[SHUTDOWN] Forcing shutdown...");
            break;
        }

        println!("[SHUTDOWN] Waiting for {active} active connections...");
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("[SHUTDOWN] ========== Server Stopped ==========");
}

async fn run_api_server(
    listener: TcpListener,
    state: Arc<config::AppState>,
    active_connections: Arc<AtomicUsize>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("[API] API server listening...");
    let api_restart_signal = Arc::clone(&state.api_restart_signal);
    let config = server::ServerLoopConfig {
        is_api_server: true,
        check_connection_limits: false,
        restart_signal: api_restart_signal,
        get_new_addr: |config| format!("{}:{}", config.api_host, config.api_port),
        log_prefix: "[API]",
    };
    server::start_server_loop(listener, state, active_connections, config).await
}
