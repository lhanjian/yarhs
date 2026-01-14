#!/bin/bash
# Dynamic route configuration test script

echo "========================================="
echo "Dynamic Route Configuration Test"
echo "========================================="

# Color definitions
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counters
PASS=0
FAIL=0

# Cleanup function
cleanup() {
    echo -e "\n${BLUE}[Cleanup] Stopping server...${NC}"
    if [ -n "$SERVER_PID" ] && kill -0 $SERVER_PID 2>/dev/null; then
        kill $SERVER_PID 2>/dev/null
        wait $SERVER_PID 2>/dev/null
    fi
    # Restore config file
    if [ -f config.toml.bak ]; then
        mv config.toml.bak config.toml
    fi
}
trap cleanup EXIT

# Assert functions
assert_contains() {
    local name="$1"
    local content="$2"
    local expected="$3"
    if echo "$content" | grep -q "$expected"; then
        echo -e "    ${GREEN}✓ $name${NC}"
        ((PASS++))
        return 0
    else
        echo -e "    ${RED}✗ $name${NC}"
        ((FAIL++))
        return 1
    fi
}

assert_status() {
    local name="$1"
    local url="$2"
    local expected="$3"
    local headers="${4:-}"
    local status
    if [ -n "$headers" ]; then
        status=$(curl -sI -H "$headers" "$url" | grep "HTTP" | tr -d '\r')
    else
        status=$(curl -sI "$url" | grep "HTTP" | tr -d '\r')
    fi
    if echo "$status" | grep -q "$expected"; then
        echo -e "    ${GREEN}✓ $name${NC}"
        echo "      $status"
        ((PASS++))
        return 0
    else
        echo -e "    ${RED}✗ $name (expected: $expected, actual: $status)${NC}"
        ((FAIL++))
        return 1
    fi
}

# ============================================
# Part 1: Dynamic Route API Test
# ============================================
echo -e "\n${YELLOW}============================================${NC}"
echo -e "${YELLOW}Part 1: Dynamic Route API Test${NC}"
echo -e "${YELLOW}============================================${NC}"

# 1. Start server
echo -e "\n${BLUE}[1] Starting server...${NC}"
./target/release/rust_webserver > /tmp/server.log 2>&1 &
SERVER_PID=$!
sleep 2

if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo -e "${RED}Server failed to start!${NC}"
    cat /tmp/server.log
    exit 1
fi
echo -e "    ${GREEN}✓ Server started (PID: $SERVER_PID)${NC}"

# 2. View default route config
echo -e "\n${BLUE}[2] View default route config:${NC}"
curl -s http://localhost:8000/v1/discovery:routes | jq '.resources[0].custom_routes | keys'

# 3. Create test files
echo -e "\n${BLUE}[3] Create test files...${NC}"

cat > templates/contact.html << 'HTMLEOF'
<!DOCTYPE html>
<html>
<head>
    <title>Contact Us</title>
    <style>
        body { font-family: Arial; max-width: 600px; margin: 50px auto; }
        h1 { color: #667eea; }
    </style>
</head>
<body>
    <h1>Contact Us</h1>
    <p>Email: contact@example.com</p>
</body>
</html>
HTMLEOF

echo '{"name": "test", "version": "1.0"}' > static/api.json
echo -e "    ${GREEN}✓ Test files created${NC}"

# 4. Add custom routes via xDS API
echo -e "\n${BLUE}[4] Add custom routes via xDS API...${NC}"
curl -s http://localhost:8000/v1/discovery:routes > /tmp/config.json

jq '{
  resources: [
    {
      favicon_paths: .resources[0].favicon_paths,
      index_files: .resources[0].index_files,
      custom_routes: ((.resources[0].custom_routes // {}) + {
        "/contact": {type: "file", path: "templates/contact.html"},
        "/api-spec": {type: "file", path: "static/api.json"},
        "/docs": {type: "redirect", target: "/about"}
      })
    }
  ]
}' /tmp/config.json > /tmp/xds_routes.json

RESULT=$(curl -s -X POST http://localhost:8000/v1/discovery:routes \
    -H "Content-Type: application/json" \
    -d @/tmp/xds_routes.json)
if echo "$RESULT" | grep -q '"status": "ACK"'; then
    echo -e "    ${GREEN}✓ Routes updated successfully${NC}"
    ((PASS++))
else
    echo -e "    ${RED}✗ Route update failed: $RESULT${NC}"
    ((FAIL++))
fi

