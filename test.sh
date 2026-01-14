#!/bin/bash
# Server functionality test script

BASE_URL="http://127.0.0.1:8080"
API_URL="http://127.0.0.1:8000"

# Cleanup function
cleanup() {
    if [ -n "$SERVER_PID" ] && kill -0 $SERVER_PID 2>/dev/null; then
        kill $SERVER_PID 2>/dev/null
        wait $SERVER_PID 2>/dev/null
    fi
}
trap cleanup EXIT

echo "======================================"
echo "🧪 Rust Web Server Functionality Test"
echo "======================================"
echo ""

# Start server
echo "🚀 Starting server..."
./target/release/rust_webserver > /tmp/server.log 2>&1 &
SERVER_PID=$!
sleep 2

if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo "❌ Server failed to start!"
    cat /tmp/server.log
    exit 1
fi
echo "✓ Server started (PID: $SERVER_PID)"
echo ""

# Test static files
echo "1️⃣  Static File Serving Test"
echo "   TXT: $(curl -s $BASE_URL/static/test.txt | head -c 25)..."
echo "   HTML: $(curl -s $BASE_URL/static/test.html | grep -o '<h1>.*</h1>')"
echo "   JSON: $(curl -s $BASE_URL/static/data.json | head -c 40)..."
echo "   CSS: $(curl -s $BASE_URL/static/style.css | head -1)"
echo ""

# Test request body limit
echo "2️⃣  Request Body Size Limit Test"
HTTP_CODE=$(curl -s -w "%{http_code}" -X POST -H "Content-Length: 20000000" $BASE_URL/ -o /dev/null)
echo "   Large request (20MB): HTTP $HTTP_CODE $([ "$HTTP_CODE" = "413" ] && echo "✓" || echo "✗")"
echo ""

# Test concurrency
echo "3️⃣  Concurrent Performance Test"
START=$(date +%s%N)
# Run 10 concurrent requests with timeout
pids=""
for i in {1..10}; do
  curl -s --max-time 2 $BASE_URL/ > /dev/null 2>&1 &
  pids="$pids $!"
done
# Wait for all with timeout
for pid in $pids; do
  wait $pid 2>/dev/null
done
END=$(date +%s%N)
ELAPSED=$(( (END - START) / 1000000 ))
echo "   10 concurrent requests: ${ELAPSED}ms"
echo ""

# Test routing
echo "4️⃣  Routing Test"
echo "   Homepage (/): $(curl -s $BASE_URL/ | grep -o '<title>.*</title>' | head -1)"
echo "   File route (/about): $(curl -s -w "HTTP %{http_code}" $BASE_URL/about -o /dev/null)"
echo "   Dir route (/static/): $(curl -s -w "HTTP %{http_code}" $BASE_URL/static/ -o /dev/null)"
echo "   Favicon: $(curl -s -I $BASE_URL/favicon.svg | grep -i 'content-type' | cut -d' ' -f2)"
echo "   Config API (port 8000): $(curl -s $API_URL/v1/discovery:logging | grep -o '"level"' | wc -l) fields"
echo ""

# Test ETag
echo "5️⃣  ETag + 304 Test"
ETAG=$(curl -sI $BASE_URL/static/test.txt | grep -i "etag:" | cut -d' ' -f2 | tr -d '\r')
echo "   ETag: $ETAG"
STATUS=$(curl -sI -H "If-None-Match: $ETAG" $BASE_URL/static/test.txt | grep "HTTP" | cut -d' ' -f2)
echo "   Conditional request: HTTP $STATUS $([ "$STATUS" = "304" ] && echo "✓" || echo "✗")"
echo ""

echo "======================================"
echo "✅ Test Complete"
echo "======================================"
