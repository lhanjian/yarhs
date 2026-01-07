# Dynamic Configuration API

## Overview

This API allows you to view and modify server configuration at runtime without restarting the service. Only specific configuration categories are available for dynamic updates.

**Base URL**: `http://localhost:8080/api`

---

## API Endpoints

### GET /api/config

Retrieve the current dynamic configuration.

#### Request

```http
GET /api/config HTTP/1.1
Host: localhost:8080
```

#### Response

**Status Code**: `200 OK`

**Headers**:
- `Content-Type: application/json`

**Body**:
```json
{
  "logging": {
    "level": "info",
    "access_log": true,
    "show_headers": false
  },
  "http": {
    "default_content_type": "text/html; charset=utf-8",
    "server_name": "Tokio-Hyper/1.0",
    "enable_cors": false
  },
  "resources": {
    "template_dir": "templates"
  }
}
```

#### Example

```bash
# View current configuration
curl http://localhost:8080/api/config

# Pretty print with jq
curl -s http://localhost:8080/api/config | jq
```

---

### PUT /api/config

Update the dynamic configuration. All fields must be provided in the request body.

#### Request

```http
PUT /api/config HTTP/1.1
Host: localhost:8080
Content-Type: application/json

{
  "logging": { ... },
  "http": { ... },
  "resources": { ... }
}
```

#### Request Body Schema

```json
{
  "logging": {
    "level": "string",        // "debug" | "info" | "error"
    "access_log": boolean,    // Enable HTTP access logging
    "show_headers": boolean   // Show request headers in logs
  },
  "http": {
    "default_content_type": "string",  // Default Content-Type header
    "server_name": "string",           // Server header value
    "enable_cors": boolean             // Enable CORS headers
  },
  "resources": {
    "template_dir": "string"  // Template directory path
  }
}
```

#### Response (Success)

**Status Code**: `200 OK`

**Body**:
```json
{
  "status": "ok",
  "message": "Configuration updated"
}
```

#### Response (Error)

**Status Code**: `400 Bad Request`

**Body**:
```json
{
  "error": "Invalid JSON: missing field `logging`"
}
```

#### Examples

##### Enable Debug Logging

```bash
curl -X PUT http://localhost:8080/api/config \
  -H "Content-Type: application/json" \
  -d '{
    "logging": {
      "level": "debug",
      "access_log": true,
      "show_headers": true
    },
    "http": {
      "default_content_type": "text/html; charset=utf-8",
      "server_name": "Tokio-Hyper/1.0",
      "enable_cors": false
    },
    "resources": {
      "template_dir": "templates"
    }
  }'
```

##### Enable CORS and Custom Server Name

```bash
curl -X PUT http://localhost:8080/api/config \
  -H "Content-Type: application/json" \
  -d '{
    "logging": {
      "level": "info",
      "access_log": true,
      "show_headers": false
    },
    "http": {
      "default_content_type": "text/html; charset=utf-8",
      "server_name": "MyCustomServer/2.0",
      "enable_cors": true
    },
    "resources": {
      "template_dir": "templates"
    }
  }'
```

##### Disable Access Logging (Silent Mode)

```bash
curl -X PUT http://localhost:8080/api/config \
  -H "Content-Type: application/json" \
  -d '{
    "logging": {
      "level": "error",
      "access_log": false,
      "show_headers": false
    },
    "http": {
      "default_content_type": "text/html; charset=utf-8",
      "server_name": "Tokio-Hyper/1.0",
      "enable_cors": false
    },
    "resources": {
      "template_dir": "templates"
    }
  }'
```

---

## Configuration Fields Reference

### Logging Configuration

| Field | Type | Description | Values |
|-------|------|-------------|--------|
| `level` | string | Log verbosity level | `"debug"`, `"info"`, `"error"` |
| `access_log` | boolean | Enable HTTP access logging | `true`, `false` |
| `show_headers` | boolean | Display request headers in logs | `true`, `false` |

