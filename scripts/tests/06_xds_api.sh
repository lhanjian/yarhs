#!/bin/bash
# xDS API Tests
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "6. xDS API"

# Snapshot endpoint - verify it has version_info field
RESPONSE=$(curl -s "$API_URL/v1/discovery")
assert_json_has "discovery snapshot endpoint" "$RESPONSE" ".version_info"

# Verify version_info contains expected structure
VERSION=$(echo "$RESPONSE" | jq -r '.version_info // empty')
if [ -n "$VERSION" ]; then
    log_pass "discovery endpoint returns valid version_info: $VERSION"
else
    log_fail "discovery endpoint version_info is empty"
fi

# Resource endpoints - verify structure with jq
RESPONSE=$(curl -s "$API_URL/v1/discovery:routes")
assert_json_has "routes resource endpoint" "$RESPONSE" ".resources"

# Verify routes resources is an array
ROUTES_COUNT=$(echo "$RESPONSE" | jq '.resources | length')
if [ "$ROUTES_COUNT" -ge 0 ]; then
    log_pass "routes endpoint returns array with $ROUTES_COUNT routes"
else
    log_fail "routes endpoint did not return valid array"
fi

RESPONSE=$(curl -s "$API_URL/v1/discovery:logging")
assert_json_has "logging resource endpoint" "$RESPONSE" ".resources[0].level"

# Verify logging level is a valid value
LOG_LEVEL=$(echo "$RESPONSE" | jq -r '.resources[0].level // empty')
if [[ "$LOG_LEVEL" =~ ^(trace|debug|info|warn|error)$ ]]; then
    log_pass "logging endpoint returns valid level: $LOG_LEVEL"
else
    log_fail "logging endpoint returned invalid level: $LOG_LEVEL"
fi

RESPONSE=$(curl -s "$API_URL/v1/discovery:listeners")
assert_json_has "listeners resource endpoint" "$RESPONSE" ".resources[0].main_server"

# Verify listener has valid port configuration
MAIN_PORT=$(echo "$RESPONSE" | jq -r '.resources[0].main_server.port // empty')
if [ -n "$MAIN_PORT" ] && [ "$MAIN_PORT" -gt 0 ]; then
    log_pass "listeners endpoint returns valid port: $MAIN_PORT"
else
    log_fail "listeners endpoint did not return valid port"
fi
