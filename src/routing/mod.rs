//! Routing module
//!
//! Provides xDS-compatible routing capabilities including:
//! - Virtual host matching based on Host header
//! - Route matching based on path prefix/exact match
//! - Header-based routing

mod matcher;
mod vhost;

pub use matcher::match_route;
pub use vhost::resolve_virtual_host;
