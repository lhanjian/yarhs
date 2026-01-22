#!/bin/bash
# YARHS Integration Tests Main Entry
# Starts server and executes all test modules
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

# Load shared module
source "$SCRIPT_DIR/tests/common.sh"

# Cleanup function
cleanup() {
    log_info "Cleaning up test environment..."
    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -f /tmp/config.json /tmp/xds_routes.json /tmp/server.log
    rm -f templates/contact.html static/api.json
}
trap cleanup EXIT

echo ""
echo "╔════════════════════════════════════════╗"
echo "║       YARHS Integration Test Suite       ║"
echo "╚════════════════════════════════════════╝"
echo ""

# ============================================
# Start Server
# ============================================
log_info "Starting server..."
./target/release/rust_webserver > /tmp/server.log 2>&1 &
SERVER_PID=$!
sleep 2

if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    log_fail "Server failed to start"
    cat /tmp/server.log
    exit 1
fi
log_pass "Server started successfully (PID: $SERVER_PID)"

# ============================================
# Execute Test Modules
# ============================================
for test_file in "$SCRIPT_DIR/tests"/[0-9]*.sh; do
    if [ -f "$test_file" ]; then
        source "$test_file"
    fi
done

# ============================================
# Results Summary
# ============================================
log_info "Stopping server..."
kill "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
SERVER_PID=""

echo ""
echo "╔════════════════════════════════════════╗"
echo "║           Test Results Summary           ║"
echo "╠════════════════════════════════════════╣"
printf "║  Passed: ${GREEN}%-3d${NC}                            ║\n" $PASS
printf "║  Failed: ${RED}%-3d${NC}                            ║\n" $FAIL
echo "╠════════════════════════════════════════╣"
echo "║  Test Coverage:                           ║"
echo "║    ✓ Static File Serving + MIME Detection ║"
echo "║    ✓ Routing (File/Dir/Redirect)          ║"
echo "║    ✓ HTTP Methods (GET/HEAD/OPTIONS/405)  ║"
echo "║    ✓ Caching (ETag + 304)                 ║"
echo "║    ✓ Range Requests (Resume Download)     ║"
echo "║    ✓ xDS API Endpoints                    ║"
echo "║    ✓ Dynamic Route Configuration          ║"
echo "║    ✓ Root Path Mapping                    ║"
echo "║    ✓ Concurrent Requests                  ║"
echo "║    ✓ Health Check Endpoints               ║"
echo "╚════════════════════════════════════════╝"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo -e "${GREEN}✅ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ $FAIL test(s) failed${NC}"
    exit 1
fi
