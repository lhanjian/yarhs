//! HTTP cache control module
//!
//! Provides `ETag` generation, `Last-Modified` handling, and conditional request support.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::SystemTime;

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

/// Format `SystemTime` as HTTP date (RFC 7231)
///
/// Example: `"Sun, 06 Nov 1994 08:49:37 GMT"`
pub fn format_http_date(time: SystemTime) -> String {
    let duration = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Calculate date components
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Days since Unix epoch to date (simplified algorithm)
    let (year, month, day, weekday) = days_to_ymd_weekday(days);

    let weekday_name = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][weekday];
    let month_name = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][month - 1];

    format!("{weekday_name}, {day:02} {month_name} {year} {hours:02}:{minutes:02}:{seconds:02} GMT")
}

/// Convert days since Unix epoch to (year, month, day, weekday)
/// Returns (year: u64, month: usize 1-12, day: u64, weekday: usize 0-6)
fn days_to_ymd_weekday(days: u64) -> (u64, usize, u64, usize) {
    // Weekday: Jan 1, 1970 was Thursday (4)
    let weekday = ((days + 4) % 7) as usize;

    // Simplified date calculation
    let mut remaining = days;
    let mut year = 1970;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }

    let days_in_months: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month: usize = 1;
    for days_in_month in days_in_months {
        if remaining < days_in_month {
            break;
        }
        remaining -= days_in_month;
        month += 1;
    }

    let day = remaining + 1;
    (year, month, day, weekday)
}

const fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Parse HTTP date string to `SystemTime`
///
/// Supports RFC 7231 format: `"Sun, 06 Nov 1994 08:49:37 GMT"`
pub fn parse_http_date(date_str: &str) -> Option<SystemTime> {
    // Format: "Sun, 06 Nov 1994 08:49:37 GMT"
    // Note: The comma after weekday makes "Sun," one token
    let parts: Vec<&str> = date_str.split_whitespace().collect();
    if parts.len() != 6 {
        return None;
    }

    // parts[0] = "Sun," (weekday with comma, ignored)
    // parts[1] = "06"
    // parts[2] = "Nov"
    // parts[3] = "1994"
    // parts[4] = "08:49:37"
    // parts[5] = "GMT"
    let day: u64 = parts.get(1)?.parse().ok()?;
    let month = match *parts.get(2)? {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let year: u64 = parts.get(3)?.parse().ok()?;

    let time_parts: Vec<&str> = parts.get(4)?.split(':').collect();
    if time_parts.len() != 3 {
        return None;
    }
    let hours: u64 = time_parts[0].parse().ok()?;
    let minutes: u64 = time_parts[1].parse().ok()?;
    let seconds: u64 = time_parts[2].parse().ok()?;

    // Convert to seconds since epoch
    let mut total_days: u64 = 0;
    for y in 1970..year {
        total_days += if is_leap_year(y) { 366 } else { 365 };
    }

    let days_in_months: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    for days in days_in_months.iter().take(month - 1) {
        total_days += days;
    }
    total_days += day - 1;

    let total_secs = total_days * 86400 + hours * 3600 + minutes * 60 + seconds;
    Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(total_secs))
}

/// Check if file has been modified since client's cached version
///
/// # Arguments
/// * `if_modified_since` - Client-sent If-Modified-Since header
/// * `file_mtime` - File's last modification time
///
/// # Returns
/// Returns true if NOT modified (should return 304), false otherwise
///
/// # Note
/// HTTP dates only have second precision, but file system mtime may have
/// nanosecond precision. We truncate mtime to seconds for proper comparison.
pub fn check_not_modified_since(if_modified_since: Option<&str>, file_mtime: SystemTime) -> bool {
    if_modified_since.is_some_and(|client_date| {
        parse_http_date(client_date).is_some_and(|client_time| {
            // Truncate file mtime to seconds precision for comparison
            // because HTTP date format only supports second-level granularity
            let mtime_secs = file_mtime
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let mtime_truncated = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(mtime_secs);
            mtime_truncated <= client_time
        })
    })
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

    #[test]
    fn test_format_http_date() {
        // Test a known date: Jan 1, 1970 00:00:00 GMT (Unix epoch)
        let epoch = SystemTime::UNIX_EPOCH;
        assert_eq!(format_http_date(epoch), "Thu, 01 Jan 1970 00:00:00 GMT");

        // Test another known date
        let secs = 784_111_777; // Sun, 06 Nov 1994 08:49:37 GMT
        let time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs);
        assert_eq!(format_http_date(time), "Sun, 06 Nov 1994 08:49:37 GMT");
    }

    #[test]
    fn test_parse_http_date() {
        let date_str = "Sun, 06 Nov 1994 08:49:37 GMT";
        let parsed = parse_http_date(date_str).unwrap();
        let expected = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(784_111_777);
        assert_eq!(parsed, expected);

        // Invalid format
        assert!(parse_http_date("invalid").is_none());
        assert!(parse_http_date("Sun 06 Nov 1994").is_none());
    }

    #[test]
    fn test_check_not_modified_since() {
        let file_time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        let older_date = "Thu, 01 Jan 1970 00:00:00 GMT";
        let newer_date = "Tue, 19 Jan 2038 03:14:07 GMT";

        // File is newer than client's cache -> modified
        assert!(!check_not_modified_since(Some(older_date), file_time));
        // File is older than client's cache -> not modified
        assert!(check_not_modified_since(Some(newer_date), file_time));
        // No header -> assume modified
        assert!(!check_not_modified_since(None, file_time));
    }

    #[test]
    fn test_check_not_modified_since_exact_match() {
        // Test exact time match (client sends back the same Last-Modified value)
        let date_str = "Wed, 07 Jan 2026 08:42:17 GMT";
        let client_time = parse_http_date(date_str).unwrap();
        assert!(
            check_not_modified_since(Some(date_str), client_time),
            "Equal times should return 304"
        );
    }

    #[test]
    fn test_check_not_modified_since_nanosecond_precision() {
        // Test nanosecond precision handling:
        // File systems store mtime with nanoseconds, but HTTP dates are second-precision
        let base_time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_767_775_337);
        let file_mtime_with_nanos = base_time + std::time::Duration::from_nanos(500_000_000);
        let http_date = format_http_date(file_mtime_with_nanos);
        assert!(
            check_not_modified_since(Some(&http_date), file_mtime_with_nanos),
            "Should return 304 even with nanosecond precision difference"
        );
    }
}
