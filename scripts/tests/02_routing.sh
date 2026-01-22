#!/bin/bash
# Routing Tests
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "2. Routing"

assert_status "Homepage route (/)" "$BASE_URL/" "200"
assert_status "File route (/about)" "$BASE_URL/about" "200"
assert_status "Dir route (/static/test.txt)" "$BASE_URL/static/test.txt" "200"

# Favicon
CONTENT_TYPE=$(curl -sI "$BASE_URL/favicon.svg" | grep -i "content-type" | tr -d '\r')
assert_contains "Favicon response" "$CONTENT_TYPE" "svg"
