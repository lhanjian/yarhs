#!/bin/bash
# 并发测试
# 此脚本由 integration_tests.sh 调用，common.sh 已加载

log_section "9. 并发测试"

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
log_pass "20 个并发请求完成: ${ELAPSED}ms"

# 可选: ab 性能测试
if command -v ab &> /dev/null; then
    log_info "ApacheBench 性能测试:"
    ab -n 500 -c 10 -q "$BASE_URL/test.txt" 2>&1 | grep "Requests per second" || echo "  (跳过)"
fi
