#!/bin/bash
# HTTP Method Handling Tests
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "3. HTTP Method Handling"

# GET (implicit, verified via status code)
assert_status "GET method" "$BASE_URL/" "200"

# HEAD
HEAD_STATUS=$(curl -sI "$BASE_URL/" | grep "HTTP" | cut -d' ' -f2 | tr -d '\r')
HEAD_LENGTH=$(curl -sI "$BASE_URL/" | grep -i "content-length" | tr -d '\r')
if [ "$HEAD_STATUS" = "200" ] && [ -n "$HEAD_LENGTH" ]; then
    log_pass "HEAD method (HTTP 200 + Content-Length)"
else
    log_fail "HEAD method"
fi

# OPTIONS
assert_status "OPTIONS method" "$BASE_URL/" "204" "-X OPTIONS"
ALLOW=$(curl -sI -X OPTIONS "$BASE_URL/" | grep -i "allow:" | tr -d '\r')
assert_contains "OPTIONS Allow header contains GET" "$ALLOW" "GET"
assert_contains "OPTIONS Allow header contains HEAD" "$ALLOW" "HEAD"

# Disallowed methods
assert_status "POST returns 405" "$BASE_URL/" "405" "-X POST"
assert_status "PUT returns 405" "$BASE_URL/" "405" "-X PUT"
assert_status "DELETE returns 405" "$BASE_URL/" "405" "-X DELETE"
