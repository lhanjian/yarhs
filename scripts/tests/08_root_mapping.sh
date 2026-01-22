#!/bin/bash
# Root Path Mapping Tests
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "8. Root Path Mapping"

# Configure root path mapping
curl -s -X POST "$API_URL/v1/discovery:routes" \
    -H "Content-Type: application/json" \
    -d '{
  "resources": [{
    "favicon_paths": ["/favicon.ico", "/favicon.svg"],
    "index_files": ["index.html", "index.htm"],
    "custom_routes": {
      "/": {"type": "dir", "path": "static"}
    }
  }]
}' > /dev/null

sleep 0.5
log_info "Root path mapping configured (/ -> static/)"

# Test root path mapping
RESPONSE=$(curl -s "$BASE_URL/test.txt")
assert_contains "Root path file (/test.txt -> static/test.txt)" "$RESPONSE" "Hello"

CONTENT_TYPE=$(curl -sI "$BASE_URL/style.css" | grep -i "content-type" | tr -d '\r')
assert_contains "Root path MIME type (/style.css)" "$CONTENT_TYPE" "text/css"
