#!/bin/bash
# Routing Tests
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "2. Routing"

# Homepage route - verify actual content
HOME_RESPONSE=$(curl -s "$BASE_URL/")
assert_status "Homepage route (/)" "$BASE_URL/" "200"
assert_contains "Homepage has HTML content" "$HOME_RESPONSE" "html"

# File route - verify actual content
ABOUT_RESPONSE=$(curl -s "$BASE_URL/about")
assert_status "File route (/about)" "$BASE_URL/about" "200"
assert_contains "About page has content" "$ABOUT_RESPONSE" "About"

# Dir route - verify actual content
TXT_RESPONSE=$(curl -s "$BASE_URL/static/test.txt")
assert_status "Dir route (/static/test.txt)" "$BASE_URL/static/test.txt" "200"
assert_contains "Static file content" "$TXT_RESPONSE" "Hello"

# Favicon (now served via /static route)
CONTENT_TYPE=$(curl -sI "$BASE_URL/static/favicon.svg" | grep -i "content-type" | tr -d '\r')
assert_contains "Favicon response" "$CONTENT_TYPE" "svg"
