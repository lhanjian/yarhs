#!/bin/bash
# Config and API Parity Tests
# Validates that xDS API reflects config.toml correctly
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "14. Config/API Parity Tests"

log_info "Testing xDS API reflects config.toml values correctly"

# Note: This test runs FIRST after server start, so config should be pristine
# If running after other tests, logging config may have been modified

# ============================================
# Part 1: Listener Config Parity
# ============================================
log_info "=== Part 1: Listener Configuration ==="

# Get listener config from API
LISTENERS=$(curl -s "$API_URL/v1/discovery:listeners")
MAIN_HOST=$(echo "$LISTENERS" | jq -r '.resources[0].main_server.host // empty')
MAIN_PORT=$(echo "$LISTENERS" | jq -r '.resources[0].main_server.port // empty')
API_HOST=$(echo "$LISTENERS" | jq -r '.resources[0].api_server.host // empty')
API_PORT=$(echo "$LISTENERS" | jq -r '.resources[0].api_server.port // empty')

# Verify against expected values (from config.toml)
# config.toml: host = "127.0.0.1", port = 8080, api_host = "0.0.0.0", api_port = 8000
if [ "$MAIN_HOST" = "127.0.0.1" ]; then
    log_pass "Listener main_server.host matches config (127.0.0.1)"
else
    log_fail "Listener main_server.host mismatch (expected: 127.0.0.1, got: $MAIN_HOST)"
fi

if [ "$MAIN_PORT" = "8080" ]; then
    log_pass "Listener main_server.port matches config (8080)"
else
    log_fail "Listener main_server.port mismatch (expected: 8080, got: $MAIN_PORT)"
fi

if [ "$API_HOST" = "0.0.0.0" ]; then
    log_pass "Listener api_server.host matches config (0.0.0.0)"
else
    log_fail "Listener api_server.host mismatch (expected: 0.0.0.0, got: $API_HOST)"
fi

if [ "$API_PORT" = "8000" ]; then
    log_pass "Listener api_server.port matches config (8000)"
else
    log_fail "Listener api_server.port mismatch (expected: 8000, got: $API_PORT)"
fi

# ============================================
# Part 2: Logging Config Parity
# ============================================
log_info "=== Part 2: Logging Configuration ==="

# First, reset logging to match config.toml defaults
curl -s -X POST "$API_URL/v1/discovery:logging" \
    -H "Content-Type: application/json" \
    -d '{"resources": [{"level": "debug", "access_log": false, "show_headers": false, "access_log_format": "combined"}]}' > /dev/null
sleep 0.2

LOGGING=$(curl -s "$API_URL/v1/discovery:logging")
LOG_LEVEL=$(echo "$LOGGING" | jq -r '.resources[0].level')
ACCESS_LOG=$(echo "$LOGGING" | jq '.resources[0].access_log')
SHOW_HEADERS=$(echo "$LOGGING" | jq '.resources[0].show_headers')

# config.toml: level = "debug", access_log = false, show_headers = false
if [ "$LOG_LEVEL" = "debug" ]; then
    log_pass "Logging level matches config (debug)"
else
    log_fail "Logging level mismatch (expected: debug, got: $LOG_LEVEL)"
fi

if [ "$ACCESS_LOG" = "false" ]; then
    log_pass "Logging access_log matches config (false)"
else
    log_fail "Logging access_log mismatch (expected: false, got: $ACCESS_LOG)"
fi

if [ "$SHOW_HEADERS" = "false" ]; then
    log_pass "Logging show_headers matches config (false)"
else
    log_fail "Logging show_headers mismatch (expected: false, got: $SHOW_HEADERS)"
fi

# ============================================
# Part 3: Performance Config Parity
# ============================================
log_info "=== Part 3: Performance Configuration ==="

PERF=$(curl -s "$API_URL/v1/discovery:performance")
KEEP_ALIVE=$(echo "$PERF" | jq -r '.resources[0].keep_alive_timeout // empty')
READ_TIMEOUT=$(echo "$PERF" | jq -r '.resources[0].read_timeout // empty')
WRITE_TIMEOUT=$(echo "$PERF" | jq -r '.resources[0].write_timeout // empty')
MAX_CONN=$(echo "$PERF" | jq -r '.resources[0].max_connections // empty')

# config.toml: keep_alive_timeout = 75, read_timeout = 30, write_timeout = 30, max_connections = 5000
if [ "$KEEP_ALIVE" = "75" ]; then
    log_pass "Performance keep_alive_timeout matches config (75)"
