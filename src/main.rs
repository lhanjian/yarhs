use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::net::TcpListener;

mod api;
mod config;
mod handler;
mod logger;
mod response;
mod server;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::Config::load()?;

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

#[allow(clippy::similar_names, clippy::future_not_send)]
async fn async_main(cfg: config::Config) -> Result<(), Box<dyn std::error::Error>> {
    let app_addr = cfg.get_socket_addr()?;
    let api_addr = cfg.get_api_socket_addr()?;

    let app_listener = server::create_reusable_listener(app_addr)?;
    let api_listener = server::create_reusable_listener(api_addr)?;

    let state = Arc::new(config::AppState::new(&cfg));
    let app_connections = Arc::new(AtomicUsize::new(0));
    let api_connections = Arc::new(AtomicUsize::new(0));

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
        ))
        .await
}

#[allow(clippy::similar_names)]
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
            logger::log_api_error(&format!("API server error: {e}"));
        }
    });

    // Run app server in main task
    let restart_signal = Arc::clone(&state.restart_signal);
    let config = server::ServerLoopConfig {
        is_api_server: false,
        check_connection_limits: true,
        restart_signal,
        get_new_addr: |config| format!("{}:{}", config.host, config.port),
        log_prefix: "",
    };
    server::start_server_loop(app_listener, state, app_connections, config).await
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
