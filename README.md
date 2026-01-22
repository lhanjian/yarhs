# 🚀 YARHS - Yet Another Rust HTTP Server

[中文文档](README.zh-CN.md)

A production-ready, high-performance async HTTP server in Rust with **dynamic routing**, **xDS-style API**, zero-downtime hot restart, and enterprise-grade features.

## ✨ Key Features

### 1. xDS-Style Configuration API
- ✅ **Resource Discovery Endpoints** - Envoy xDS-like protocol, manage resources by type
- ✅ **Version Control** - Optimistic locking prevents concurrent conflicts
- ✅ **Incremental Updates** - Update only what you need without affecting other configs

```bash
# xDS-style API (Recommended)
curl http://localhost:8000/v1/discovery           # Get all resources snapshot
curl http://localhost:8000/v1/discovery:routes    # Get routing config
curl http://localhost:8000/v1/discovery:logging   # Get logging config

# Update a single resource
curl -X POST http://localhost:8000/v1/discovery:logging \
  -H "Content-Type: application/json" \
  -d '{"resources": [{"level": "debug", "access_log": true, "show_headers": true}]}'
```

### 2. Dynamic Routing
- ✅ **Runtime Route Modification** - Add/modify routes via API without restart
- ✅ **Multiple Route Types** - Support for file, directory, and redirect routes
- ✅ **Default Documents** - Auto-serve index.html for directory access
- ✅ **ETag + 304** - Conditional request support, saves bandwidth
- ✅ **Exact Matching** - Custom routes use exact match, static files use prefix match
- ✅ **Priority Control** - Favicon > Custom Routes > Static Files > Default Homepage
- Uses `SO_REUSEPORT` for concurrent port listening
- Dual-loop concurrency model: new listener starts immediately, old one drains gracefully
- Supports same-port restart and cross-port migration

### 3. Smart Caching System
- Configuration hot-reload cache
- Atomic operations avoid lock contention
- **ETag Support** - Content hash-based ETag generation
- **Conditional Requests** - Returns 304 Not Modified when If-None-Match matches

### 4. HTTP Method Handling (Nginx-style)
- ✅ **GET** - Return file content normally
- ✅ **HEAD** - Return headers only (with Content-Length), no body
- ✅ **OPTIONS** - Return 204 with Allow header, CORS preflight support
- ❌ **POST/PUT/DELETE** - Return 405 Method Not Allowed with Allow header

### 5. Health Check Endpoints
- ✅ **Liveness Probe** - `/healthz` endpoint for Kubernetes liveness checks
- ✅ **Readiness Probe** - `/readyz` endpoint for readiness checks
- ✅ **Configurable Paths** - Customize endpoint paths via config or API
- ✅ **No-Cache Headers** - Proper cache control for health responses
- ✅ **Dynamic Toggle** - Enable/disable health endpoints at runtime

```bash
# Built-in health check endpoints
curl http://localhost:8080/healthz   # Liveness probe -> "ok"
curl http://localhost:8080/readyz    # Readiness probe -> "ok"

# Configure custom paths via API
curl -X POST http://localhost:8000/v1/discovery:routes \
  -H "Content-Type: application/json" \
  -d '{
    "resources": [{
      "health": {
        "enabled": true,
        "liveness_path": "/health/live",
        "readiness_path": "/health/ready"
      }
    }]
  }'
```

### 6. Access Log Formatting
- ✅ **Multiple Formats** - combined (Apache/Nginx), common (CLF), json, custom
- ✅ **Custom Patterns** - Define your own log format with variables
- ✅ **Request Timing** - Includes request processing time in logs
- ✅ **Dynamic Configuration** - Change format at runtime via API

```bash
# Supported formats
# combined: Apache/Nginx combined format (default)
# common: Common Log Format (CLF)
# json: JSON structured logs
# custom: Your own pattern with variables

# Configure via API
curl -X POST http://localhost:8000/v1/discovery:logging \
  -H "Content-Type: application/json" \
  -d '{
    "resources": [{
      "level": "info",
      "access_log": true,
      "access_log_format": "json"
    }]
  }'

# Custom format variables:
# $remote_addr, $time_local, $time_iso8601, $request, $request_method
# $request_uri, $status, $body_bytes_sent, $http_referer
# $http_user_agent, $request_time
```

### 7. Log File Output
- ✅ **File-based Logging** - Write access and error logs to files
- ✅ **Runtime Configuration** - Change log file paths via API
- ✅ **Auto Directory Creation** - Creates parent directories automatically
- ✅ **Thread-safe** - Safe concurrent writes from multiple handlers