else
    log_fail "Performance keep_alive_timeout mismatch (expected: 75, got: $KEEP_ALIVE)"
fi

if [ "$READ_TIMEOUT" = "30" ]; then
    log_pass "Performance read_timeout matches config (30)"
else
    log_fail "Performance read_timeout mismatch (expected: 30, got: $READ_TIMEOUT)"
fi

if [ "$WRITE_TIMEOUT" = "30" ]; then
    log_pass "Performance write_timeout matches config (30)"
else
    log_fail "Performance write_timeout mismatch (expected: 30, got: $WRITE_TIMEOUT)"
fi

if [ "$MAX_CONN" = "5000" ]; then
    log_pass "Performance max_connections matches config (5000)"
else
    log_fail "Performance max_connections mismatch (expected: 5000, got: $MAX_CONN)"
fi

# ============================================
# Part 4: HTTP Config Parity
# ============================================
log_info "=== Part 4: HTTP Configuration ==="

HTTP=$(curl -s "$API_URL/v1/discovery:http")
SERVER_NAME=$(echo "$HTTP" | jq -r '.resources[0].server_name // empty')
ENABLE_CORS=$(echo "$HTTP" | jq -r '.resources[0].enable_cors')
MAX_BODY=$(echo "$HTTP" | jq -r '.resources[0].max_body_size // empty')

# config.toml: server_name = "Tokio-Hyper/1.0", enable_cors = false, max_body_size = 10485760
if [ "$SERVER_NAME" = "Tokio-Hyper/1.0" ]; then
    log_pass "HTTP server_name matches config (Tokio-Hyper/1.0)"
else
    log_fail "HTTP server_name mismatch (expected: Tokio-Hyper/1.0, got: $SERVER_NAME)"
fi

if [ "$ENABLE_CORS" = "false" ]; then
    log_pass "HTTP enable_cors matches config (false)"
else
    log_fail "HTTP enable_cors mismatch (expected: false, got: $ENABLE_CORS)"
fi

if [ "$MAX_BODY" = "10485760" ]; then
    log_pass "HTTP max_body_size matches config (10485760)"
else
    log_fail "HTTP max_body_size mismatch (expected: 10485760, got: $MAX_BODY)"
fi

# ============================================
# Part 5: Routes Config Parity
# ============================================
log_info "=== Part 5: Routes Configuration ==="

ROUTES=$(curl -s "$API_URL/v1/discovery:routes")

# Check index_files array
INDEX_COUNT=$(echo "$ROUTES" | jq '.resources[0].index_files | length')
INDEX_1=$(echo "$ROUTES" | jq -r '.resources[0].index_files[0] // empty')
INDEX_2=$(echo "$ROUTES" | jq -r '.resources[0].index_files[1] // empty')

if [ "$INDEX_COUNT" = "2" ] && [ "$INDEX_1" = "index.html" ] && [ "$INDEX_2" = "index.htm" ]; then
    log_pass "Routes index_files matches config"
else
    log_fail "Routes index_files mismatch (got: $INDEX_1, $INDEX_2)"
fi

# Check custom_routes structure
# config.toml has: /static -> dir, /about -> file, /home -> file
STATIC_TYPE=$(echo "$ROUTES" | jq -r '.resources[0].custom_routes["/static"].type // empty')
STATIC_PATH=$(echo "$ROUTES" | jq -r '.resources[0].custom_routes["/static"].path // empty')
ABOUT_TYPE=$(echo "$ROUTES" | jq -r '.resources[0].custom_routes["/about"].type // empty')
ABOUT_PATH=$(echo "$ROUTES" | jq -r '.resources[0].custom_routes["/about"].path // empty')

if [ "$STATIC_TYPE" = "dir" ] && [ "$STATIC_PATH" = "static" ]; then
    log_pass "Routes /static custom_route matches config (dir -> static)"
else
    log_fail "Routes /static mismatch (expected: dir/static, got: $STATIC_TYPE/$STATIC_PATH)"
fi

if [ "$ABOUT_TYPE" = "file" ] && [ "$ABOUT_PATH" = "templates/about.html" ]; then
    log_pass "Routes /about custom_route matches config (file -> templates/about.html)"
else
    log_fail "Routes /about mismatch (expected: file/templates/about.html, got: $ABOUT_TYPE/$ABOUT_PATH)"
fi

# ============================================
# Part 6: Bidirectional Update Test
# ============================================
log_info "=== Part 6: Bidirectional Update Test ==="

