#!/bin/bash
# Dynamic Route Configuration Tests
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "7. Dynamic Route Configuration"

# Create test files
mkdir -p templates static
cat > templates/contact.html << 'EOF'
<!DOCTYPE html>
<html><head><title>Contact</title></head>
<body><h1>Contact Us</h1></body></html>
EOF
echo '{"name": "test", "version": "1.0"}' > static/api.json
log_info "Test files created"

# Get current config and add routes
curl -s "$API_URL/v1/discovery:routes" > /tmp/config.json

jq '{
  resources: [{
    index_files: .resources[0].index_files,
    custom_routes: ((.resources[0].custom_routes // {}) + {
      "/contact": {type: "file", path: "templates/contact.html"},
      "/api-spec": {type: "file", path: "static/api.json"},
      "/docs": {type: "redirect", target: "/about"}
    })
  }]
}' /tmp/config.json > /tmp/xds_routes.json

RESPONSE=$(curl -s -X POST "$API_URL/v1/discovery:routes" \
    -H "Content-Type: application/json" \
    -d @/tmp/xds_routes.json)

assert_json_field "Dynamic route addition (ACK)" "$RESPONSE" ".status" "ACK"

sleep 0.3  # Wait for config to take effect

# Test newly added routes
RESPONSE=$(curl -s "$BASE_URL/contact")
assert_contains "Dynamic File route (/contact)" "$RESPONSE" "Contact Us"

RESPONSE=$(curl -s "$BASE_URL/api-spec")
assert_contains "Dynamic JSON route (/api-spec)" "$RESPONSE" '"version"'

LOCATION=$(curl -sI "$BASE_URL/docs" | grep -i "location:" | tr -d '\r')
assert_contains "Dynamic Redirect route (/docs)" "$LOCATION" "/about"
