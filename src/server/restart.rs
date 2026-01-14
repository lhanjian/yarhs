// Hot restart module
// Handles zero-downtime restart and old connection draining

use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::net::TcpListener;

use super::connection::accept_connection;
use crate::config;
use crate::logger;

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
/// - Problem: `conn_counter` is shared between old and new connections
/// - If we wait for count==0, we'd wait for new connections too
/// - Solution: Let old connections finish naturally in background
/// - 100ms drain period is sufficient for most backlog connections
///
/// # Arguments
///
/// * `old_listener` - The listener being replaced
/// * `state` - Shared application state
/// * `conn_counter` - Connection counter
pub async fn drain_old_listener(
    old_listener: TcpListener,
    state: Arc<config::AppState>,
    conn_counter: Arc<AtomicUsize>,
) {
    println!("[RESTART] Old loop draining backlog for 100ms...");

    let drain_deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(100);

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
                        logger::log_old_listener_error(&format!("Accept error: {e}"));
                        break;
                    }
                }
            }

            () = tokio::time::sleep_until(drain_deadline) => {
                println!("[RESTART] Backlog drain completed (100ms elapsed, {iteration} iterations)");
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