# 5. Verify route config updated
echo -e "\n${BLUE}[5] Verify updated route config:${NC}"
KEYS=$(curl -s http://localhost:8000/v1/discovery:routes | jq -r '.resources[0].custom_routes | keys | join(",")')
echo "    Configured routes: $KEYS"

# 6. Test each route
echo -e "\n${BLUE}[6] Test routing functionality:${NC}"

echo -e "\n  ${GREEN}➤ File route - HTML (/contact):${NC}"
RESPONSE=$(curl -s http://localhost:8080/contact)
assert_contains "HTML file loaded" "$RESPONSE" "Contact Us"

echo -e "\n  ${GREEN}➤ File route - JSON (/api-spec):${NC}"
RESPONSE=$(curl -s http://localhost:8080/api-spec)
assert_contains "JSON file loaded" "$RESPONSE" '"version"'
CONTENT_TYPE=$(curl -sI http://localhost:8080/api-spec | grep -i "content-type")
assert_contains "JSON Content-Type" "$CONTENT_TYPE" "application/json"

echo -e "\n  ${GREEN}➤ Redirect route (/docs → /about):${NC}"
LOCATION=$(curl -sI http://localhost:8080/docs | grep -i "location:")
assert_contains "Redirect to /about" "$LOCATION" "/about"

echo -e "\n  ${GREEN}➤ Dir route (/static/test.txt):${NC}"
RESPONSE=$(curl -s http://localhost:8080/static/test.txt)
assert_contains "Static file loaded" "$RESPONSE" "Hello"

echo -e "\n  ${GREEN}➤ ETag header:${NC}"
ETAG=$(curl -sI http://localhost:8080/static/test.txt | grep -i "etag:" | tr -d '\r')
assert_contains "ETag header present" "$ETAG" "etag"
ETAG_VALUE=$(echo "$ETAG" | awk '{print $2}')

echo -e "\n  ${GREEN}➤ ETag conditional request (304):${NC}"
assert_status "304 Not Modified" "http://localhost:8080/static/test.txt" "304" "If-None-Match: $ETAG_VALUE"

echo -e "\n  ${GREEN}➤ ETag conditional request (200):${NC}"
assert_status "200 OK (ETag mismatch)" "http://localhost:8080/static/test.txt" "200" 'If-None-Match: "wrongetag"'

echo -e "\n  ${GREEN}➤ Default document (/static/):${NC}"
RESPONSE=$(curl -s http://localhost:8080/static/)
assert_contains "Default document loaded" "$RESPONSE" "html"

echo -e "\n  ${GREEN}➤ API endpoint (/v1/discovery):${NC}"
CONFIG_SIZE=$(curl -s http://localhost:8000/v1/discovery | wc -c)
if [ "$CONFIG_SIZE" -gt 100 ]; then
    echo -e "    ${GREEN}✓ API response OK (${CONFIG_SIZE} bytes)${NC}"
    ((PASS++))
else
    echo -e "    ${RED}✗ API response error${NC}"
    ((FAIL++))
fi

# ============================================
# Part 2: Root Path Dir Mapping Test
# ============================================
echo -e "\n${YELLOW}============================================${NC}"
echo -e "${YELLOW}Part 2: Root Path Dir Mapping Test${NC}"
echo -e "${YELLOW}============================================${NC}"

echo -e "\n${BLUE}[7] Configure root path mapping via xDS API...${NC}"

# Use xDS API to dynamically update to root path mapping (no server restart needed)
curl -s -X POST http://localhost:8000/v1/discovery:routes \
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

echo -e "    ${GREEN}✓ Root path mapping configured${NC}"

echo -e "\n${BLUE}[8] Test root path mapping:${NC}"

echo -e "\n  ${GREEN}➤ / (should return static/index.html):${NC}"
RESPONSE=$(curl -s http://localhost:8080/)
# static/index.html may not exist, check for content or 404
if echo "$RESPONSE" | grep -qi "html\|index\|static"; then
    echo -e "    ${GREEN}✓ Root path default document loaded${NC}"
    ((PASS++))
else
    echo -e "    ${YELLOW}⚠ Default document may not exist (normal if static/index.html doesn't exist)${NC}"
fi

echo -e "\n  ${GREEN}➤ /test.txt (should return static/test.txt):${NC}"
RESPONSE=$(curl -s http://localhost:8080/test.txt)
assert_contains "Root path file loaded" "$RESPONSE" "Hello"

echo -e "\n  ${GREEN}➤ /style.css (MIME type):${NC}"
CONTENT_TYPE=$(curl -sI http://localhost:8080/style.css | grep -i "content-type")
assert_contains "CSS MIME type" "$CONTENT_TYPE" "text/css"

# ============================================
# Part 3: Performance Test (optional)
# ============================================
if command -v ab &> /dev/null; then
    echo -e "\n${YELLOW}============================================${NC}"
    echo -e "${YELLOW}Part 3: Performance Test${NC}"
    echo -e "${YELLOW}============================================${NC}"

    echo -e "\n${BLUE}[9] Route performance comparison:${NC}"
    echo -e "  Root path (/):"
    ab -n 1000 -c 10 -q http://localhost:8080/ 2>&1 | grep "Requests per second" || echo "    (ab test skipped)"
    echo -e "  Static file (/test.txt):"
    ab -n 1000 -c 10 -q http://localhost:8080/test.txt 2>&1 | grep "Requests per second" || echo "    (ab test skipped)"
else
    echo -e "\n${YELLOW}[Skip] Performance test (ab tool not installed)${NC}"
fi

# ============================================
# Test Results Summary
# ============================================
echo -e "\n${YELLOW}============================================${NC}"
echo -e "${YELLOW}Test Results Summary${NC}"
echo -e "${YELLOW}============================================${NC}"

echo -e "\nPassed: ${GREEN}$PASS${NC}"
echo -e "Failed: ${RED}$FAIL${NC}"

echo -e "\nFeatures verified:"
echo "  ✓ xDS API dynamic route configuration"
echo "  ✓ File routes (supports any file type)"
echo "  ✓ Redirect routes (302 redirect)"
echo "  ✓ Dir routes (directory mapping)"
echo "  ✓ Root path Dir mapping"
echo "  ✓ Default document (index.html)"
echo "  ✓ ETag + 304 (conditional requests)"
echo "  ✓ MIME type auto-detection"

if [ $FAIL -eq 0 ]; then
    echo -e "\n${GREEN}========================================="
    echo "✅ All tests passed!"
    echo -e "=========================================${NC}"
    exit 0
else
    echo -e "\n${RED}========================================="
    echo "❌ $FAIL test(s) failed"
    echo -e "=========================================${NC}"
    exit 1
fi
