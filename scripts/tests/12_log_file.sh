#!/bin/bash
# Log File Output Tests
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "12. Log File Output"

# Create temp directory for log files
LOG_DIR="/tmp/yarhs_logs_$$"
mkdir -p "$LOG_DIR"
ACCESS_LOG="$LOG_DIR/access.log"
ERROR_LOG="$LOG_DIR/error.log"

# Test 1: Configure access log file via API
log_info "Testing log file configuration via API..."

UPDATE_RESPONSE=$(curl -s -X POST "$API_URL/v1/discovery:logging" \
    -H "Content-Type: application/json" \
    -d "{
        \"resources\": [{
            \"level\": \"info\",
            \"access_log\": true,
            \"show_headers\": false,
            \"access_log_format\": \"combined\",
            \"access_log_file\": \"$ACCESS_LOG\",
            \"error_log_file\": \"$ERROR_LOG\"
        }]
    }")

assert_json_field "Configure log files ACK" "$UPDATE_RESPONSE" ".status" "ACK"

sleep 0.2

# Test 2: Verify log file was created
if [ -f "$ACCESS_LOG" ]; then
    log_pass "Access log file created: $ACCESS_LOG"
else
    log_fail "Access log file not created"
fi

# Test 3: Generate some requests to write to log
curl -s "$BASE_URL/test.txt" > /dev/null
curl -s "$BASE_URL/index.html" > /dev/null
curl -s "$BASE_URL/nonexistent" > /dev/null

sleep 0.3

# Test 4: Verify access log contains entries
if [ -f "$ACCESS_LOG" ] && [ -s "$ACCESS_LOG" ]; then
    ACCESS_LINES=$(wc -l < "$ACCESS_LOG")
    if [ "$ACCESS_LINES" -ge 3 ]; then
        log_pass "Access log has entries: $ACCESS_LINES lines"
    else
        log_fail "Access log has insufficient entries (expected >= 3, got: $ACCESS_LINES)"
    fi
else
    log_fail "Access log is empty or missing"
fi

# Test 5: Verify access log format (combined format has specific fields)
if [ -f "$ACCESS_LOG" ]; then
    # Check for combined log format pattern: IP - - [timestamp] "METHOD /path HTTP/1.1" status bytes "referer" "user-agent"
    if grep -q 'GET /test.txt HTTP' "$ACCESS_LOG"; then
        log_pass "Access log contains request entries"
    else
        log_fail "Access log format incorrect"
        head -3 "$ACCESS_LOG"
    fi
fi

# Test 6: Verify API config reflects log file paths
LOGGING_CONFIG=$(curl -s "$API_URL/v1/discovery:logging")
STORED_ACCESS=$(echo "$LOGGING_CONFIG" | jq -r '.resources[0].access_log_file // empty')
if [ "$STORED_ACCESS" = "$ACCESS_LOG" ]; then
    log_pass "API shows access_log_file path"
else
    log_fail "API access_log_file path mismatch (expected: $ACCESS_LOG, got: $STORED_ACCESS)"
fi

# Test 7: Change to JSON format and verify log output
curl -s -X POST "$API_URL/v1/discovery:logging" \
    -H "Content-Type: application/json" \
    -d "{
        \"resources\": [{
            \"level\": \"info\",
            \"access_log\": true,
            \"show_headers\": false,
            \"access_log_format\": \"json\",
            \"access_log_file\": \"$ACCESS_LOG\"
        }]
    }" > /dev/null

sleep 0.2

# Make a request with JSON format
curl -s "$BASE_URL/style.css" > /dev/null
sleep 0.2

# Verify JSON format entry appears
if tail -1 "$ACCESS_LOG" | jq -e '.method' > /dev/null 2>&1; then
    log_pass "JSON format log entry written"
else
    log_fail "JSON format log entry not found"
    tail -1 "$ACCESS_LOG"
fi

# Test 8: Restore to stdout (null file path)
curl -s -X POST "$API_URL/v1/discovery:logging" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "level": "info",
            "access_log": true,
            "show_headers": false,
            "access_log_format": "combined",
            "access_log_file": null,
            "error_log_file": null
        }]
    }' > /dev/null

sleep 0.2

# Verify config shows null paths
LOGGING_CONFIG=$(curl -s "$API_URL/v1/discovery:logging")
RESTORED_PATH=$(echo "$LOGGING_CONFIG" | jq -r '.resources[0].access_log_file // "null"')
if [ "$RESTORED_PATH" = "null" ]; then
    log_pass "Restored to stdout (null path)"
else
    log_fail "Failed to restore to stdout (got: $RESTORED_PATH)"
fi

# Cleanup
rm -rf "$LOG_DIR"

log_info "Log file output tests completed"