```bash
# Configure log files via API
curl -X POST http://localhost:8000/v1/discovery:logging \
  -H "Content-Type: application/json" \
  -d '{
    "resources": [{
      "level": "info",
      "access_log": true,
      "access_log_format": "combined",
      "access_log_file": "/var/log/yarhs/access.log",
      "error_log_file": "/var/log/yarhs/error.log"
    }]
  }'

# Or configure in config.toml
# [logging]
# access_log_file = "/var/log/yarhs/access.log"
# error_log_file = "/var/log/yarhs/error.log"
```

### 8. High Performance
- **40k+ QPS** (static files)
- **63k+ QPS** (API endpoints)
- Fully async I/O, built on Tokio + Hyper

## 📦 Project Structure

```
yarhs/
├── src/
│   ├── main.rs           - Entry point
│   ├── handler.rs        - Dynamic route handling
│   ├── response.rs       - Response building, ETag, caching
│   ├── logger.rs         - Logging utilities
│   ├── api/              - xDS-style configuration API module
│   │   ├── mod.rs        - Module exports and routing
│   │   ├── types.rs      - xDS type definitions
│   │   ├── handlers.rs   - GET/POST request handlers
│   │   ├── updaters.rs   - Resource update functions
│   │   └── response.rs   - API response builders
│   ├── config/           - Configuration management module
│   │   ├── mod.rs        - Config loading
│   │   ├── types.rs      - Config type definitions
│   │   ├── state.rs      - AppState shared state
│   │   └── version.rs    - xDS version management
│   └── server/           - Server core module
│       ├── mod.rs        - Module exports
│       ├── listener.rs   - TCP listener (SO_REUSEPORT)
│       ├── connection.rs - Connection handling
│       ├── loop.rs       - Server main loop
│       ├── restart.rs    - Hot restart logic
│       └── signal.rs     - Signal handling
├── scripts/              - Test scripts
│   ├── run_all_tests.sh  - Unified test script
│   └── integration_tests.sh - Integration tests
├── static/               - Static assets
├── templates/            - HTML templates
├── config.toml           - Server configuration
├── API.md                - xDS API documentation
├── CONFIG.md             - Configuration documentation
└── ROUTES.md             - Routing documentation
```

## 🎯 Route Configuration

### Configuration Example

**config.toml:**
```toml
[routes]
favicon_paths = ["/favicon.ico", "/favicon.svg"]

[routes.custom_routes]
"/about" = { type = "file", path = "templates/about.html" }
"/static" = { type = "dir", path = "static" }
"/old-url" = { type = "redirect", target = "/new-url" }
```

### Route Types

| Type | Description | Example |
|------|-------------|---------|
| `file` | Return a single file (auto MIME detection) | `{"type": "file", "path": "templates/about.html"}` |
| `dir` | Directory mapping (prefix match) | `{"type": "dir", "path": "static"}` |
| `redirect` | HTTP 302 redirect | `{"type": "redirect", "target": "/new"}` |

### API Operations

```bash
# === xDS-style API (Recommended) ===

# View all config snapshot
curl http://localhost:8000/v1/discovery

# View specific resources
curl http://localhost:8000/v1/discovery:routes
curl http://localhost:8000/v1/discovery:logging

# Update route configuration
curl -X POST http://localhost:8000/v1/discovery:routes \
  -H "Content-Type: application/json" \
  -d '{
    "resources": [{
      "favicon_paths": ["/favicon.ico"],
      "index_files": ["index.html"],
      "custom_routes": {
        "/about": {"type": "file", "path": "templates/about.html"}
      }
    }]
  }'

# View full API documentation
curl http://localhost:8000/v1/discovery  # Get all resources
```

## 🚀 Quick Start

### Build and Run

```bash
# Development mode
cargo run

# Production build
cargo build --release
./target/release/rust_webserver
```

### Test Routing Features

```bash
# Run all tests (unit tests + integration tests)
./scripts/run_all_tests.sh

# Run integration tests only
./scripts/integration_tests.sh
```

The test script will:
1. Start the server
2. Create test files (HTML templates)
3. Dynamically configure routes
4. Test all route types
5. Run performance comparison tests
6. Auto cleanup

### Performance Testing

```bash
# Using wrk
wrk -t4 -c100 -d30s http://127.0.0.1:8080/

# Using ApacheBench
ab -n 10000 -c 100 http://127.0.0.1:8080/
```

## 📚 Tech Stack

