//! HTTP cache control module
//!
//! Provides `ETag` generation and conditional request handling.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Generate `ETag` using fast hashing
///
/// # Arguments
/// * `content` - File content
///
/// # Returns
/// Quoted `ETag` string, e.g., `"abc123def"`
pub fn generate_etag(content: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    let v = hasher.finish();
    format!("\"{v:x}\"")
}

/// Check if client's `If-None-Match` header matches the server's `ETag`
///
/// Supports:
/// - Single `ETag`: `"abc123"`
/// - Multiple `ETags`: `"abc123", "def456"`
/// - Wildcard: `*`
///
/// # Arguments
/// * `if_none_match` - Client-sent If-None-Match header
/// * `etag` - Server-computed `ETag`
///
/// # Returns
/// Returns true if matched (should return 304), false otherwise
pub fn check_etag_match(if_none_match: Option<&str>, etag: &str) -> bool {
    if_none_match.is_some_and(|client_etag| {
        // Handle multiple ETags separated by comma
        client_etag
            .split(',')
            .any(|e| e.trim() == etag || e.trim() == "*")
    })
}

// TODO: When implementing reverse proxy, use CachePolicy to support different cache policies per route
/// Cache control policy (reserved for future extension)
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum CachePolicy {
    /// Public cache with specified max-age (seconds)
    Public(u32),
    /// Private cache (browser cache only)
    Private(u32),
    /// No cache
    NoCache,
    /// No store
    NoStore,
}

impl CachePolicy {
    /// Convert to Cache-Control header value
    #[allow(dead_code)]
    pub fn to_header_value(self) -> String {
        match self {
            Self::Public(max_age) => format!("public, max-age={max_age}"),
            Self::Private(max_age) => format!("private, max-age={max_age}"),
            Self::NoCache => "no-cache".to_string(),
            Self::NoStore => "no-store".to_string(),
        }
    }
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self::Public(3600) // 1 hour
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_etag() {
        let etag = generate_etag(b"hello world");
        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));
        assert!(etag.len() > 2);
    }

    #[test]
    fn test_etag_consistency() {
        let etag1 = generate_etag(b"same content");
        let etag2 = generate_etag(b"same content");
        assert_eq!(etag1, etag2);
    }

    #[test]
    fn test_etag_difference() {
        let etag1 = generate_etag(b"content a");
        let etag2 = generate_etag(b"content b");
        assert_ne!(etag1, etag2);
    }

    #[test]
    fn test_check_etag_match() {
        let etag = "\"abc123\"";
        assert!(check_etag_match(Some("\"abc123\""), etag));
        assert!(check_etag_match(Some("\"xyz\", \"abc123\""), etag));
        assert!(check_etag_match(Some("*"), etag));
        assert!(!check_etag_match(Some("\"different\""), etag));
        assert!(!check_etag_match(None, etag));
    }

    #[test]
    fn test_cache_policy() {
        assert_eq!(
            CachePolicy::Public(3600).to_header_value(),
            "public, max-age=3600"
        );
        assert_eq!(
            CachePolicy::Private(600).to_header_value(),
            "private, max-age=600"
        );
        assert_eq!(CachePolicy::NoCache.to_header_value(), "no-cache");
        assert_eq!(CachePolicy::NoStore.to_header_value(), "no-store");
    }
}
