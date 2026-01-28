#!/bin/bash
# Health Check Endpoint Tests
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "10. Health Check Endpoints"

# Test liveness endpoint
assert_status "Liveness probe /healthz" "$BASE_URL/healthz" "200"

# Verify response body
HEALTH_BODY=$(curl -s "$BASE_URL/healthz")
if [ "$HEALTH_BODY" = "ok" ]; then
    log_pass "Liveness response body: ok"
else
    log_fail "Liveness response body (expected: ok, got: $HEALTH_BODY)"
fi

# Verify no-cache headers
CACHE_HEADER=$(curl -sI "$BASE_URL/healthz" | grep -i "cache-control" | tr -d '\r')
assert_contains "Health check Cache-Control" "$CACHE_HEADER" "no-cache"

# Test readiness endpoint
assert_status "Readiness probe /readyz" "$BASE_URL/readyz" "200"

# Verify readiness response
READY_BODY=$(curl -s "$BASE_URL/readyz")
if [ "$READY_BODY" = "ok" ]; then
    log_pass "Readiness response body: ok"
else
    log_fail "Readiness response body (expected: ok, got: $READY_BODY)"
fi

# Test HEAD method on health endpoints
HEAD_STATUS=$(curl -sI -X HEAD "$BASE_URL/healthz" | head -1 | awk '{print $2}')
if [ "$HEAD_STATUS" = "200" ]; then
    log_pass "HEAD /healthz returns 200"
else
    log_fail "HEAD /healthz (expected: 200, got: $HEAD_STATUS)"
fi

# Test dynamic health config update via API - custom paths
log_info "Testing health config dynamic update..."

# Update to custom paths
CUSTOM_RESPONSE=$(curl -s -X POST "$API_URL/v1/discovery:routes" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "index_files": ["index.html"],
            "custom_routes": {
                "/static": {"type": "dir", "path": "static"}
            },
            "health": {
                "enabled": true,
                "liveness_path": "/health/live",
                "readiness_path": "/health/ready"
            }
        }]
    }')

assert_json_field "Custom health paths ACK" "$CUSTOM_RESPONSE" ".status" "ACK"

sleep 0.2

# Verify custom paths work
assert_status "Custom liveness /health/live" "$BASE_URL/health/live" "200"
assert_status "Custom readiness /health/ready" "$BASE_URL/health/ready" "200"

# Verify custom path response body
CUSTOM_BODY=$(curl -s "$BASE_URL/health/live")
if [ "$CUSTOM_BODY" = "ok" ]; then
    log_pass "Custom path response body: ok"
else
    log_fail "Custom path response body (expected: ok, got: $CUSTOM_BODY)"
fi

# Restore default config
curl -s -X POST "$API_URL/v1/discovery:routes" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "index_files": ["index.html", "index.htm"],
            "custom_routes": {
                "/static": {"type": "dir", "path": "static"},
                "/about": {"type": "file", "path": "templates/about.html"},
                "/home": {"type": "file", "path": "templates/index.html"}
            },
            "health": {
                "enabled": true,
                "liveness_path": "/healthz",
                "readiness_path": "/readyz"
            }
        }]
    }' > /dev/null

sleep 0.2

# Verify default paths restored
assert_status "Restored /healthz" "$BASE_URL/healthz" "200"

log_info "Health check tests completed"
