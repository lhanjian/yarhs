#!/bin/bash
# 缓存与条件请求测试
# 此脚本由 integration_tests.sh 调用，common.sh 已加载

log_section "4. 缓存与条件请求"

# ETag
ETAG=$(curl -sI "$BASE_URL/static/test.txt" | grep -i "etag:" | cut -d' ' -f2 | tr -d '\r')
if [ -n "$ETAG" ]; then
    log_pass "ETag 响应头: $ETAG"
else
    log_fail "ETag 响应头缺失"
fi

# 304 Not Modified
assert_status "If-None-Match 匹配返回 304" "$BASE_URL/static/test.txt" "304" "If-None-Match: $ETAG"

# ETag 不匹配
assert_status "If-None-Match 不匹配返回 200" "$BASE_URL/static/test.txt" "200" 'If-None-Match: "wrongetag"'

# Cache-Control
CACHE=$(curl -sI "$BASE_URL/static/test.txt" | grep -i "cache-control" | tr -d '\r')
assert_contains "Cache-Control 头" "$CACHE" "max-age"
