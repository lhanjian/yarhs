# TODO: Architecture Improvements

A gap analysis comparing the current project with senior engineer standards.

---

## 🔍 Gap Analysis

### 1. **Error Handling: Lack of Unified Error Type System**

**Current:**
```rust
fn main() -> Result<(), Box<dyn std::error::Error>>  // Generic error, loses context
```

**Senior Approach:**
```rust
#[derive(Debug, thiserror::Error)]
enum YarhsError {
    #[error("Config error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Route not found: {path}")]
    RouteNotFound { path: String },
}
```

**Gap**: Cannot precisely capture, classify, or report errors; logs and monitoring cannot pinpoint root causes.

---

### 2. **Observability: Lack of Structured Metrics and Tracing**

**Current**: `println!` logging, no metrics exposed

**Senior Approach**:
- Prometheus metrics (QPS, latency percentiles, connection count)
- OpenTelemetry tracing (request chain tracing)
- Structured logging (JSON format, with request_id)

```rust
// Missing code like this
metrics::counter!("http_requests_total", "method" => method, "status" => status);
metrics::histogram!("http_request_duration_seconds").record(duration);
```

---

### 3. **Configuration Management: Incomplete Hot-Reload Mechanism**

**Current**: xDS API can update in-memory config, but:
- No config persistence (lost on restart)
- No config validation/rollback
- No config change audit

**Senior Approach**:
- Validate before write + dry-run
- Persist changes to storage
- Support `config diff` / `config rollback`
- Publish change events (support external system subscription)

---

### 4. **Lifecycle Management: Incomplete Graceful Shutdown**

**Current**: Has SO_REUSEPORT hot restart, but lacks:
- Signal handling (SIGTERM/SIGINT)
- Connection draining timeout
- Health check endpoints (/healthz, /readyz)

**Senior Approach**:
```rust
tokio::select! {
    _ = shutdown_signal() => {
        // Stop accepting new connections
        // Wait for existing connections to complete (with timeout)
        // Clean up resources
    }
}
```

---

### 5. **Testing: Insufficient Unit Test Coverage**

**Current**: `cargo test` shows 0 tests, only integration test scripts

**Senior Approach**:
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn route_handler_matches_file() { ... }
    
    #[tokio::test]
    async fn api_update_routes_returns_ack() { ... }
    
    #[test]
    fn etag_generation_deterministic() { ... }
}
```

Test pyramid: Unit tests > Integration tests > E2E tests

---

### 6. **Abstraction Level: Lack of Trait Abstraction**

**Current**: Concrete implementation coupling, e.g., `handle_api_config` directly matches paths

**Senior Approach**:
```rust
trait ResourceHandler {
    fn resource_type(&self) -> ResourceType;
    async fn get(&self, state: &AppState) -> Response;
    async fn update(&self, req: Request, state: &AppState) -> Response;
}

// Benefits:
// - Add new resource types without changing core logic
// - Test each handler independently
// - Support middleware (auth, rate limiting)
```

---

### 7. **Security: Lack of Basic Protection**

**Current**:
- No API authentication/authorization
- No request rate limiting
- No audit logging

**Production Requirements**:
```rust
// At minimum need:
- Basic Auth / Token verification
- Per-IP rate limiting
- Sensitive operation audit
```

---

### 8. **Dependency Injection: Hardcoded Dependencies**

**Current**: `AppState` directly holds `RwLock<DynamicConfig>`

**Senior Approach**:
```rust
struct AppState<C: ConfigStore, M: MetricsRecorder> {
    config: C,
    metrics: M,
}

// Benefits:
// - Mock in tests
// - Swap implementations (memory/Redis/etcd)
```

---

## 📊 Gap Summary

| Dimension | Current Level | Senior Level | Gap |
|-----------|---------------|--------------|-----|
| Error Handling | Generic Box<dyn Error> | Custom error types + context | ⭐⭐⭐ |
| Observability | println logging | Metrics + Tracing + structured logs | ⭐⭐⭐⭐ |
| Test Coverage | 0 unit tests | 80%+ coverage | ⭐⭐⭐⭐ |
| Abstraction Design | Concrete impl | Trait abstraction + DI | ⭐⭐⭐ |
| Production Ready | Basic features | Health check/graceful shutdown/rate limit/auth | ⭐⭐⭐⭐ |
| Config Management | In-memory hot-reload | Persistence + validation + audit + rollback | ⭐⭐⭐ |

---

## 💡 Priority Improvements

- [ ] **Add structured error types** (low effort, high value)
- [ ] **Add /healthz endpoint** (required for K8s deployment)
- [ ] **Add basic Prometheus metrics**
- [ ] **Add core logic unit tests**
- [ ] **Implement graceful shutdown**

---

**Current Status**: Feature-complete prototype  
**Target**: Production-grade service (requires investment in observability, reliability, security)
