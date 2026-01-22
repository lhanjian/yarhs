//! Access log format module
//!
//! Supports multiple log formats:
//! - `combined` (Apache/Nginx combined format)
//! - `common` (Common Log Format - CLF)
//! - `json` (JSON structured logging)
//! - Custom patterns with variables

use chrono::Local;

/// Access log entry containing all request/response information
#[derive(Debug, Clone)]
pub struct AccessLogEntry {
    /// Client IP address
    pub remote_addr: String,
    /// Request timestamp
    pub time: chrono::DateTime<Local>,
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Request URI path
    pub path: String,
    /// Query string (without leading ?)
    pub query: Option<String>,
    /// HTTP version (1.0, 1.1, 2)
    pub http_version: String,
    /// Response status code
    pub status: u16,
    /// Response body size in bytes
    pub body_bytes: usize,
    /// Referer header
    pub referer: Option<String>,
    /// User-Agent header
    pub user_agent: Option<String>,
    /// Request processing time in microseconds
    pub request_time_us: u64,
}

impl AccessLogEntry {
    /// Create a new access log entry with current timestamp
    pub fn new(remote_addr: String, method: String, path: String) -> Self {
        Self {
            remote_addr,
            time: Local::now(),
            method,
            path,
            query: None,
            http_version: "1.1".to_string(),
            status: 200,
            body_bytes: 0,
            referer: None,
            user_agent: None,
            request_time_us: 0,
        }
    }

    /// Format the log entry according to the specified format
    pub fn format(&self, format: &str) -> String {
        match format {
            "combined" => self.format_combined(),
            "common" => self.format_common(),
            "json" => self.format_json(),
            custom => self.format_custom(custom),
        }
    }

    /// Apache/Nginx Combined Log Format
    /// `$remote_addr - - [$time_local] "$request" $status $body_bytes_sent "$http_referer" "$http_user_agent"`
    fn format_combined(&self) -> String {
        format!(
            "{} - - [{}] \"{} {}{} HTTP/{}\" {} {} \"{}\" \"{}\"",
            self.remote_addr,
            self.time.format("%d/%b/%Y:%H:%M:%S %z"),
            self.method,
            self.path,
            self.query
                .as_ref()
                .map(|q| format!("?{q}"))
                .unwrap_or_default(),
            self.http_version,
            self.status,
            self.body_bytes,
            self.referer.as_deref().unwrap_or("-"),
            self.user_agent.as_deref().unwrap_or("-"),
        )
    }

    /// Common Log Format (CLF)
    /// `$remote_addr - - [$time_local] "$request" $status $body_bytes_sent`
    fn format_common(&self) -> String {
        format!(
            "{} - - [{}] \"{} {}{} HTTP/{}\" {} {}",
            self.remote_addr,
            self.time.format("%d/%b/%Y:%H:%M:%S %z"),
            self.method,
            self.path,
            self.query
                .as_ref()
                .map(|q| format!("?{q}"))
                .unwrap_or_default(),
            self.http_version,
            self.status,
            self.body_bytes,
        )
    }

    /// JSON structured log format
    fn format_json(&self) -> String {
        // Manual JSON building to avoid serde dependency for simple case
        let query_json = self
            .query
            .as_ref()
            .map_or_else(|| "null".to_string(), |q| format!("\"{}\"", escape_json(q)));
        let referer_json = self
            .referer
            .as_ref()
            .map_or_else(|| "null".to_string(), |r| format!("\"{}\"", escape_json(r)));
        let user_agent_json = self
            .user_agent
            .as_ref()
            .map_or_else(|| "null".to_string(), |u| format!("\"{}\"", escape_json(u)));

        format!(
            r#"{{"remote_addr":"{}","time":"{}","method":"{}","path":"{}","query":{},"http_version":"{}","status":{},"body_bytes":{},"referer":{},"user_agent":{},"request_time_us":{}}}"#,
            escape_json(&self.remote_addr),
            self.time.to_rfc3339(),
            escape_json(&self.method),
            escape_json(&self.path),
            query_json,
            escape_json(&self.http_version),
            self.status,
            self.body_bytes,
            referer_json,
            user_agent_json,
            self.request_time_us,
        )
    }

