#!/bin/bash
# Access Log Format Tests
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "11. Access Log Formats"

# Helper function to extract access_log_format from JSON response using jq
get_log_format() {
    echo "$1" | jq -r '.resources[0].access_log_format // empty'
}

# Test default combined format by checking API config
LOGGING_CONFIG=$(curl -s "$API_URL/v1/discovery:logging")
DEFAULT_FORMAT=$(get_log_format "$LOGGING_CONFIG")
if [ "$DEFAULT_FORMAT" = "combined" ]; then
    log_pass "Default access log format: combined"
else
    log_fail "Default access log format (expected: combined, got: $DEFAULT_FORMAT)"
fi

# Test changing to common format via API
UPDATE_RESPONSE=$(curl -s -X POST "$API_URL/v1/discovery:logging" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "level": "info",
            "access_log": true,
            "show_headers": false,
            "access_log_format": "common"
        }]
    }')

assert_json_field "Update to common format ACK" "$UPDATE_RESPONSE" ".status" "ACK"

sleep 0.2

# Verify the format was updated
LOGGING_CONFIG=$(curl -s "$API_URL/v1/discovery:logging")
NEW_FORMAT=$(get_log_format "$LOGGING_CONFIG")
if [ "$NEW_FORMAT" = "common" ]; then
    log_pass "Format updated to: common"
else
    log_fail "Format updated to common (expected: common, got: $NEW_FORMAT)"
fi

# Test changing to json format
UPDATE_RESPONSE=$(curl -s -X POST "$API_URL/v1/discovery:logging" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "level": "info",
            "access_log": true,
            "show_headers": false,
            "access_log_format": "json"
        }]
    }')

assert_json_field "Update to json format ACK" "$UPDATE_RESPONSE" ".status" "ACK"

sleep 0.2

LOGGING_CONFIG=$(curl -s "$API_URL/v1/discovery:logging")
NEW_FORMAT=$(get_log_format "$LOGGING_CONFIG")
if [ "$NEW_FORMAT" = "json" ]; then
    log_pass "Format updated to: json"
else
    log_fail "Format updated to json (expected: json, got: $NEW_FORMAT)"
fi

# Test custom format
CUSTOM_FORMAT='$remote_addr - $status - $request_time'
UPDATE_RESPONSE=$(curl -s -X POST "$API_URL/v1/discovery:logging" \
    -H "Content-Type: application/json" \
    -d "{
        \"resources\": [{
            \"level\": \"info\",
            \"access_log\": true,
            \"show_headers\": false,
            \"access_log_format\": \"$CUSTOM_FORMAT\"
        }]
    }")

assert_json_field "Update to custom format ACK" "$UPDATE_RESPONSE" ".status" "ACK"

sleep 0.2

LOGGING_CONFIG=$(curl -s "$API_URL/v1/discovery:logging")
# Custom format should be stored correctly - check via jq
STORED_FORMAT=$(echo "$LOGGING_CONFIG" | jq -r '.resources[0].access_log_format // empty')
if [[ "$STORED_FORMAT" == *'remote_addr'* ]]; then
    log_pass "Custom format stored correctly"
else
    log_fail "Custom format not stored (got: $STORED_FORMAT)"
fi

# Restore default config
curl -s -X POST "$API_URL/v1/discovery:logging" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "level": "info",
            "access_log": true,
            "show_headers": false,
            "access_log_format": "combined"
        }]
    }' > /dev/null

sleep 0.2

# Verify restored
LOGGING_CONFIG=$(curl -s "$API_URL/v1/discovery:logging")
RESTORED_FORMAT=$(get_log_format "$LOGGING_CONFIG")
if [ "$RESTORED_FORMAT" = "combined" ]; then
    log_pass "Restored default format: combined"
else
    log_fail "Restored default format (expected: combined, got: $RESTORED_FORMAT)"
fi

log_info "Access log format tests completed"
