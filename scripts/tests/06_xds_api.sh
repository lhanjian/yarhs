#!/bin/bash
# xDS API Tests
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "6. xDS API"

# Snapshot endpoint - verify it has version_info field
RESPONSE=$(curl -s "$API_URL/v1/discovery")
assert_json_has "discovery snapshot endpoint" "$RESPONSE" ".version_info"

# Resource endpoints - verify structure with jq
RESPONSE=$(curl -s "$API_URL/v1/discovery:routes")
assert_json_has "routes resource endpoint" "$RESPONSE" ".resources"

RESPONSE=$(curl -s "$API_URL/v1/discovery:logging")
assert_json_has "logging resource endpoint" "$RESPONSE" ".resources[0].level"

RESPONSE=$(curl -s "$API_URL/v1/discovery:listeners")
assert_json_has "listeners resource endpoint" "$RESPONSE" ".resources[0].main_server"
