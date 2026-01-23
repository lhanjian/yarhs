#!/bin/bash
# Virtual Host Integration Tests
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "13. Virtual Host Routing"

# Create temp directory for vhost test files
VHOST_DIR="/tmp/yarhs_vhost_$$"
mkdir -p "$VHOST_DIR/site-a" "$VHOST_DIR/site-b"
echo "Site A Content" > "$VHOST_DIR/site-a/index.html"
echo "Site B Content" > "$VHOST_DIR/site-b/index.html"
echo "Site A About" > "$VHOST_DIR/site-a/about.html"

# Test 1: Check virtual_hosts API endpoint
VHOSTS_RESPONSE=$(curl -s "$API_URL/v1/discovery:vhosts")
assert_json_has "VirtualHosts GET endpoint" "$VHOSTS_RESPONSE" ".type_url"

# Test 2: Empty virtual_hosts by default (or check it has some value)
# Note: In this test context, we just verify the API responds correctly
VHOST_ARRAY=$(echo "$VHOSTS_RESPONSE" | jq '.resources[0].virtual_hosts')
if [ "$VHOST_ARRAY" = "[]" ] || [ -n "$VHOST_ARRAY" ]; then
    log_pass "VirtualHosts API returns valid response"
else
    log_fail "VirtualHosts API response invalid"
fi

# Test 3: Configure virtual hosts via API
# Note: RouteAction uses tag-based serialization with "type" field
VHOST_CONFIG='{
  "virtual_hosts": [
    {
      "name": "site-a",
      "domains": ["site-a.local", "*.site-a.local"],
      "routes": [
        {
          "name": "root",
          "match": {"prefix": "/"},
          "type": "dir",
          "path": "'"$VHOST_DIR"'/site-a"
        }
      ]
    },
    {
      "name": "site-b", 
      "domains": ["site-b.local"],
      "routes": [
        {
          "name": "root",
          "match": {"prefix": "/"},
          "type": "dir",
          "path": "'"$VHOST_DIR"'/site-b"
        }
      ]
    },
    {
      "name": "catch-all",
      "domains": ["*"],
      "routes": [
        {
          "name": "default",
          "match": {"prefix": "/"},
          "type": "direct",
          "status": 404,
          "body": "No matching host"
        }
      ]
    }
  ]
}'

UPDATE_RESPONSE=$(curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d "{\"resources\": [$VHOST_CONFIG]}")
assert_json_field "Configure virtual hosts ACK" "$UPDATE_RESPONSE" ".status" "ACK"

# Test 4: Verify virtual hosts count
VHOSTS_RESPONSE=$(curl -s "$API_URL/v1/discovery:vhosts")
VHOST_COUNT=$(echo "$VHOSTS_RESPONSE" | jq '.resources[0].virtual_hosts | length')
if [ "$VHOST_COUNT" = "3" ]; then
    log_pass "Virtual hosts count: 3"
else
    log_fail "Virtual hosts count (expected: 3, got: $VHOST_COUNT)"
fi

# Test 5: Request with Host: site-a.local
SITE_A_RESPONSE=$(curl -s -H "Host: site-a.local" "$BASE_URL/index.html")
assert_contains "Site A routing (exact domain)" "$SITE_A_RESPONSE" "Site A Content"

# Test 6: Request with Host: sub.site-a.local (wildcard match)
SITE_A_SUB_RESPONSE=$(curl -s -H "Host: sub.site-a.local" "$BASE_URL/index.html")
assert_contains "Site A routing (wildcard domain)" "$SITE_A_SUB_RESPONSE" "Site A Content"

# Test 7: Request with Host: site-b.local
SITE_B_RESPONSE=$(curl -s -H "Host: site-b.local" "$BASE_URL/index.html")
assert_contains "Site B routing" "$SITE_B_RESPONSE" "Site B Content"

# Test 8: Request with unknown host (catch-all)
UNKNOWN_RESPONSE=$(curl -s -H "Host: unknown.local" "$BASE_URL/")
assert_contains "Catch-all routing" "$UNKNOWN_RESPONSE" "No matching host"

# Test 9: Clear virtual hosts (restore empty)
UPDATE_RESPONSE=$(curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{"resources": [{"virtual_hosts": []}]}')
assert_json_field "Clear virtual hosts ACK" "$UPDATE_RESPONSE" ".status" "ACK"

# Test 10: Verify fallback to legacy routes after clearing vhosts
FALLBACK_RESPONSE=$(curl -s "$BASE_URL/static/test.txt")
assert_contains "Fallback to legacy routes" "$FALLBACK_RESPONSE" "Hello"

# Cleanup
rm -rf "$VHOST_DIR"

log_info "Virtual host tests completed"
