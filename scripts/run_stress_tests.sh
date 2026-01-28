#!/bin/bash
# YARHS Stress Tests Runner
# Runs only 98 and 99 stress/edge case tests separately

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

# Load shared module
source "$SCRIPT_DIR/tests/common.sh"

# Cleanup function
cleanup() {
    log_info "Cleaning up test environment..."
    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill -9 "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    pkill -9 -f "target/release/rust_webserver" 2>/dev/null || true
    rm -f /tmp/config.json /tmp/xds_routes.json /tmp/server.log
    rm -rf /tmp/yarhs_robust_* /tmp/yarhs_stress_*
}
trap cleanup EXIT

echo ""
echo "╔════════════════════════════════════════╗"
echo "║   YARHS Stress/Edge Case Test Suite      ║"
echo "╚════════════════════════════════════════╝"
echo ""

# Kill any existing server
pkill -9 -f "target/release/rust_webserver" 2>/dev/null || true
sleep 1

# Start Server
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

# Run stress tests
for test_file in "$SCRIPT_DIR/tests"/9[89]_*.sh; do
    if [ -f "$test_file" ]; then
        log_info "Running $(basename "$test_file")..."
        source "$test_file"
    fi
done

# Stop Server
log_info "Stopping server..."
kill "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
SERVER_PID=""

# Results Summary
echo ""
echo "╔════════════════════════════════════════╗"
echo "║       Stress Test Results Summary        ║"
echo "╠════════════════════════════════════════╣"
printf "║  Passed: ${GREEN}%-3d${NC}                            ║\n" $PASS
printf "║  Failed: ${RED}%-3d${NC}                            ║\n" $FAIL
echo "╚════════════════════════════════════════╝"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo -e "${GREEN}✅ All stress tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ $FAIL stress test(s) failed${NC}"
    exit 1
fi