    /// Custom format with variable substitution
    ///
    /// Supported variables:
    /// - `$remote_addr` - Client IP address
    /// - `$time_local` - Local time in Common Log Format
    /// - `$time_iso8601` - ISO 8601 timestamp
    /// - `$request` - Full request line ("METHOD /path HTTP/version")
    /// - `$request_method` - HTTP method
    /// - `$request_uri` - Request URI with query string
    /// - `$status` - Response status code
    /// - `$body_bytes_sent` - Response body size
    /// - `$http_referer` - Referer header
    /// - `$http_user_agent` - User-Agent header
    /// - `$request_time` - Request processing time in seconds (3 decimal places)
    fn format_custom(&self, pattern: &str) -> String {
        let mut result = pattern.to_string();

        let request_uri = if let Some(q) = &self.query {
            format!("{}?{}", self.path, q)
        } else {
            self.path.clone()
        };

        let request_line = format!("{} {} HTTP/{}", self.method, request_uri, self.http_version);

        // Order matters: longer variables first to avoid partial replacement
        result = result.replace("$remote_addr", &self.remote_addr);
        result = result.replace(
            "$time_local",
            &self.time.format("%d/%b/%Y:%H:%M:%S %z").to_string(),
        );
        result = result.replace("$time_iso8601", &self.time.to_rfc3339());
        // Order matters: longer variables first to avoid partial replacement
        // $request_time must come before $request
        #[allow(clippy::cast_precision_loss)]
        let request_time = self.request_time_us as f64 / 1_000_000.0;
        result = result.replace("$request_time", &format!("{request_time:.3}"));
        result = result.replace("$request_method", &self.method);
        result = result.replace("$request_uri", &request_uri);
        result = result.replace("$request", &request_line);
        result = result.replace("$status", &self.status.to_string());
        result = result.replace("$body_bytes_sent", &self.body_bytes.to_string());
        result = result.replace("$http_referer", self.referer.as_deref().unwrap_or("-"));
        result = result.replace(
            "$http_user_agent",
            self.user_agent.as_deref().unwrap_or("-"),
        );

        result
    }
}

/// Escape special characters for JSON string
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry() -> AccessLogEntry {
        let mut entry = AccessLogEntry::new(
            "192.168.1.1".to_string(),
            "GET".to_string(),
            "/api/users".to_string(),
        );
        entry.query = Some("page=1".to_string());
        entry.http_version = "1.1".to_string();
        entry.status = 200;
        entry.body_bytes = 1234;
        entry.referer = Some("https://example.com".to_string());
        entry.user_agent = Some("Mozilla/5.0".to_string());
        entry.request_time_us = 1500;
        entry
    }

    #[test]
    fn test_format_combined() {
        let entry = create_test_entry();
        let log = entry.format("combined");
        assert!(log.contains("192.168.1.1"));
        assert!(log.contains("GET /api/users?page=1 HTTP/1.1"));
        assert!(log.contains("200 1234"));
        assert!(log.contains("https://example.com"));
        assert!(log.contains("Mozilla/5.0"));
    }

    #[test]
    fn test_format_common() {
        let entry = create_test_entry();
        let log = entry.format("common");
        assert!(log.contains("192.168.1.1"));
        assert!(log.contains("GET /api/users?page=1 HTTP/1.1"));
        assert!(log.contains("200 1234"));
        // Common format does not include referer/user-agent
        assert!(!log.contains("https://example.com"));
    }

    #[test]
    fn test_format_json() {
        let entry = create_test_entry();
        let log = entry.format("json");
        assert!(log.contains(r#""remote_addr":"192.168.1.1""#));
        assert!(log.contains(r#""method":"GET""#));
        assert!(log.contains(r#""status":200"#));
        assert!(log.contains(r#""body_bytes":1234"#));
    }

    #[test]
    fn test_format_custom() {
        let entry = create_test_entry();
        let log = entry.format("$remote_addr - $status - $request_time");
        println!("Custom format output: {log}");
        assert!(log.contains("192.168.1.1"));
        assert!(log.contains("200"));
        // 1500us = 0.0015s, formatted as 0.002 (3 decimal places)
        assert!(
            log.contains("0.00"),
            "Expected log to contain '0.00', got: {log}"
        );
    }
}
