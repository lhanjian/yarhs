#!/bin/bash
# Static File Serving Tests
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "1. Static File Serving"

# Multiple file types
RESPONSE=$(curl -s "$BASE_URL/static/test.txt")
assert_contains "TXT file content" "$RESPONSE" "Hello"

RESPONSE=$(curl -s "$BASE_URL/static/test.html")
assert_contains "HTML file content" "$RESPONSE" "<h1>"

RESPONSE=$(curl -s "$BASE_URL/static/data.json")
assert_contains "JSON file content" "$RESPONSE" "{"

# MIME type detection
CONTENT_TYPE=$(curl -sI "$BASE_URL/static/style.css" | grep -i "content-type" | tr -d '\r')
assert_contains "CSS MIME type" "$CONTENT_TYPE" "text/css"

CONTENT_TYPE=$(curl -sI "$BASE_URL/static/data.json" | grep -i "content-type" | tr -d '\r')
assert_contains "JSON MIME type" "$CONTENT_TYPE" "application/json"

# Default document
RESPONSE=$(curl -s "$BASE_URL/static/")
assert_contains "Directory default document (index.html)" "$RESPONSE" "html"

# 404 test
assert_status "Non-existent file returns 404" "$BASE_URL/static/nonexistent.xyz" "404"
