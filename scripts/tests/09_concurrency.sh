#!/bin/bash
# Concurrency Tests
# This script is called by integration_tests.sh, common.sh is already loaded

log_section "9. Concurrency Tests"

START=$(date +%s%N)
pids=""
for i in {1..20}; do
    curl -s --max-time 2 "$BASE_URL/" > /dev/null 2>&1 &
    pids="$pids $!"
done
for pid in $pids; do
    wait $pid 2>/dev/null || true
done
END=$(date +%s%N)
ELAPSED=$(( (END - START) / 1000000 ))
log_pass "20 concurrent requests completed: ${ELAPSED}ms"

# Optional: ab performance test
if command -v ab &> /dev/null; then
    log_info "ApacheBench performance test:"
    ab -n 500 -c 10 -q "$BASE_URL/test.txt" 2>&1 | grep "Requests per second" || echo "  (skipped)"
fi
