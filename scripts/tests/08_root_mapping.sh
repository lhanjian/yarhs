#!/bin/bash
# 根路径映射测试
# 此脚本由 integration_tests.sh 调用，common.sh 已加载

log_section "8. 根路径映射"

# 配置根路径映射
curl -s -X POST "$API_URL/v1/discovery:routes" \
    -H "Content-Type: application/json" \
    -d '{
  "resources": [{
    "favicon_paths": ["/favicon.ico", "/favicon.svg"],
    "index_files": ["index.html", "index.htm"],
    "custom_routes": {
      "/": {"type": "dir", "path": "static"}
    }
  }]
}' > /dev/null

sleep 0.5
log_info "根路径映射已配置 (/ -> static/)"

# 测试根路径映射
RESPONSE=$(curl -s "$BASE_URL/test.txt")
assert_contains "根路径文件 (/test.txt -> static/test.txt)" "$RESPONSE" "Hello"

CONTENT_TYPE=$(curl -sI "$BASE_URL/style.css" | grep -i "content-type" | tr -d '\r')
assert_contains "根路径 MIME 类型 (/style.css)" "$CONTENT_TYPE" "text/css"
