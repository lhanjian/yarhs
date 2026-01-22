#!/bin/bash
# 静态文件服务测试
# 此脚本由 integration_tests.sh 调用，common.sh 已加载

log_section "1. 静态文件服务"

# 多种文件类型
RESPONSE=$(curl -s "$BASE_URL/static/test.txt")
assert_contains "TXT 文件内容" "$RESPONSE" "Hello"

RESPONSE=$(curl -s "$BASE_URL/static/test.html")
assert_contains "HTML 文件内容" "$RESPONSE" "<h1>"

RESPONSE=$(curl -s "$BASE_URL/static/data.json")
assert_contains "JSON 文件内容" "$RESPONSE" "{"

# MIME 类型检测
CONTENT_TYPE=$(curl -sI "$BASE_URL/static/style.css" | grep -i "content-type" | tr -d '\r')
assert_contains "CSS MIME 类型" "$CONTENT_TYPE" "text/css"

CONTENT_TYPE=$(curl -sI "$BASE_URL/static/data.json" | grep -i "content-type" | tr -d '\r')
assert_contains "JSON MIME 类型" "$CONTENT_TYPE" "application/json"

# 默认文档
RESPONSE=$(curl -s "$BASE_URL/static/")
assert_contains "目录默认文档 (index.html)" "$RESPONSE" "html"

# 404 测试
assert_status "不存在的文件返回 404" "$BASE_URL/static/nonexistent.xyz" "404"
