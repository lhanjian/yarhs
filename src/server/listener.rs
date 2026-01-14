// Reusable listener module
// Creates TCP listeners with SO_REUSEPORT support for zero-downtime restarts

use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::TcpListener;

/// Create a `TcpListener` with `SO_REUSEPORT` and `SO_REUSEADDR` enabled.
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
pub fn create_reusable_listener(addr: std::net::SocketAddr) -> std::io::Result<TcpListener> {
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
