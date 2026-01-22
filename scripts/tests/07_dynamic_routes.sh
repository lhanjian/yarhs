#!/bin/bash
# 动态路由配置测试
# 此脚本由 integration_tests.sh 调用，common.sh 已加载

log_section "7. 动态路由配置"

# 创建测试文件
mkdir -p templates static
cat > templates/contact.html << 'EOF'
<!DOCTYPE html>
<html><head><title>Contact</title></head>
<body><h1>Contact Us</h1></body></html>
EOF
echo '{"name": "test", "version": "1.0"}' > static/api.json
log_info "测试文件已创建"

# 获取当前配置并添加路由
curl -s "$API_URL/v1/discovery:routes" > /tmp/config.json

jq '{
  resources: [{
    favicon_paths: .resources[0].favicon_paths,
    index_files: .resources[0].index_files,
    custom_routes: ((.resources[0].custom_routes // {}) + {
      "/contact": {type: "file", path: "templates/contact.html"},
      "/api-spec": {type: "file", path: "static/api.json"},
      "/docs": {type: "redirect", target: "/about"}
    })
  }]
}' /tmp/config.json > /tmp/xds_routes.json

RESPONSE=$(curl -s -X POST "$API_URL/v1/discovery:routes" \
    -H "Content-Type: application/json" \
    -d @/tmp/xds_routes.json)

if echo "$RESPONSE" | grep -q '"status".*"ACK"'; then
    log_pass "动态添加路由 (ACK)"
else
    log_fail "动态添加路由失败: $RESPONSE"
fi

sleep 0.3  # 等待配置生效

# 测试新添加的路由
RESPONSE=$(curl -s "$BASE_URL/contact")
assert_contains "动态 File 路由 (/contact)" "$RESPONSE" "Contact Us"

RESPONSE=$(curl -s "$BASE_URL/api-spec")
assert_contains "动态 JSON 路由 (/api-spec)" "$RESPONSE" '"version"'

LOCATION=$(curl -sI "$BASE_URL/docs" | grep -i "location:" | tr -d '\r')
assert_contains "动态 Redirect 路由 (/docs)" "$LOCATION" "/about"
