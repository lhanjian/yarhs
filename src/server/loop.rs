// Server loop module
// Unified server main loop, handles connection acceptance and hot restart

use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::net::TcpListener;

use super::connection::accept_connection;
use super::listener::create_reusable_listener;
use super::restart::drain_old_listener;
use crate::config;
use crate::logger;

/// Configuration for server loop behavior
pub struct ServerLoopConfig<F>
where
    F: Fn(&config::DynamicServerConfig) -> String,
{
    pub is_api_server: bool,
    pub check_connection_limits: bool,
    pub restart_signal: Arc<tokio::sync::Notify>,
    pub get_new_addr: F,
    pub log_prefix: &'static str,
}

/// Unified server loop that handles both main and API servers
///
/// This function consolidates the common logic between main server and API server loops,
/// reducing code duplication and improving maintainability.
#[allow(clippy::too_many_lines, clippy::ignored_unit_patterns)]
pub async fn start_server_loop<F>(
    mut listener: TcpListener,
    state: Arc<config::AppState>,
    active_connections: Arc<AtomicUsize>,
    config: ServerLoopConfig<F>,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn(&config::DynamicServerConfig) -> String,
{
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
                            config.check_connection_limits,
                            config.log_prefix,
                            config.is_api_server,
                        );
                    }
                    Err(e) => {
                        if config.is_api_server {
                            logger::log_api_error(&format!("Failed to accept connection: {e}"));
                        } else {
                            logger::log_error(&format!("Failed to accept connection: {e}"));
                        }
                    }
                }
            }

            _ = config.restart_signal.notified() => {
                if config.is_api_server {
                    println!("[API RESTART] ========== API Restart Signal Received ==========");
                } else {
                    logger::log_restart_triggered();
                }

                let new_config = {
                    let config = state.new_server_config.read().await;
                    if let Some(c) = config.as_ref() { c.clone() } else {
                        logger::log_error("No new config available for restart");
                        continue;
                    }
                };

                let old_addr = listener.local_addr()?;
                let new_addr_str = (config.get_new_addr)(&new_config);
                let new_addr = match new_addr_str.parse::<std::net::SocketAddr>() {
                    Ok(addr) => addr,
                    Err(e) => {
                        logger::log_error(&format!(
                            "Invalid server address '{new_addr_str}': {e}"
                        ));
                        continue;
                    }
                };

                if config.is_api_server {
                    println!("[API RESTART] Current address: {old_addr}");
                    println!("[API RESTART] New address: {new_addr}");
                } else {
                    logger::log_binding_new_address(&new_addr);
                }

                let same_addr = old_addr == new_addr;

                // Bind new listener
                let new_listener = match create_reusable_listener(new_addr) {
                    Ok(l) => {
                        if config.is_api_server {
                            println!("[API RESTART] ✓ New listener successfully bound on {new_addr}");
                        } else {
                            logger::log_new_listener_bound(&new_addr);
                        }
                        l
                    }
                    Err(e) => {
                        if config.is_api_server {
                            logger::log_api_error(&format!("✗ Failed to bind {new_addr}: {e}"));
                            logger::log_api_error(&format!("API server will continue on old address: {old_addr}"));
                        } else {
                            logger::log_bind_failed(&new_addr, &e);
                            let mut cfg = state.new_server_config.write().await;
                            *cfg = None;
                        }
                        continue;
                    }
                };

                // Log restart info
                if config.is_api_server {
                    println!("[API RESTART] Starting new server loop on {new_addr}");
                    if same_addr {
                        println!("[API RESTART] Force restart on same address: {new_addr}");
                    } else {
                        println!("[API RESTART] Switching from {old_addr} to {new_addr}");
                    }
                } else {
                    println!("[RESTART] Starting new server loop on {new_addr}");
                    if same_addr {
                        println!("[RESTART] Force restart on same address: {new_addr}");
                        println!("[RESTART] Both old and new listeners will run concurrently for 100ms");
                    } else {
                        println!("[RESTART] Switching from {old_addr} to {new_addr}");
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
                if config.is_api_server {
                    println!("[API RESTART] ✓ Listener switched successfully");
                    println!("[API RESTART] ========== API Server Now Running on {new_addr} ==========");
                    println!("[API RESTART] Old address {old_addr} is being drained and will close soon\n");
                } else {
                    println!("======================================");
                    println!("Server successfully restarted!");
                    println!("Listening on: http://{new_addr}");
                    println!("======================================");
                }
            }
        }
    }
}
