# Dynamic Routing Configuration

## Overview

This server supports dynamic routing configuration, allowing you to modify routing rules via API at runtime without restarting the server (unless you modify server configuration).

## Route Configuration Structure

### Basic Route Configuration

```toml
[routes]
favicon_paths = ["/favicon.ico", "/favicon.svg"]  # Favicon path list
index_files = ["index.html", "index.htm"]         # Default document list
```

### Default Documents

When accessing a directory path (e.g., `/static/` or `/`), the server automatically looks for files configured in `index_files`:

```toml
[routes]
index_files = ["index.html", "index.htm"]
```

**Behavior Examples**:
- Access `/static/` → Returns `static/index.html` (if exists)
- Access `/` (configured as `"/" = { type = "dir", path = "public" }`) → Returns `public/index.html`

**Lookup Order**: In configuration list order, returns the first existing file found.

### Custom Routes

Define custom routes in `config.toml`:

```toml
[routes.custom_routes]
"/about" = { type = "file", path = "templates/about.html" }
"/old-path" = { type = "redirect", target = "/new-path" }
"/assets" = { type = "dir", path = "public/assets" }
```

## Route Types

### 1. File (Single File Route)

Returns a specified single file, automatically detecting MIME type based on extension:

```json
{
  "type": "file",
  "path": "templates/about.html"
}
```

**Supported File Types**:
- HTML, CSS, JS, JSON
- PNG, JPG, GIF, SVG, WebP, ICO
- PDF, XML, TXT, MD
- Font files (WOFF, WOFF2, TTF)
- Audio/Video (MP4, WebM, MP3, WAV)
- Other files return `application/octet-stream`

**Example**:
- Config: `"/about" = { type = "file", path = "templates/about.html" }`
- Access: `/about` → Returns `templates/about.html`

### 2. Dir (Directory Route)

Maps a URL prefix to a file directory, supporting prefix matching and default documents:

```json
{
  "type": "dir",
  "path": "static"
}
```

**Features**:
- Prefix matching (`/static/css/style.css` → `static/css/style.css`)
- Default documents (`/static/` → `static/index.html`)
- Root path mapping (`"/" = { type = "dir", path = "public" }`)

**Example**:
- Config: `"/static" = { type = "dir", path = "public/static" }`
- Access: `/static/css/style.css` → Reads `public/static/css/style.css`
- Access: `/static/` → Reads `public/static/index.html` (default document)

### 3. Redirect (Redirect Route)

Redirects requests to another URL:

```json
{
  "type": "redirect",
  "target": "/new-location"
}
```

**Example**:
- Config: `"/old" = { type = "redirect", target = "/new" }`
- Access: `/old` → 302 redirect to `/new`

## Route Priority

Route matching follows this priority order:

1. **Favicon Routes** - Exact paths in `favicon_paths` list
2. **Custom Routes (Exact Match)** - file/redirect types in `custom_routes`
3. **Custom Routes (Prefix Match)** - dir type in `custom_routes`
4. **Default Route** - Returns default homepage

## ETag and Conditional Requests

### ETag Mechanism

The server automatically generates ETags for all static files (based on fast content hashing) for client cache validation:

**Response Header Example**:
```
HTTP/1.1 200 OK
ETag: "23cc8d56a93cc61c"
Cache-Control: public, max-age=3600
```

### Conditional Requests (If-None-Match)

Clients can send an `If-None-Match` header to validate cache:

**Request**:
```bash
curl -H 'If-None-Match: "23cc8d56a93cc61c"' http://localhost:8080/static/test.txt
```

**When ETag Matches** (304 Not Modified):
```
HTTP/1.1 304 Not Modified
ETag: "23cc8d56a93cc61c"
Cache-Control: public, max-age=3600
```

**When ETag Doesn't Match** (200 OK):
```
HTTP/1.1 200 OK
ETag: "new-etag-value"
Cache-Control: public, max-age=3600
Content-Length: 23
```

### Bandwidth Savings

Using 304 responses significantly reduces bandwidth consumption:
- No response body transmitted
- Only necessary headers returned
- Browser uses locally cached content

## Modifying Routes via API

### View Current Route Configuration

```bash
curl http://localhost:8000/v1/discovery:routes | jq '.resources[0]'
```

### Update Route Configuration

Update routes via xDS-style endpoint:

```bash
# POST route resources (xDS format)
curl -X POST http://localhost:8000/v1/discovery:routes \
  -H "Content-Type: application/json" \
  -d '{"resources": [{
    "favicon_paths": ["/favicon.ico", "/favicon.svg"],
    "index_files": ["index.html"],
    "custom_routes": {
      "/about": {"type": "file", "path": "templates/about.html"},
      "/data": {"type": "file", "path": "static/data.json"},
      "/static": {"type": "dir", "path": "static"}
    }
}]}'
```

## Configuration Examples

### Multi-language Website Configuration

```toml
[routes.custom_routes]
"/zh" = { type = "file", path = "templates/index-zh.html" }
"/en" = { type = "file", path = "templates/index-en.html" }
"/ja" = { type = "file", path = "templates/index-ja.html" }
```

### File Download Site Configuration

```toml
[routes.custom_routes]
"/downloads" = { type = "dir", path = "public/downloads" }
"/images" = { type = "dir", path = "public/images" }
"/videos" = { type = "dir", path = "public/videos" }
```

### URL Redirect Configuration

```toml
[routes.custom_routes]
"/old-api" = { type = "redirect", target = "/api/v2" }
"/legacy" = { type = "redirect", target = "/new" }
```

### Mixed Configuration

```toml
[routes.custom_routes]
"/about" = { type = "file", path = "pages/about.html" }
"/api-spec" = { type = "file", path = "docs/openapi.json" }
"/assets" = { type = "dir", path = "public/assets" }
"/old-docs" = { type = "redirect", target = "/docs" }
```

## Notes

1. **Paths must start with `/`**
2. **file/redirect use exact matching**, dir uses prefix matching
3. **path field is relative to server working directory**
4. **Route configuration changes take effect immediately**, no restart required
5. **Security**: Directory routes perform path traversal checks to prevent accessing parent directories
6. **Full configuration object required when updating**, partial updates not yet supported

## API Response Format

### Success Response

```json
{
  "status": "ok",
  "message": "Configuration updated"
}
```

### Error Response

```json
{
  "error": "Invalid JSON: missing field `routes` at line 10 column 3"
}
```

## Performance Considerations

- Route configuration uses `HashMap` storage, O(1) lookup time complexity
- Configuration updates use read-write lock `RwLock`, concurrent read operations allowed
- Static files use async I/O, non-blocking server