- **Tokio 1.41** - Async runtime
- **Hyper 1.5** - HTTP server
- **socket2 0.6** - Low-level socket control (SO_REUSEPORT)
- **serde + serde_json** - JSON serialization
- **config 0.14** - TOML config management

## 🔧 Configuration

See [CONFIG.md](CONFIG.md) for full configuration reference.

Main configuration sections:
- `server` - Server address and ports
- `logging` - Log level, access log
- `http` - HTTP headers, CORS
- `performance` - Timeouts, connection limits
- `routes` - Route configuration (dynamic)

## 📖 Documentation

- [API.md](API.md) - Dynamic configuration API documentation
- [CONFIG.md](CONFIG.md) - Configuration options reference
- [ROUTES.md](ROUTES.md) - Complete routing guide

## ⚡ Performance Benchmarks

Benchmark results (wrk 4 threads, 100 connections, 30 seconds):

| Route Type | QPS | Notes |
|------------|-----|-------|
| API endpoints | ~63,000 | Pure JSON response |
| Static files | ~40,000 | Async file read |
| File routes | ~35,000 | Single file loading |

## 🎨 Use Cases

### 1. Documentation Site
```toml
[routes.custom_routes]
"/guide" = { type = "file", path = "docs/guide.html" }
"/api" = { type = "file", path = "docs/api.html" }
"/changelog" = { type = "file", path = "docs/changelog.html" }
```

### 2. Multi-language Website
```toml
[routes.custom_routes]
"/zh" = { type = "file", path = "templates/index-zh.html" }
"/en" = { type = "file", path = "templates/index-en.html" }
"/ja" = { type = "file", path = "templates/index-ja.html" }
```

### 3. File Download Site
```toml
[routes.custom_routes]
"/downloads" = { type = "dir", path = "public/downloads" }
"/images" = { type = "dir", path = "public/images" }
```

### 4. URL Redirects
```toml
[routes.custom_routes]
"/old-api" = { type = "redirect", target = "/api/v2" }
"/docs-v1" = { type = "redirect", target = "/docs/v2" }
```

## 🛡️ Code Quality

- ✅ **Zero Compiler Warnings** - Production-ready code
- ✅ **No Race Conditions** - Thread safety via atomic operations
- ✅ **Comprehensive Error Handling** - All I/O operations handle errors
- ✅ **Type Safety** - Full utilization of Rust's type system
- ✅ **Well Documented** - Key logic is thoroughly commented

## 💡 Innovations

1. **Dynamic Routing System** - Rare runtime route configuration
2. **SO_REUSEPORT Zero-Downtime** - Advanced hot-update approach
3. **Multi-layer Cache Optimization** - 10x+ performance improvement
4. **Type-Driven Design** - Flexible routing via Enum + Serde
5. **Production-Grade Architecture** - Complete monitoring, logging, performance optimization

## 📝 Changelog

### v0.3.0 (2026-01-22)

- ✨ Add HTTP method handling (Nginx-style)
  - GET: Normal response with body
  - HEAD: Headers only, no body (with Content-Length)
  - OPTIONS: 204 response with Allow header, CORS preflight support
  - Other methods: 405 Method Not Allowed
- 📚 Update documentation and test scripts

### v0.2.1 (2026-01-14)

- ✨ Add default document (index_files) support
- ✨ Add ETag + 304 conditional request support
- 🔧 Rename route types (template→file, static→dir)
- 🏗️ Modular refactoring: split into `api/`, `config/`, `server/` directories
- 🧹 Enable Clippy pedantic + nursery strict checks
- 🧪 Add unified test script `scripts/run_all_tests.sh`
- 🗑️ Remove legacy `/api/config` endpoints, use xDS-style API exclusively

### v0.2.0 (2026-01-07)

- ✨ Add dynamic route configuration
- ✨ Support 3 route types (File, Dir, Redirect)
- ✨ Runtime route modification via API
- 📚 Add ROUTES.md routing documentation
- 🧪 Add integration test suite

### v0.1.0 (2026-01-06)

- ✨ Basic HTTP server functionality
- ✨ Zero-downtime hot restart (SO_REUSEPORT)
- ✨ Dynamic configuration API
- ⚡ Performance optimization (30k+ QPS)
- 📚 Complete documentation (API.md, CONFIG.md)

## 🤝 Contributing

Issues and Pull Requests are welcome!

## 📄 License

MIT License

---

**Status**: ✅ Production Ready | 🚀 Actively Maintained

**Last Updated**: 2026-01-22
