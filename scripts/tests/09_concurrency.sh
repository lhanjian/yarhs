#!/bin/bash
# Concurrency Tests
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "9. Concurrency Tests"

# Prepare results directory
CONCURRENT_DIR="/tmp/yarhs_concurrent_$$"
mkdir -p "$CONCURRENT_DIR"

# Use the health endpoint which is always available and returns known content
START=$(date +%s%N)
pids=""
for i in {1..20}; do
    curl -s --max-time 2 "$BASE_URL/healthz" > "$CONCURRENT_DIR/result_$i.txt" 2>&1 &
    pids="$pids $!"
done
for pid in $pids; do
    wait $pid 2>/dev/null || true
done
END=$(date +%s%N)
ELAPSED=$(( (END - START) / 1000000 ))
log_pass "20 concurrent requests completed: ${ELAPSED}ms"

# Verify all concurrent requests returned correct content ("ok")
CORRECT_COUNT=0
for i in {1..20}; do
    if grep -q "ok" "$CONCURRENT_DIR/result_$i.txt" 2>/dev/null; then
        CORRECT_COUNT=$((CORRECT_COUNT + 1))
    fi
done
if [ "$CORRECT_COUNT" -eq 20 ]; then
    log_pass "All 20 concurrent requests returned correct content"
else
    log_fail "Concurrent requests returned incorrect content ($CORRECT_COUNT/20 correct)"
fi

# Cleanup
rm -rf "$CONCURRENT_DIR"

# Optional: ab performance test
if command -v ab &> /dev/null; then
    log_info "ApacheBench performance test:"
    ab -n 500 -c 10 -q "$BASE_URL/test.txt" 2>&1 | grep "Requests per second" || echo "  (skipped)"
fi
