#!/bin/bash
# Range Request Tests (Resume Download)
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "5. Range Requests (Resume Download)"

# Accept-Ranges header
ACCEPT_RANGES=$(curl -sI "$BASE_URL/static/test.txt" | grep -i "accept-ranges:" | tr -d '\r')
assert_contains "Accept-Ranges header" "$ACCEPT_RANGES" "bytes"

# Fixed range request (first 10 bytes)
RANGE_STATUS=$(curl -sI -H "Range: bytes=0-9" "$BASE_URL/static/test.txt" | grep "HTTP" | cut -d' ' -f2 | tr -d '\r')
CONTENT_RANGE=$(curl -sI -H "Range: bytes=0-9" "$BASE_URL/static/test.txt" | grep -i "content-range:" | tr -d '\r')
if [ "$RANGE_STATUS" = "206" ]; then
    log_pass "Range request returns 206 Partial Content"
else
    log_fail "Range request returns 206 (got: $RANGE_STATUS)"
fi
assert_contains "Content-Range header" "$CONTENT_RANGE" "bytes 0-9/"

# Open range request (from byte 5 to end)
CONTENT_RANGE=$(curl -sI -H "Range: bytes=5-" "$BASE_URL/static/test.txt" | grep -i "content-range:" | tr -d '\r')
assert_contains "Open range request (bytes=5-)" "$CONTENT_RANGE" "bytes 5-"

# Suffix range request (last 5 bytes)
CONTENT_RANGE=$(curl -sI -H "Range: bytes=-5" "$BASE_URL/static/test.txt" | grep -i "content-range:" | tr -d '\r')
assert_contains "Suffix range request (bytes=-5)" "$CONTENT_RANGE" "bytes"

# Verify returned content length
RANGE_LENGTH=$(curl -sI -H "Range: bytes=0-9" "$BASE_URL/static/test.txt" | grep -i "content-length:" | cut -d' ' -f2 | tr -d '\r')
if [ "$RANGE_LENGTH" = "10" ]; then
    log_pass "Range Content-Length correct (10)"
else
    log_fail "Range Content-Length incorrect (expected: 10, got: $RANGE_LENGTH)"
fi

# 416 Range Not Satisfiable (exceeds file size)
assert_status "Invalid range returns 416 (bytes=99999-)" "$BASE_URL/static/test.txt" "416" "Range: bytes=99999-"

# 416 Content-Range header format
CONTENT_RANGE_416=$(curl -sI -H "Range: bytes=99999-" "$BASE_URL/static/test.txt" | grep -i "content-range:" | tr -d '\r')
assert_contains "416 Content-Range format (bytes */size)" "$CONTENT_RANGE_416" "bytes \*/"
