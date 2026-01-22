#!/bin/bash
# xDS API Tests
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "6. xDS API"

# Snapshot endpoint
RESPONSE=$(curl -s "$API_URL/v1/discovery")
assert_contains "discovery snapshot endpoint" "$RESPONSE" "version_info"

# Resource endpoints
RESPONSE=$(curl -s "$API_URL/v1/discovery:routes")
assert_contains "routes resource endpoint" "$RESPONSE" "resources"

RESPONSE=$(curl -s "$API_URL/v1/discovery:logging")
assert_contains "logging resource endpoint" "$RESPONSE" "level"

RESPONSE=$(curl -s "$API_URL/v1/discovery:listeners")
assert_contains "listeners resource endpoint" "$RESPONSE" "main_server"
