// Signal handling module (nginx-style)
//
// Supported signals:
// - SIGHUP:  Reload configuration (hot restart)
// - SIGTERM: Graceful shutdown
// - SIGINT:  Graceful shutdown (Ctrl+C)
// - SIGUSR1: Reopen log files (reserved for future use)
// - SIGUSR2: Upgrade executable (reserved for future use)

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;

/// Signal handler state
pub struct SignalHandler {
    /// Shutdown signal (SIGTERM, SIGINT)
    pub shutdown: Arc<Notify>,
    /// Reload signal (SIGHUP)
    pub reload: Arc<Notify>,
    /// Whether shutdown has been requested
    pub shutdown_requested: Arc<AtomicBool>,
}

impl SignalHandler {
    pub fn new() -> Self {
        Self {
            shutdown: Arc::new(Notify::new()),
            reload: Arc::new(Notify::new()),
            shutdown_requested: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Start signal handlers (Unix only)
///
/// This spawns a background task that listens for Unix signals
/// and triggers appropriate actions.
///
/// # Signals
///
/// | Signal  | Action           | Nginx Equivalent |
/// |---------|------------------|------------------|
/// | SIGHUP  | Reload config    | `nginx -s reload`|
/// | SIGTERM | Graceful stop    | `nginx -s stop`  |
/// | SIGINT  | Graceful stop    | Ctrl+C           |
/// | SIGUSR1 | (reserved)       | Reopen logs      |
/// | SIGUSR2 | (reserved)       | Upgrade binary   |
#[cfg(unix)]
pub fn start_signal_handler(
    handler: Arc<SignalHandler>,
    restart_signal: Arc<Notify>,
    api_restart_signal: Arc<Notify>,
) {
    use tokio::signal::unix::{signal, SignalKind};

    tokio::spawn(async move {
        let mut sighup = signal(SignalKind::hangup()).expect("Failed to register SIGHUP handler");
        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
        let mut sigint =
            signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");
        let mut sigusr1 =
            signal(SignalKind::user_defined1()).expect("Failed to register SIGUSR1 handler");
        let mut sigusr2 =
            signal(SignalKind::user_defined2()).expect("Failed to register SIGUSR2 handler");

        println!("[SIGNAL] Signal handlers registered:");
        println!("  - SIGHUP  (kill -HUP <pid>)   : Reload configuration");
        println!("  - SIGTERM (kill <pid>)        : Graceful shutdown");
        println!("  - SIGINT  (Ctrl+C)            : Graceful shutdown");
        println!("  - SIGUSR1 (kill -USR1 <pid>)  : (reserved for log rotation)");
        println!("  - SIGUSR2 (kill -USR2 <pid>)  : (reserved for binary upgrade)");
        println!("[SIGNAL] Process ID: {}", std::process::id());

        loop {
            tokio::select! {
                // SIGHUP: Reload configuration (like nginx -s reload)
                _ = sighup.recv() => {
                    println!("\n[SIGNAL] ========== SIGHUP Received ==========");
                    println!("[SIGNAL] Reloading configuration...");

                    // Trigger both main and API server restart
                    handler.reload.notify_one();
                    restart_signal.notify_one();
                    api_restart_signal.notify_one();

                    println!("[SIGNAL] Reload signal sent to all servers");
                    println!("[SIGNAL] =========================================\n");
                }

                // SIGTERM: Graceful shutdown
                _ = sigterm.recv() => {
                    println!("\n[SIGNAL] ========== SIGTERM Received ==========");
                    println!("[SIGNAL] Initiating graceful shutdown...");
                    handler.shutdown_requested.store(true, Ordering::SeqCst);
                    handler.shutdown.notify_waiters();
                    println!("[SIGNAL] Shutdown signal sent");
                    println!("[SIGNAL] =========================================\n");
                    break;
                }

                // SIGINT: Graceful shutdown (Ctrl+C)
                _ = sigint.recv() => {
                    println!("\n[SIGNAL] ========== SIGINT Received (Ctrl+C) ==========");
                    println!("[SIGNAL] Initiating graceful shutdown...");
                    handler.shutdown_requested.store(true, Ordering::SeqCst);
                    handler.shutdown.notify_waiters();
                    println!("[SIGNAL] Shutdown signal sent");
                    println!("[SIGNAL] ================================================\n");
                    break;
                }

                // SIGUSR1: Reserved for log rotation
                _ = sigusr1.recv() => {
                    println!("\n[SIGNAL] ========== SIGUSR1 Received ==========");
                    println!("[SIGNAL] Log rotation signal (not yet implemented)");
                    println!("[SIGNAL] =========================================\n");
                }

                // SIGUSR2: Reserved for binary upgrade
                _ = sigusr2.recv() => {
                    println!("\n[SIGNAL] ========== SIGUSR2 Received ==========");
                    println!("[SIGNAL] Binary upgrade signal (not yet implemented)");
                    println!("[SIGNAL] To upgrade: deploy new binary, send SIGHUP");
                    println!("[SIGNAL] =========================================\n");
                }
            }
        }
    });
}

/// Windows fallback - only handles Ctrl+C
#[cfg(not(unix))]
pub fn start_signal_handler(
    handler: Arc<SignalHandler>,
    _restart_signal: Arc<Notify>,
    _api_restart_signal: Arc<Notify>,
) {
    tokio::spawn(async move {
        println!("[SIGNAL] Windows mode: Only Ctrl+C is supported");
        println!("[SIGNAL] Use API endpoints for configuration reload");

        if let Ok(()) = tokio::signal::ctrl_c().await {
            println!("\n[SIGNAL] Ctrl+C received, initiating shutdown...");
            handler.shutdown_requested.store(true, Ordering::SeqCst);
            handler.shutdown.notify_waiters();
        }
    });
}
