# Configuration API (xDS Style)

## Overview

YARHS provides a dynamic configuration API inspired by Envoy's xDS (x Discovery Service) protocol. This allows you to:

- **View and update configuration** at runtime without restarting
- **Track changes with versions** using optimistic locking
- **Update individual resources** without affecting others
- **Graceful restarts** when server addresses change

**API Server**: `http://localhost:8000` (configurable via `server.api_port`)

---

## API Endpoints

### xDS Discovery Endpoints (Recommended)

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/v1/discovery` | Get snapshot of all resources |
| GET | `/v1/discovery:listeners` | Get listener (server address) config |
| POST | `/v1/discovery:listeners` | Update listener config |
| GET | `/v1/discovery:routes` | Get routing config |
| POST | `/v1/discovery:routes` | Update routing config |
| GET | `/v1/discovery:http` | Get HTTP config |
| POST | `/v1/discovery:http` | Update HTTP config |
| GET | `/v1/discovery:logging` | Get logging config |
| POST | `/v1/discovery:logging` | Update logging config |
| GET | `/v1/discovery:performance` | Get performance config |
| POST | `/v1/discovery:performance` | Update performance config |
| GET | `/v1/discovery:vhosts` | Get virtual hosts config |
| POST | `/v1/discovery:vhosts` | Update virtual hosts config |

---

## Resource Types

YARHS defines 6 resource types:

| Type | Description | Changes Require Restart |
|------|-------------|------------------------|
| `LISTENER` | Server addresses (main & API) | ✅ Yes |
| `ROUTE` | URL routing rules | ❌ No (hot reload) |
| `HTTP` | HTTP settings (CORS, headers) | ❌ No (hot reload) |
| `LOGGING` | Log level, access log | ❌ No (hot reload) |
| `PERFORMANCE` | Timeouts, connections | ❌ No (hot reload) |
| `VIRTUAL_HOST` | Domain-based routing | ❌ No (hot reload) |

---

## xDS Response Format

### Discovery Response

```json
{
  "version_info": "1768373413797",
  "resources": [...],
  "nonce": "1",
  "type_url": "type.yarhs.io/LISTENER"
}
```

| Field | Description |
|-------|-------------|
| `version_info` | Resource version (Unix timestamp in ms) |
| `resources` | Array of resources |
| `nonce` | Incremental counter for this resource type |
| `type_url` | Resource type identifier |

### Resource Format

```json
{
  "@type": "type.yarhs.io/ROUTE",
  "name": "default",
  "index_files": [...],
  "custom_routes": {...}
}
```

### ACK Response (Success)

```json
{
  "status": "ACK",
  "version_info": "1768373459561",
  "nonce": "2",
  "message": "Routes updated"
}
```

### NACK Response (Error)

```json
{
  "status": "NACK",
  "error_detail": {
    "code": 400,
    "message": "Invalid route resource: missing field"
  }
}
```

---

## API Usage Examples

### 1. Get All Resources (Snapshot)

```bash
curl http://localhost:8000/v1/discovery
```

Response:
```json
{
  "version_info": "1768373413797",
  "resources": {
    "listener": {
      "version_info": "1768373413797",
      "nonce": "1",
      "value": {
        "main_server": { "host": "127.0.0.1", "port": 8080 },
        "api_server": { "host": "0.0.0.0", "port": 8000 }
      }
    },
    "route": {...},
    "http": {...},
    "logging": {...},
    "performance": {...}
  }
}
```

### 2. Get Specific Resource Type

```bash
# Get routes only
curl http://localhost:8000/v1/discovery:routes

# Get logging config
curl http://localhost:8000/v1/discovery:logging
```

### 3. Update a Resource (POST)

```bash
# Update logging config
curl -X POST http://localhost:8000/v1/discovery:logging \
  -H "Content-Type: application/json" \
  -d '{
    "resources": [{
      "level": "debug",
      "access_log": true,
      "show_headers": true
    }]
  }'
```

Response:
```json
{
  "status": "ACK",
  "version_info": "1768373459561",
  "nonce": "2",
  "message": "Logging config updated"
}
```

### 4. Optimistic Locking (Prevent Conflicts)

Include `version_info` to ensure no concurrent updates:

```bash
# First get current version
VERSION=$(curl -s http://localhost:8000/v1/discovery:logging | jq -r '.version_info')

# Update with version check
curl -X POST http://localhost:8000/v1/discovery:logging \
  -H "Content-Type: application/json" \
  -d "{
    \"version_info\": \"$VERSION\",
    \"resources\": [{
      \"level\": \"info\",
      \"access_log\": false,
      \"show_headers\": false
    }]
  }"
```

If version mismatches, returns:
```json
{
  "status": "NACK",
  "error_detail": {
    "code": 409,
    "message": "Version conflict: expected 1768373459561, got 12345"
  }
}
```

### 5. Update Server Address (Triggers Restart)

```bash
curl -X POST http://localhost:8000/v1/discovery:listeners \
  -H "Content-Type: application/json" \
  -d '{
    "resources": [{
      "main_server": { "host": "127.0.0.1", "port": 9090 }
    }],
    "force_restart": false
  }'