**Effect**: Immediate

**Example Use Cases**:
- Set to `"debug"` during troubleshooting
- Disable `access_log` in production for performance
- Enable `show_headers` to inspect client requests

---

### HTTP Configuration

| Field | Type | Description | Example |
|-------|------|-------------|---------|
| `default_content_type` | string | Default Content-Type header | `"text/html; charset=utf-8"` |
| `server_name` | string | Server identification header | `"Tokio-Hyper/1.0"` |
| `enable_cors` | boolean | Add CORS headers to responses | `true`, `false` |

**Effect**: Immediate

**Example Use Cases**:
- Change `server_name` to hide server implementation
- Enable `enable_cors` for cross-origin API access
- Modify `default_content_type` for different content types

**Note**: When `enable_cors` is `true`, the following header is added:
```
Access-Control-Allow-Origin: *
```

---

### Resources Configuration

| Field | Type | Description | Example |
|-------|------|-------------|---------|
| `template_dir` | string | Directory containing HTML templates | `"templates"` |

**Effect**: Takes effect on next request (no restart required)

**Example Use Cases**:
- Switch between different template sets
- Point to a staging template directory for testing

---

## Non-Dynamic Configuration

The following configuration cannot be changed via API and requires a server restart:

### Server Configuration (`[server]` in config.toml)
- `host` - Bind address
- `port` - Listen port
- `workers` - Worker thread count

**Reason**: Requires rebinding network socket and reinitializing runtime.

### Performance Configuration (`[performance]` in config.toml)
- `keep_alive_timeout`
- `read_timeout`
- `write_timeout`
- `max_connections`

**Reason**: Only affects new connections; existing connections retain old settings.

To modify these, edit [`config.toml`](config.toml) and restart the server.

---

## Error Responses

### 400 Bad Request

Returned when the request body is malformed or contains invalid data.

```json
{
  "error": "Invalid JSON: expected value at line 1 column 1"
}
```

**Common Causes**:
- Malformed JSON syntax
- Missing required fields
- Invalid field types

### 405 Method Not Allowed

Returned when using an unsupported HTTP method.

```http
HTTP/1.1 405 Method Not Allowed
Content-Type: text/plain

Method Not Allowed
```

**Supported Methods**: `GET`, `PUT`

---

## Integration Examples

### Shell Script for Production Deployment

```bash
#!/bin/bash
# deploy-config.sh - Update server configuration

API_URL="http://localhost:8080/api/config"

# Get current config
echo "Current configuration:"
curl -s $API_URL | jq

# Update to production settings
echo -e "\nUpdating to production configuration..."
curl -X PUT $API_URL \
  -H "Content-Type: application/json" \
  -d '{
    "logging": {
      "level": "error",
      "access_log": false,
      "show_headers": false
    },
    "http": {
      "default_content_type": "text/html; charset=utf-8",
      "server_name": "WebServer/1.0",
      "enable_cors": false
    },
    "resources": {
      "template_dir": "templates"
    }
  }'

echo -e "\n\nNew configuration:"
curl -s $API_URL | jq
```

### Python Script

```python
import requests
import json

API_URL = "http://localhost:8080/api/config"

# Get current config
response = requests.get(API_URL)
current_config = response.json()
print("Current config:", json.dumps(current_config, indent=2))

# Update config
new_config = current_config.copy()
new_config["logging"]["level"] = "debug"
new_config["http"]["enable_cors"] = True

response = requests.put(API_URL, json=new_config)
print("Update result:", response.json())

# Verify update
response = requests.get(API_URL)
print("Updated config:", json.dumps(response.json(), indent=2))
```

### JavaScript (Node.js) Script

