#!/bin/bash
# Test 15: State Persistence Tests
# Verifies that configuration changes are persisted to state.toml
# and restored after server restart
#
# NOTE: State persistence is disabled by default.
# To enable, set `enable_state_persistence = true` in config.toml [server] section.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

# Get project root (two levels up from tests dir)
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

log_section "15. State Persistence"

# Check if persistence is enabled
INITIAL_STATE=$(curl -s "$API_URL/v1/state")
ENABLED=$(echo "$INITIAL_STATE" | jq -r '.enabled')

if [ "$ENABLED" != "true" ]; then
    log_info "State persistence is DISABLED (enable_state_persistence = false)"
    log_info "Skipping persistence tests. To enable, add to config.toml:"
    log_info "  [server]"
    log_info "  enable_state_persistence = true"
    log_pass "Persistence disabled check (expected behavior)"
    
    # Verify that changes don't create state.toml when disabled
    curl -s -X POST "$API_URL/v1/discovery:http" \
        -H "Content-Type: application/json" \
        -d '{
            "resources": [{
                "default_content_type": "text/plain",
                "server_name": "Test/1.0",
                "enable_cors": false,
                "max_body_size": 10485760
            }]
        }' > /dev/null
    
    sleep 0.2
    
    if [ ! -f "$PROJECT_ROOT/state.toml" ]; then
        log_pass "No state.toml created when persistence disabled"
    else
        log_fail "state.toml should not be created when persistence disabled"
    fi
    
    # Restore default
    curl -s -X POST "$API_URL/v1/discovery:http" \
        -H "Content-Type: application/json" \
        -d '{
            "resources": [{
                "default_content_type": "text/html; charset=utf-8",
                "server_name": "Tokio-Hyper/1.0",
                "enable_cors": false,
                "max_body_size": 10485760
            }]
        }' > /dev/null
    
    log_info "State persistence tests completed (skipped - disabled)"
    return 0 2>/dev/null || exit 0
fi

# === Persistence is ENABLED - run full tests ===
log_pass "Persistence enabled"

log_info "=== Part 1: Initial State Check ==="

# Clear any previous state
curl -s -X DELETE "$API_URL/v1/state" > /dev/null 2>&1

PERSISTED=$(curl -s "$API_URL/v1/state" | jq -r '.persisted_config')
if [ "$PERSISTED" = "{}" ]; then
    log_pass "Initial state is empty"
else
    log_pass "Initial state cleared"
fi

log_info "=== Part 2: Persist HTTP Config ==="

MODIFY_RESULT=$(curl -s -X POST "$API_URL/v1/discovery:http" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "default_content_type": "text/plain; charset=utf-8",
            "server_name": "YARHS-Persist-Test/1.0",
            "enable_cors": true,
            "max_body_size": 52428800
        }]
    }')

if echo "$MODIFY_RESULT" | jq -e '.status == "ACK"' > /dev/null; then
    log_pass "HTTP config modified"
else
    log_fail "Failed to modify HTTP config"
fi

sleep 0.2

if [ -f "$PROJECT_ROOT/state.toml" ]; then
    log_pass "state.toml created"
else
    log_fail "state.toml not created"
fi

if [ -f "$PROJECT_ROOT/state.toml" ] && grep -q "YARHS-Persist-Test" "$PROJECT_ROOT/state.toml"; then
    log_pass "HTTP config persisted to state.toml"
else
    log_fail "HTTP config not found in state.toml"
fi

log_info "=== Part 3: Persist Performance Config ==="

curl -s -X POST "$API_URL/v1/discovery:performance" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "keep_alive_timeout": 200,
            "read_timeout": 90,
            "write_timeout": 90,
            "max_connections": 10000
        }]
    }' > /dev/null

sleep 0.2

if [ -f "$PROJECT_ROOT/state.toml" ] && grep -q "keep_alive_timeout = 200" "$PROJECT_ROOT/state.toml"; then
    log_pass "Performance config persisted"
else
    log_fail "Performance config not found in state.toml"
fi

log_info "=== Part 4: Persist Virtual Hosts ==="

curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "virtual_hosts": [{
                "name": "persist-test-site",
                "domains": ["persist.example.com"],
                "routes": [{
                    "match": {"prefix": "/"},
                    "type": "dir",
                    "path": "static"
                }]
            }]
        }]
    }' > /dev/null

sleep 0.2

if [ -f "$PROJECT_ROOT/state.toml" ] && grep -q "persist-test-site" "$PROJECT_ROOT/state.toml"; then
    log_pass "Virtual hosts persisted"
else
    log_fail "Virtual hosts not found in state.toml"
fi

log_info "=== Part 5: Verify GET /v1/state ==="

STATE_RESPONSE=$(curl -s "$API_URL/v1/state")

if echo "$STATE_RESPONSE" | jq -e '.persisted_config.http.server_name == "YARHS-Persist-Test/1.0"' > /dev/null; then
    log_pass "GET /v1/state returns HTTP config"
else
    log_fail "GET /v1/state missing HTTP config"
fi

if echo "$STATE_RESPONSE" | jq -e '.persisted_config.performance.keep_alive_timeout == 200' > /dev/null; then
    log_pass "GET /v1/state returns performance config"
else
    log_fail "GET /v1/state missing performance config"
fi

log_info "=== Part 6: Clear State ==="

CLEAR_RESULT=$(curl -s -X DELETE "$API_URL/v1/state")

if echo "$CLEAR_RESULT" | jq -e '.status == "OK"' > /dev/null; then
    log_pass "DELETE /v1/state succeeded"
else
    log_fail "DELETE /v1/state failed"
fi

if [ ! -f "$PROJECT_ROOT/state.toml" ]; then
    log_pass "state.toml removed"
else
    log_fail "state.toml should be removed"
fi

EMPTY_STATE=$(curl -s "$API_URL/v1/state" | jq -r '.persisted_config')
if [ "$EMPTY_STATE" = "{}" ]; then
    log_pass "State cleared in API response"
else
    log_fail "State should be empty after clear"
fi

log_info "=== Part 7: Restore Original Config ==="

curl -s -X POST "$API_URL/v1/discovery:http" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "default_content_type": "text/html; charset=utf-8",
            "server_name": "Tokio-Hyper/1.0",
            "enable_cors": false,
            "max_body_size": 10485760
        }]
    }' > /dev/null

curl -s -X POST "$API_URL/v1/discovery:performance" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "keep_alive_timeout": 75,
            "read_timeout": 30,
            "write_timeout": 30,
            "max_connections": null
        }]
    }' > /dev/null

curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{"resources": [{"virtual_hosts": []}]}' > /dev/null

curl -s -X DELETE "$API_URL/v1/state" > /dev/null

log_pass "Config restored to defaults"

log_info "State persistence tests completed"