```

### 6. Add Custom Routes

```bash
curl -X POST http://localhost:8000/v1/discovery:routes \
  -H "Content-Type: application/json" \
  -d '{
    "resources": [{
      "index_files": ["index.html", "index.htm"],
      "custom_routes": {
        "/home": { "type": "file", "path": "templates/index.html" },
        "/static": { "type": "dir", "path": "static" },
        "/docs": { "type": "redirect", "target": "https://docs.example.com" }
      }
    }]
  }'
```

### 7. Configure Health Check Endpoints

```bash
# Update health check paths
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

# Disable health check endpoints
curl -X POST http://localhost:8000/v1/discovery:routes \
  -H "Content-Type: application/json" \
  -d '{
    "resources": [{
      "health": {
        "enabled": false
      }
    }]
  }'
```

**Health check endpoints:**
- Liveness probe (default: `/healthz`) - Returns `200 OK` with body `"ok"`
- Readiness probe (default: `/readyz`) - Returns `200 OK` with body `"ok"`
- Headers: `Cache-Control: no-cache, no-store, must-revalidate`

---

## Resource Schemas

### LISTENER Resource

```json
{
  "main_server": {
    "host": "127.0.0.1",
    "port": 8080
  },
  "api_server": {
    "host": "0.0.0.0",
    "port": 8000
  }
}
```

### ROUTE Resource

```json
{
  "index_files": ["index.html", "index.htm"],
  "custom_routes": {
    "/path": {
      "type": "file|dir|redirect",
      "path": "local/path",       // for file/dir
      "target": "https://..."     // for redirect
    }
  },
  "health": {
    "enabled": true,
    "liveness_path": "/healthz",
    "readiness_path": "/readyz"
  }
}
```

### HTTP Resource

```json
{
  "default_content_type": "text/html; charset=utf-8",
  "server_name": "Tokio-Hyper/1.0",
  "enable_cors": false,
  "max_body_size": 10485760
}
```

### LOGGING Resource

```json
{
  "level": "debug|info|warn|error",
  "access_log": true,
  "show_headers": false
}
```

### PERFORMANCE Resource

```json
{
  "keep_alive_timeout": 75,
  "read_timeout": 30,
  "write_timeout": 30,
  "max_connections": 5000
}
```

### VIRTUAL_HOST Resource

```json
{
  "virtual_hosts": [
    {
      "name": "api-site",
      "domains": ["api.example.com"],
      "routes": [
        {
          "name": "api-v1",
          "match": {
            "prefix": "/v1",
            "headers": [
              {"name": "X-API-Version", "exact": "1"}
            ]
          },
          "type": "dir",
          "path": "/var/www/api/v1"
        }
      ]
    },
    {
      "name": "www-site",
      "domains": ["www.example.com", "*.example.com"],
      "routes": [
        {
          "name": "static",
          "match": {"prefix": "/"},
          "type": "dir",
          "path": "/var/www/html"
        }
      ]
    },
    {
      "name": "catch-all",
      "domains": ["*"],
      "routes": [
        {
          "name": "default",
          "match": {"prefix": "/"},
          "type": "direct",
          "status": 404,
          "body": "Unknown host"
        }
      ]
    }
  ]
}
```

**VirtualHost Fields:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Unique identifier for the virtual host |
| `domains` | array | Yes | List of domains to match (`*` = catch-all, `*.example.com` = wildcard) |
| `routes` | array | Yes | List of routes for this virtual host |
| `index_files` | array | No | Override default index files for this host |

**Route Fields:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | No | Optional route name for identification |
| `match` | object | Yes | Match conditions (prefix, path, headers) |
| `type` | string | Yes | Action type: `dir`, `file`, `redirect`, `direct` |

**Match Conditions:**
| Field | Type | Description |
|-------|------|-------------|
| `prefix` | string | Path prefix match (e.g., `/api` matches `/api/users`) |
| `path` | string | Exact path match |
| `headers` | array | Header matchers (name, exact/prefix/present) |

**Route Actions:**
| Type | Fields | Description |
|------|--------|-------------|
| `dir` | `path` | Serve files from directory |
| `file` | `path` | Serve a specific file |
| `redirect` | `target`, `code` (default: 302) | HTTP redirect |
| `direct` | `status`, `body`, `content_type` | Direct response |

**Domain Matching Priority:**
1. Exact match (`api.example.com`)
2. Wildcard match (`*.example.com`)
3. Catch-all (`*`)

---

## Error Codes

| HTTP Code | Status | Description |
|-----------|--------|-------------|
| 200 | ACK | Update successful |
| 400 | NACK | Invalid request (bad JSON, missing fields) |
| 404 | - | Unknown endpoint |
| 405 | - | Method not allowed |
| 409 | NACK | Version conflict (optimistic lock failure) |
| 500 | - | Internal server error |

---

## Best Practices

1. **Use xDS endpoints** for granular updates
2. **Always use optimistic locking** (`version_info`) in production
3. **Avoid frequent listener updates** as they trigger server restarts
4. **Route/HTTP/Logging/Performance** changes are instant (no restart)
5. **Monitor nonce values** to track configuration changes over time