```javascript
const API_URL = "http://localhost:8080/api/config";

// Get current config
async function getConfig() {
  const response = await fetch(API_URL);
  return await response.json();
}

// Update config
async function updateConfig(config) {
  const response = await fetch(API_URL, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(config)
  });
  return await response.json();
}

// Usage
(async () => {
  const config = await getConfig();
  console.log("Current:", config);
  
  config.logging.level = "debug";
  config.http.enable_cors = true;
  
  const result = await updateConfig(config);
  console.log("Update result:", result);
})();
```

---

## Best Practices

### 1. Always GET Before PUT
```bash
# Get current config
current=$(curl -s http://localhost:8080/api/config)

# Modify only what you need
echo $current | jq '.logging.level = "debug"' | \
  curl -X PUT http://localhost:8080/api/config \
  -H "Content-Type: application/json" \
  -d @-
```

### 2. Validate Configuration Changes
```bash
# Update config
curl -X PUT http://localhost:8080/api/config -d @new-config.json

# Verify the change took effect
curl -I http://localhost:8080/ | grep -i "server:"
```

### 3. Use Version Control for Configs
```bash
# Save current config
curl -s http://localhost:8080/api/config > config-backup-$(date +%Y%m%d).json

# Restore from backup
curl -X PUT http://localhost:8080/api/config -d @config-backup-20260107.json
```

### 4. Monitor After Changes
```bash
# Enable debug logging
curl -X PUT http://localhost:8080/api/config -d @debug-config.json

# Watch logs
tail -f /var/log/webserver.log

# Restore when done
curl -X PUT http://localhost:8080/api/config -d @production-config.json
```

---

## Troubleshooting

### Configuration Not Taking Effect

**Issue**: Changes don't seem to apply.

**Solution**:
1. Verify the PUT request succeeded (status 200)
2. GET the config to confirm changes were saved
3. Some settings only affect new requests/connections

### JSON Parsing Errors

**Issue**: `400 Bad Request` with JSON error.

**Solution**:
1. Validate JSON with a linter: `jq . < your-config.json`
2. Ensure all required fields are present
3. Check field types match the schema

### Permission Denied

**Issue**: Cannot read template directory after changing `template_dir`.

**Solution**:
1. Ensure the directory exists
2. Check file permissions
3. Use absolute paths or paths relative to server working directory

---

## Monitoring and Observability

### Health Check with Config Validation

```bash
#!/bin/bash
# health-check.sh

CONFIG=$(curl -s http://localhost:8080/api/config)

if [ $? -eq 0 ]; then
  LEVEL=$(echo $CONFIG | jq -r '.logging.level')
  CORS=$(echo $CONFIG | jq -r '.http.enable_cors')
  
  echo "✓ API accessible"
  echo "  Log level: $LEVEL"
  echo "  CORS: $CORS"
else
  echo "✗ API not accessible"
  exit 1
fi
```

### Configuration Drift Detection

```bash
# Compare running config with baseline
diff <(curl -s http://localhost:8080/api/config | jq -S .) \
     <(jq -S . < baseline-config.json)
```

---

## Security Considerations

⚠️ **Warning**: This API has no authentication or authorization.

### Recommendations for Production:

1. **Use Reverse Proxy with Auth**
   ```nginx
   location /api/config {
       auth_basic "Restricted";
       auth_basic_user_file /etc/nginx/.htpasswd;
       proxy_pass http://localhost:8080;
   }
   ```

2. **Firewall Rules**
   ```bash
   # Only allow from specific IPs
   iptables -A INPUT -p tcp --dport 8080 -s 10.0.0.0/8 -j ACCEPT
   iptables -A INPUT -p tcp --dport 8080 -j DROP
   ```

3. **Network Isolation**
   - Bind to `127.0.0.1` only
   - Use VPN/SSH tunnel for remote access

4. **Audit Logging**
   - Log all configuration changes
   - Implement change approval workflow

---

## Version History

- **v1.0** (2026-01-07) - Initial release
  - GET /api/config
  - PUT /api/config
  - Support for logging, http, and resources configuration
