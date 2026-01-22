#!/bin/bash
# 路由功能测试
# 此脚本由 integration_tests.sh 调用，common.sh 已加载

log_section "2. 路由功能"

assert_status "主页路由 (/)" "$BASE_URL/" "200"
assert_status "File 路由 (/about)" "$BASE_URL/about" "200"
assert_status "Dir 路由 (/static/test.txt)" "$BASE_URL/static/test.txt" "200"

# Favicon
CONTENT_TYPE=$(curl -sI "$BASE_URL/favicon.svg" | grep -i "content-type" | tr -d '\r')
assert_contains "Favicon 响应" "$CONTENT_TYPE" "svg"
