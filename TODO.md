# TODO: YARHS Roadmap

## ✅ Completed Features

### Core Functionality
- [x] Static file serving with MIME detection
- [x] Dynamic routing (file, dir, redirect)
- [x] ETag + 304 conditional requests
- [x] Range requests (resume download)
- [x] HTTP method handling (GET/HEAD/OPTIONS/405)
- [x] Hot restart with SO_REUSEPORT

### Configuration & API
- [x] xDS-style configuration API
- [x] Version control with optimistic locking
- [x] Virtual host routing with domain matching
- [x] Health check endpoints (/healthz, /readyz)
- [x] Access log formatting (combined, common, json, custom)
- [x] Log file output

### Operations
- [x] Signal handling (SIGTERM/SIGINT/SIGHUP)
- [x] Graceful shutdown with connection draining
- [x] Configuration persistence (state.toml, opt-in via config)
- [x] API Dashboard Web UI
- [x] Unit tests (33 tests) + Integration tests (177+ tests)

---

## 🚧 In Progress / Near-term

### 1. **HTTPS/TLS Support** ⭐⭐⭐⭐⭐
**Why**: Production deployment requires encryption. Currently HTTP only.

```rust
// Add rustls support
[dependencies]
tokio-rustls = "0.25"
rustls-pemfile = "2.0"
```

**Scope**:
- [ ] Load certificate and private key from files
- [ ] TLS configuration in config.toml
- [ ] HTTP -> HTTPS redirect option
- [ ] Dynamic certificate reload via API

---

### 2. **Observability: Metrics Endpoint** ⭐⭐⭐⭐

**Current**: Only println logging, no metrics export.

**Needed**:
- [ ] `/metrics` endpoint (Prometheus format)
- [ ] Basic metrics: `http_requests_total`, `http_request_duration_seconds`
- [ ] Connection count, active requests

```
# Example output
# HELP http_requests_total Total HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",status="200"} 12345
http_requests_total{method="GET",status="404"} 23
```

---

### 3. **Reverse Proxy / Load Balancer** ⭐⭐⭐⭐

**Why**: Many use cases need proxying to backend services.

**Scope**:
- [ ] Proxy route type: `{"type": "proxy", "upstream": "http://backend:8080"}`
- [ ] Connection pooling to backends
- [ ] Health checks for upstreams
- [ ] Load balancing (round-robin, least-conn)

---

### 4. **Rate Limiting** ⭐⭐⭐

**Why**: Protection against abuse and DoS.

**Scope**:
- [ ] Per-IP rate limiting
- [ ] Token bucket or sliding window algorithm
- [ ] Configurable limits per route/vhost
- [ ] Return 429 Too Many Requests

---

## 📋 Backlog (Future)

### Security
- [ ] Basic Auth / Token verification for admin API
- [x] Request body size limits (`max_body_size` in config)
- [ ] Audit logging for config changes

### Configuration
- [x] Config persistence to file (state.toml)
- [ ] Config diff / rollback capability
- [ ] WebSocket support for config push

### Performance
- [ ] Sendfile optimization for large files
- [ ] HTTP/2 support (h2)
- [ ] Compression (gzip, brotli)

### Developer Experience
- [ ] WASM build for edge deployment
- [ ] Docker image
- [ ] Helm chart for Kubernetes

---

## 📊 Current Status Summary

| Category | Status | Notes |
|----------|--------|-------|
| HTTP Server | ✅ Complete | GET/HEAD/OPTIONS, static files, caching |
| Routing | ✅ Complete | Virtual hosts, path matching, redirects |
| API | ✅ Complete | xDS-style, version control, web dashboard |
| Health | ✅ Complete | /healthz, /readyz, configurable |
| Logging | ✅ Complete | Multiple formats, file output |
| Persistence | ✅ Complete | state.toml for config persistence |
| Testing | ✅ Good | 33 unit + 177 integration tests |
| TLS | ❌ Missing | Priority #1 |
| Metrics | ❌ Missing | Priority #2 |
| Proxy | ❌ Missing | Priority #3 |

---

**Current Version**: v0.3.0 - Feature-complete HTTP server  
**Next Milestone**: v0.4.0 - HTTPS + Metrics
