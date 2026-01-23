//! Route matching module
//!
//! Implements path and header matching for xDS Route.

use crate::config::{HeaderMatcher, Route, RouteMatch};

/// Find the first matching route for a given path and headers
pub fn match_route<'a>(
    path: &str,
    headers: Option<&[(&str, &str)]>,
    routes: &'a [Route],
) -> Option<&'a Route> {
    routes
        .iter()
        .find(|route| matches_route_rule(&route.match_rule, path, headers))
}

/// Check if a path matches a route rule
pub fn match_path(rule: &RouteMatch, path: &str) -> bool {
    // Exact path match takes priority
    if let Some(exact) = &rule.path {
        return path == exact;
    }

    // Prefix match
    if let Some(prefix) = &rule.prefix {
        return path.starts_with(prefix);
    }

    // No path rule means match all
    true
}

/// Check if request matches a route rule (path + headers)
fn matches_route_rule(rule: &RouteMatch, path: &str, headers: Option<&[(&str, &str)]>) -> bool {
    // First check path
    if !match_path(rule, path) {
        return false;
    }

    // Then check headers if specified
    if let Some(header_matchers) = &rule.headers {
        if !match_headers(header_matchers, headers) {
            return false;
        }
    }

    true
}

/// Check if headers match all header matchers
fn match_headers(matchers: &[HeaderMatcher], headers: Option<&[(&str, &str)]>) -> bool {
    let Some(headers) = headers else {
        return matchers.is_empty();
    };

    for matcher in matchers {
        if !match_single_header(matcher, headers) {
            return false;
        }
    }
    true
}

/// Check if a single header matcher is satisfied
fn match_single_header(matcher: &HeaderMatcher, headers: &[(&str, &str)]) -> bool {
    let header_value = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(&matcher.name))
        .map(|(_, value)| *value);

    // Check "present" condition
    if let Some(should_present) = matcher.present {
        let is_present = header_value.is_some();
        if is_present != should_present {
            return false;
        }
        // If we only check presence, we're done
        if matcher.exact.is_none() && matcher.prefix.is_none() {
            return true;
        }
    }

    let Some(value) = header_value else {
        // Header not present but we need to match value
        return false;
    };

    // Check exact match
    if let Some(exact) = &matcher.exact {
        return value == exact;
    }

    // Check prefix match
    if let Some(prefix) = &matcher.prefix {
        return value.starts_with(prefix);
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RouteAction;

    fn make_route(prefix: Option<&str>, path: Option<&str>) -> Route {
        Route {
            name: None,
            match_rule: RouteMatch {
                prefix: prefix.map(String::from),
                path: path.map(String::from),
                headers: None,
            },
            action: RouteAction::Direct {
                status: 200,
                body: None,
                content_type: None,
            },
        }
    }

    #[test]
    fn test_match_path_exact() {
        let rule = RouteMatch {
            path: Some("/about".to_string()),
            prefix: None,
            headers: None,
        };
        assert!(match_path(&rule, "/about"));
        assert!(!match_path(&rule, "/about/"));
        assert!(!match_path(&rule, "/about/team"));
    }

    #[test]
    fn test_match_path_prefix() {
        let rule = RouteMatch {
            path: None,
            prefix: Some("/api".to_string()),
            headers: None,
        };
        assert!(match_path(&rule, "/api"));
        assert!(match_path(&rule, "/api/users"));
        assert!(match_path(&rule, "/api/v1/users"));
        assert!(!match_path(&rule, "/about"));
    }

    #[test]
    fn test_match_path_no_rule() {
        let rule = RouteMatch {
            path: None,
            prefix: None,
            headers: None,
        };
        assert!(match_path(&rule, "/anything"));
    }

    #[test]
    fn test_match_route_order() {
        let routes = vec![
            make_route(Some("/api/v1"), None),
            make_route(Some("/api"), None),
            make_route(None, None), // catch-all
        ];

        // Should match first applicable route in order
        let result = match_route("/api/v1/users", None, &routes);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().match_rule.prefix,
            Some("/api/v1".to_string())
        );

        let result = match_route("/api/v2/users", None, &routes);
        assert!(result.is_some());
        assert_eq!(result.unwrap().match_rule.prefix, Some("/api".to_string()));
    }

    #[test]
    fn test_match_headers() {
        let matchers = vec![HeaderMatcher {
            name: "X-Api-Key".to_string(),
            exact: Some("secret".to_string()),
            prefix: None,
            present: None,
        }];

        let headers = vec![("X-Api-Key", "secret")];
        assert!(match_headers(&matchers, Some(&headers)));

        let headers = vec![("X-Api-Key", "wrong")];
        assert!(!match_headers(&matchers, Some(&headers)));

        assert!(!match_headers(&matchers, None));
    }

    #[test]
    fn test_match_header_present() {
        let matchers = vec![HeaderMatcher {
            name: "Authorization".to_string(),
            exact: None,
            prefix: None,
            present: Some(true),
        }];

        let headers = vec![("Authorization", "Bearer token")];
        assert!(match_headers(&matchers, Some(&headers)));

        let headers: Vec<(&str, &str)> = vec![];
        assert!(!match_headers(&matchers, Some(&headers)));
    }
}
