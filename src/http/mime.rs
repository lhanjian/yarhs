//! MIME type detection module
//!
//! Returns the corresponding Content-Type based on file extension.

/// Get MIME Content-Type based on file extension
///
/// # Examples
/// ```
/// use yarhs::http::mime::get_content_type;
/// assert_eq!(get_content_type(Some("html")), "text/html; charset=utf-8");
/// assert_eq!(get_content_type(Some("mp4")), "video/mp4");
/// assert_eq!(get_content_type(None), "application/octet-stream");
/// ```
pub fn get_content_type(extension: Option<&str>) -> &'static str {
    match extension {
        // Text
        Some("html" | "htm") => "text/html; charset=utf-8",
        Some("css") => "text/css",
        Some("txt" | "md") => "text/plain; charset=utf-8",
        Some("xml") => "application/xml",

        // JavaScript/WASM
        Some("js" | "mjs") => "application/javascript",
        Some("json") => "application/json",
        Some("wasm") => "application/wasm",

        // Images
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("webp") => "image/webp",

        // Video
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("ogg" | "ogv") => "video/ogg",
        Some("mov") => "video/quicktime",
        Some("avi") => "video/x-msvideo",

        // Audio
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("flac") => "audio/flac",
        Some("m4a") => "audio/mp4",

        // Fonts
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("eot") => "application/vnd.ms-fontobject",

        // Documents
        Some("pdf") => "application/pdf",
        Some("zip") => "application/zip",
        Some("gz" | "gzip") => "application/gzip",
        Some("tar") => "application/x-tar",

        // Default
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_types() {
        assert_eq!(get_content_type(Some("html")), "text/html; charset=utf-8");
        assert_eq!(get_content_type(Some("css")), "text/css");
        assert_eq!(get_content_type(Some("js")), "application/javascript");
        assert_eq!(get_content_type(Some("json")), "application/json");
        assert_eq!(get_content_type(Some("png")), "image/png");
        assert_eq!(get_content_type(Some("mp4")), "video/mp4");
    }

    #[test]
    fn test_unknown_extension() {
        assert_eq!(get_content_type(Some("xyz")), "application/octet-stream");
        assert_eq!(get_content_type(None), "application/octet-stream");
    }
}
