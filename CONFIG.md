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
- `server.workers` - Worker thread count (optional)

### Logging Configuration
- `logging.level` - Log verbosity: "debug", "info", "error" (default: "info")
- `logging.access_log` - Enable access logging (default: true)
- `logging.show_headers` - Show request headers (default: false)

### Resources Configuration
- `resources.template_dir` - Template directory path (default: "templates")
- `resources.static_dir` - Static files directory (optional)
- `resources.max_body_size` - Max request body size in bytes (default: 10485760)

### Performance Configuration
- `performance.keep_alive_timeout` - Keep-alive timeout in seconds (default: 75)
- `performance.read_timeout` - Read timeout in seconds (default: 30)
- `performance.write_timeout` - Write timeout in seconds (default: 30)
- `performance.max_connections` - Max concurrent connections (optional)

### HTTP Configuration
- `http.default_content_type` - Default Content-Type header
- `http.server_name` - Server name header (default: "Tokio-Hyper/1.0")
- `http.enable_cors` - Enable CORS headers (default: false)

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

[logging]
level = "debug"
access_log = true
show_headers = true

[http]
enable_cors = true
```

### Production config
```toml
[server]
host = "0.0.0.0"
port = 80

[logging]
level = "info"
access_log = false

[performance]
max_connections = 10000
keep_alive_timeout = 60
```
