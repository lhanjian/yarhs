// Server module entry point
// Provides server startup, connection handling, and hot restart functionality

pub mod connection;
pub mod listener;
pub mod restart;

// Rust doesn't allow 'loop' as a module name (reserved keyword), renamed to server_loop
#[path = "loop.rs"]
pub mod server_loop;

// Re-export commonly used types
pub use listener::create_reusable_listener;
pub use server_loop::{start_server_loop, ServerLoopConfig};
