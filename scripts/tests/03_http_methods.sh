#!/bin/bash
# HTTP 方法处理测试
# 此脚本由 integration_tests.sh 调用，common.sh 已加载

log_section "3. HTTP 方法处理"

# GET (隐式，通过状态码验证)
assert_status "GET 方法" "$BASE_URL/" "200"

# HEAD
HEAD_STATUS=$(curl -sI "$BASE_URL/" | grep "HTTP" | cut -d' ' -f2 | tr -d '\r')
HEAD_LENGTH=$(curl -sI "$BASE_URL/" | grep -i "content-length" | tr -d '\r')
if [ "$HEAD_STATUS" = "200" ] && [ -n "$HEAD_LENGTH" ]; then
    log_pass "HEAD 方法 (HTTP 200 + Content-Length)"
else
    log_fail "HEAD 方法"
fi

# OPTIONS
assert_status "OPTIONS 方法" "$BASE_URL/" "204" "-X OPTIONS"
ALLOW=$(curl -sI -X OPTIONS "$BASE_URL/" | grep -i "allow:" | tr -d '\r')
assert_contains "OPTIONS Allow 头包含 GET" "$ALLOW" "GET"
assert_contains "OPTIONS Allow 头包含 HEAD" "$ALLOW" "HEAD"

# 不允许的方法
assert_status "POST 返回 405" "$BASE_URL/" "405" "-X POST"
assert_status "PUT 返回 405" "$BASE_URL/" "405" "-X PUT"
assert_status "DELETE 返回 405" "$BASE_URL/" "405" "-X DELETE"
