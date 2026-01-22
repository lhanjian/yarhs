#!/bin/bash
# Caching and Conditional Request Tests
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "4. Caching and Conditional Requests"

# ETag
ETAG=$(curl -sI "$BASE_URL/static/test.txt" | grep -i "etag:" | cut -d' ' -f2 | tr -d '\r')
if [ -n "$ETAG" ]; then
    log_pass "ETag response header: $ETAG"
else
    log_fail "ETag response header missing"
fi

# 304 Not Modified (If-None-Match)
assert_status "If-None-Match match returns 304" "$BASE_URL/static/test.txt" "304" "If-None-Match: $ETAG"

# ETag mismatch
assert_status "If-None-Match mismatch returns 200" "$BASE_URL/static/test.txt" "200" 'If-None-Match: "wrongetag"'

# Cache-Control
CACHE=$(curl -sI "$BASE_URL/static/test.txt" | grep -i "cache-control" | tr -d '\r')
assert_contains "Cache-Control header" "$CACHE" "max-age"

# Last-Modified header
LAST_MODIFIED=$(curl -sI "$BASE_URL/static/test.txt" | grep -i "last-modified:" | cut -d' ' -f2- | tr -d '\r')
if [ -n "$LAST_MODIFIED" ]; then
    log_pass "Last-Modified response header: $LAST_MODIFIED"
else
    log_fail "Last-Modified response header missing"
fi

# 304 Not Modified (If-Modified-Since)
assert_status "If-Modified-Since match returns 304" "$BASE_URL/static/test.txt" "304" "If-Modified-Since: $LAST_MODIFIED"

# If-Modified-Since with old date returns 200
assert_status "If-Modified-Since old date returns 200" "$BASE_URL/static/test.txt" "200" "If-Modified-Since: Thu, 01 Jan 1970 00:00:00 GMT"
