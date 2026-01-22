//! HTTP Range request parsing module
//!
//! Range header parsing for resumable downloads, compliant with RFC 7233.

/// Parsed Range request
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeRequest {
    /// Start byte position
    pub start: usize,
    /// End byte position, None means until end of file
    pub end: Option<usize>,
}

impl RangeRequest {
    /// Calculate actual end position (considering file size)
    #[inline]
    pub fn end_position(&self, file_size: usize) -> usize {
        self.end.unwrap_or_else(|| file_size.saturating_sub(1))
    }

    /// Calculate content length (for test validation only)
    #[cfg(test)]
    pub fn content_length(&self, file_size: usize) -> usize {
        let end = self.end_position(file_size);
        end.saturating_sub(self.start) + 1
    }
}

/// Range header parse result
#[derive(Debug)]
pub enum RangeParseResult {
    /// Valid range request
    Valid(RangeRequest),
    /// Range not satisfiable (start >= `file_size`) - should return 416
    NotSatisfiable,
    /// No Range header or malformed (ignore, return full content)
    None,
}

/// Parse HTTP Range header (single range only, bytes unit)
///
/// Supported formats:
/// - `bytes=start-end` - Specific range
/// - `bytes=start-` - From start to end
/// - `bytes=-suffix` - Last suffix bytes
///
/// # Arguments
/// * `range_header` - Value of Range header
/// * `file_size` - Total file size
///
/// # Examples
/// ```
/// use yarhs::http::range::{parse_range_header, RangeParseResult};
///
/// // Fixed range
/// let result = parse_range_header(Some("bytes=0-99"), 1000);
/// assert!(matches!(result, RangeParseResult::Valid(_)));
///
/// // No Range header
/// let result = parse_range_header(None, 1000);
/// assert!(matches!(result, RangeParseResult::None));
/// ```
pub fn parse_range_header(range_header: Option<&str>, file_size: usize) -> RangeParseResult {
    let Some(header) = range_header else {
        return RangeParseResult::None;
    };

    let Some(header) = header.strip_prefix("bytes=") else {
        return RangeParseResult::None; // Not bytes unit, ignore
    };

    // Only support single range (not multi-range)
    if header.contains(',') {
        return RangeParseResult::None;
    }

    let parts: Vec<&str> = header.split('-').collect();
    if parts.len() != 2 {
        return RangeParseResult::None;
    }

    let (start_str, end_str) = (parts[0].trim(), parts[1].trim());

    // Suffix range: "-500" means last 500 bytes
    if start_str.is_empty() {
        return parse_suffix_range(end_str, file_size);
    }

    // Standard range: "start-" or "start-end"
    parse_standard_range(start_str, end_str, file_size)
}

/// Parse suffix range (e.g., "-500")
fn parse_suffix_range(suffix_str: &str, file_size: usize) -> RangeParseResult {
    let Ok(suffix) = suffix_str.parse::<usize>() else {
        return RangeParseResult::None;
    };

    if suffix == 0 {
        return RangeParseResult::NotSatisfiable;
    }

    // Suffix larger than file is valid, just return whole file as range
    let start = file_size.saturating_sub(suffix);
    RangeParseResult::Valid(RangeRequest {
        start,
        end: Some(file_size - 1),
    })
}

/// Parse standard range (e.g., "0-99" or "100-")
fn parse_standard_range(start_str: &str, end_str: &str, file_size: usize) -> RangeParseResult {
    let Ok(start) = start_str.parse::<usize>() else {
        return RangeParseResult::None;
    };

    // Start beyond file size is not satisfiable
    if start >= file_size {
        return RangeParseResult::NotSatisfiable;
    }

    let end = if end_str.is_empty() {
        None // Open-ended range
    } else {
        let Ok(e) = end_str.parse::<usize>() else {
            return RangeParseResult::None;
        };
        // Clamp end to file size - 1
        Some(e.min(file_size - 1))
    };

    // Validate: start <= end
    if let Some(e) = end {
        if start > e {
            return RangeParseResult::NotSatisfiable;
        }
    }

    RangeParseResult::Valid(RangeRequest { start, end })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_range() {
        assert!(matches!(
            parse_range_header(None, 100),
            RangeParseResult::None
        ));
    }

    #[test]
    fn test_standard_range() {
        match parse_range_header(Some("bytes=0-9"), 100) {
            RangeParseResult::Valid(r) => {
                assert_eq!(r.start, 0);
                assert_eq!(r.end, Some(9));
                assert_eq!(r.content_length(100), 10);
            }
            _ => panic!("Expected Valid"),
        }
    }

    #[test]
    fn test_open_range() {
        match parse_range_header(Some("bytes=50-"), 100) {
            RangeParseResult::Valid(r) => {
                assert_eq!(r.start, 50);
                assert_eq!(r.end, None);
                assert_eq!(r.end_position(100), 99);
                assert_eq!(r.content_length(100), 50);
            }
            _ => panic!("Expected Valid"),
        }
    }

    #[test]
    fn test_suffix_range() {
        match parse_range_header(Some("bytes=-20"), 100) {
            RangeParseResult::Valid(r) => {
                assert_eq!(r.start, 80);
                assert_eq!(r.end, Some(99));
            }
            _ => panic!("Expected Valid"),
        }
    }

    #[test]
    fn test_not_satisfiable() {
        assert!(matches!(
            parse_range_header(Some("bytes=200-"), 100),
            RangeParseResult::NotSatisfiable
        ));
    }

    #[test]
    fn test_invalid_format() {
        assert!(matches!(
            parse_range_header(Some("bytes=a-b"), 100),
            RangeParseResult::None
        ));
        assert!(matches!(
            parse_range_header(Some("bytes=0-9,20-29"), 100),
            RangeParseResult::None
        ));
    }
}