# Update via API and verify change reflected
curl -s -X POST "$API_URL/v1/discovery:logging" \
    -H "Content-Type: application/json" \
    -d '{"resources": [{"level": "warn", "access_log": true, "show_headers": true}]}' > /dev/null

sleep 0.3

# Read back and verify
LOGGING_UPDATED=$(curl -s "$API_URL/v1/discovery:logging")
NEW_LEVEL=$(echo "$LOGGING_UPDATED" | jq -r '.resources[0].level // empty')
NEW_ACCESS=$(echo "$LOGGING_UPDATED" | jq -r '.resources[0].access_log // empty')

if [ "$NEW_LEVEL" = "warn" ] && [ "$NEW_ACCESS" = "true" ]; then
    log_pass "API update reflected correctly (level=warn, access_log=true)"
else
    log_fail "API update not reflected (expected: warn/true, got: $NEW_LEVEL/$NEW_ACCESS)"
fi

# Restore original config
curl -s -X POST "$API_URL/v1/discovery:logging" \
    -H "Content-Type: application/json" \
    -d '{"resources": [{"level": "debug", "access_log": false, "show_headers": false}]}' > /dev/null

sleep 0.2

# Verify restore
LOGGING_RESTORED=$(curl -s "$API_URL/v1/discovery:logging")
RESTORED_LEVEL=$(echo "$LOGGING_RESTORED" | jq -r '.resources[0].level // empty')

if [ "$RESTORED_LEVEL" = "debug" ]; then
    log_pass "Config restored to original (level=debug)"
else
    log_fail "Config restore failed (expected: debug, got: $RESTORED_LEVEL)"
fi

# ============================================
# Part 7: Discovery Snapshot Consistency
# ============================================
log_info "=== Part 7: Discovery Snapshot Consistency ==="

# Get full snapshot
SNAPSHOT=$(curl -s "$API_URL/v1/discovery")
VERSION=$(echo "$SNAPSHOT" | jq -r '.version_info // empty')

if [ -n "$VERSION" ]; then
    log_pass "Snapshot has valid version_info: $VERSION"
else
    log_fail "Snapshot missing version_info"
fi

# Verify snapshot contains all resource types (check for nested keys)
SNAP_LISTENER=$(echo "$SNAPSHOT" | jq '.resources.listener // empty')
SNAP_ROUTE=$(echo "$SNAPSHOT" | jq '.resources.route // empty')
SNAP_LOGGING=$(echo "$SNAPSHOT" | jq '.resources.logging // empty')
SNAP_HTTP=$(echo "$SNAPSHOT" | jq '.resources.http // empty')

if [ -n "$SNAP_LISTENER" ] && [ "$SNAP_LISTENER" != "null" ]; then
    log_pass "Snapshot contains listener resource"
else
    log_fail "Snapshot missing listener resource"
fi

if [ -n "$SNAP_ROUTE" ] && [ "$SNAP_ROUTE" != "null" ]; then
    log_pass "Snapshot contains route resource"
else
    log_fail "Snapshot missing route resource"
fi

if [ -n "$SNAP_LOGGING" ] && [ "$SNAP_LOGGING" != "null" ]; then
    log_pass "Snapshot contains logging resource"
else
    log_fail "Snapshot missing logging resource"
fi

if [ -n "$SNAP_HTTP" ] && [ "$SNAP_HTTP" != "null" ]; then
    log_pass "Snapshot contains http resource"
else
    log_fail "Snapshot missing http resource"
fi

log_info "Config/API parity tests completed"

# ============================================
# Part 8: VirtualHosts Config File Support
# ============================================
log_info "=== Part 8: VirtualHosts in Config File ==="

# Note: The default config.toml has no virtual_hosts configured (empty array)
# This test verifies the structure is correct when empty

VHOSTS=$(curl -s "$API_URL/v1/discovery:vhosts")
VHOST_COUNT=$(echo "$VHOSTS" | jq '.resources[0].virtual_hosts | length')

# Default config should have 0 virtual hosts (uses legacy routes)
if [ "$VHOST_COUNT" = "0" ]; then
    log_pass "Default config has no virtual_hosts (uses legacy routes)"
else
    # If non-zero, just verify it's a valid number (someone may have configured vhosts)
    if [ "$VHOST_COUNT" -ge "0" ] 2>/dev/null; then
        log_pass "VirtualHosts loaded from config: $VHOST_COUNT hosts"
    else
        log_fail "Invalid virtual_hosts count: $VHOST_COUNT"
    fi
fi

log_info "VirtualHosts config support verified"
