//! Virtual host matching module
//!
//! Implements domain matching logic for xDS `VirtualHost`.
//! Supports exact match, wildcard prefix (*.), and catch-all (*).

use crate::config::VirtualHost;

/// Resolve the matching virtual host for a given Host header
///
/// Matching priority:
/// 1. Exact domain match ("api.example.com")
/// 2. Wildcard suffix match ("*.example.com")
/// 3. Catch-all ("*")
///
/// Returns None if no virtual host matches.
pub fn resolve_virtual_host<'a>(
    host: &str,
    virtual_hosts: &'a [VirtualHost],
) -> Option<&'a VirtualHost> {
    // Strip port from host if present (e.g., "example.com:8080" -> "example.com")
    let host = host.split(':').next().unwrap_or(host);

    // First pass: look for exact match
    for vhost in virtual_hosts {
        for domain in &vhost.domains {
            if domain == host {
                return Some(vhost);
            }
        }
    }

    // Second pass: look for wildcard match
    for vhost in virtual_hosts {
        for domain in &vhost.domains {
            if domain.starts_with("*.") && match_wildcard_domain(domain, host) {
                return Some(vhost);
            }
        }
    }

    // Third pass: look for catch-all
    virtual_hosts
        .iter()
        .find(|vhost| vhost.domains.iter().any(|d| d == "*"))
}

/// Match a domain against a pattern
///
/// Supports:
/// - Exact match: "api.example.com" matches "api.example.com"
/// - Wildcard prefix: "*.example.com" matches "api.example.com", "www.example.com"
/// - Catch-all: "*" matches any domain
#[allow(dead_code)] // Used in tests and may be useful for external callers
pub fn match_domain(pattern: &str, host: &str) -> bool {
    // Strip port from host
    let host = host.split(':').next().unwrap_or(host);

    if pattern == "*" {
        return true;
    }

    if pattern == host {
        return true;
    }

    if pattern.starts_with("*.") {
        return match_wildcard_domain(pattern, host);
    }

    false
}

/// Match wildcard domain pattern (*.example.com)
fn match_wildcard_domain(pattern: &str, host: &str) -> bool {
    // "*.example.com" should match:
    // - "api.example.com" (one level)
    // - "www.api.example.com" (multiple levels)
    // - "example.com" (the domain itself, without subdomain)

    let suffix = &pattern[1..]; // ".example.com"

    // Check if host ends with the suffix
    if host.ends_with(suffix) {
        return true;
    }

    // Also match the bare domain (e.g., "*.example.com" matches "example.com")
    let bare_domain = &pattern[2..]; // "example.com"
    host == bare_domain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_domain_exact() {
        assert!(match_domain("api.example.com", "api.example.com"));
        assert!(!match_domain("api.example.com", "www.example.com"));
    }

    #[test]
    fn test_match_domain_wildcard() {
        assert!(match_domain("*.example.com", "api.example.com"));
        assert!(match_domain("*.example.com", "www.example.com"));
        assert!(match_domain("*.example.com", "example.com"));
        assert!(!match_domain("*.example.com", "api.other.com"));
    }

    #[test]
    fn test_match_domain_catch_all() {
        assert!(match_domain("*", "anything.com"));
        assert!(match_domain("*", "api.example.com"));
    }

    #[test]
    fn test_match_domain_with_port() {
        assert!(match_domain("example.com", "example.com:8080"));
        assert!(match_domain("*.example.com", "api.example.com:8080"));
    }

    #[test]
    fn test_resolve_virtual_host_priority() {
        let vhosts = vec![
            VirtualHost {
                name: "catch-all".to_string(),
                domains: vec!["*".to_string()],
                routes: vec![],
                index_files: None,
            },
            VirtualHost {
                name: "wildcard".to_string(),
                domains: vec!["*.example.com".to_string()],
                routes: vec![],
                index_files: None,
            },
            VirtualHost {
                name: "exact".to_string(),
                domains: vec!["api.example.com".to_string()],
                routes: vec![],
                index_files: None,
            },
        ];

        // Exact match takes priority
        let result = resolve_virtual_host("api.example.com", &vhosts);
        assert_eq!(result.unwrap().name, "exact");

        // Wildcard matches when no exact
        let result = resolve_virtual_host("www.example.com", &vhosts);
        assert_eq!(result.unwrap().name, "wildcard");

        // Catch-all for unknown hosts
        let result = resolve_virtual_host("other.com", &vhosts);
        assert_eq!(result.unwrap().name, "catch-all");
    }
}
