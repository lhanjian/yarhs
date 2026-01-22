# Rust Web Server Configuration

This server supports flexible configuration through `config.toml` file and environment variables.

## Configuration Methods

### 1. Using config.toml (Primary)
Edit the `config.toml` file in the project root.

### 2. Using Environment Variables (Override)
Prefix any config key with `SERVER_` and use double underscores for nesting:
```bash
SERVER_SERVER__PORT=3000 cargo run
SERVER_LOGGING__LEVEL=debug cargo run
SERVER_HTTP__ENABLE_CORS=true cargo run
```

## Available Configuration Options

### Server Configuration
- `server.host` - Bind address (default: "127.0.0.1")
- `server.port` - Listen port (default: 8080)
- `server.api_host` - API server bind address (default: "0.0.0.0")
- `server.api_port` - API management port (default: 8000)
- `server.workers` - Worker thread count (optional, defaults to CPU cores)

### Logging Configuration
- `logging.level` - Log verbosity: "debug", "info", "error" (default: "info")
- `logging.access_log` - Enable access logging (default: true)
- `logging.show_headers` - Show request headers (default: false)
- `logging.access_log_format` - Access log format (default: "combined")
  - `combined` - Apache/Nginx combined format
  - `common` - Common Log Format (CLF)
  - `json` - JSON structured logging
  - Custom pattern with variables
- `logging.access_log_file` - Access log file path (optional, stdout if not set)
- `logging.error_log_file` - Error log file path (optional, stderr if not set)

### Performance Configuration
- `performance.keep_alive_timeout` - Keep-alive timeout in seconds (default: 75)
- `performance.read_timeout` - Read timeout in seconds (default: 30)
- `performance.write_timeout` - Write timeout in seconds (default: 30)
- `performance.max_connections` - Max concurrent connections (default: 5000)

### HTTP Configuration
- `http.default_content_type` - Default Content-Type header (default: "text/html; charset=utf-8")
- `http.server_name` - Server name header (default: "Tokio-Hyper/1.0")
- `http.enable_cors` - Enable CORS headers (default: false)
- `http.max_body_size` - Max request body size in bytes (default: 10485760)

### Routes Configuration
- `routes.favicon_paths` - Favicon URL paths (default: ["/favicon.ico", "/favicon.svg"])
- `routes.index_files` - Default document filenames (default: ["index.html", "index.htm"])
- `routes.custom_routes` - Custom route definitions (see [ROUTES.md](ROUTES.md))

### Health Check Configuration
- `routes.health.enabled` - Enable health check endpoints (default: true)
- `routes.health.liveness_path` - Liveness probe path (default: "/healthz")
- `routes.health.readiness_path` - Readiness probe path (default: "/readyz")

## Examples

### Minimal config.toml
```toml
[server]
port = 3000
```

### Development config
```toml
[server]
host = "0.0.0.0"
port = 8080
api_port = 8000

[logging]
level = "debug"
access_log = true
show_headers = true

[http]
enable_cors = true

[routes.custom_routes]
"/static" = { type = "dir", path = "static" }
```

### Production config
```toml
[server]
host = "0.0.0.0"
port = 80
api_host = "127.0.0.1"
api_port = 8000

[logging]
level = "info"
access_log = true
access_log_format = "combined"
access_log_file = "/var/log/yarhs/access.log"
error_log_file = "/var/log/yarhs/error.log"

[performance]
max_connections = 10000
keep_alive_timeout = 60

[routes]
index_files = ["index.html"]

[routes.health]
enabled = true
liveness_path = "/healthz"
readiness_path = "/readyz"

[routes.custom_routes]
"/assets" = { type = "dir", path = "public/assets" }
"/about" = { type = "file", path = "pages/about.html" }
```
