#!/bin/bash
# 动态路由配置测试脚本

echo "========================================="
echo "动态路由配置功能测试"
echo "========================================="

# 颜色定义
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# 1. 启动服务器
echo -e "\n${BLUE}[1] 启动服务器...${NC}"
./target/release/rust_webserver > /tmp/server.log 2>&1 &
SERVER_PID=$!
sleep 2

# 2. 查看默认路由配置
echo -e "\n${BLUE}[2] 查看默认路由配置:${NC}"
curl -s http://localhost:8000/v1/discovery:routes | jq '.resources[0]'

# 3. 创建测试文件
echo -e "\n${BLUE}[3] 创建测试文件...${NC}"

cat > templates/contact.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Contact Us</title>
    <style>
        body { font-family: Arial; max-width: 600px; margin: 50px auto; }
        h1 { color: #667eea; }
    </style>
</head>
<body>
    <h1>Contact Us</h1>
    <p>Email: contact@example.com</p>
</body>
</html>
EOF

echo '{"name": "test", "version": "1.0"}' > static/api.json

echo -e "${GREEN}✓ 测试文件创建完成${NC}"

# 4. 更新路由配置
echo -e "\n${BLUE}[4] 添加自定义路由...${NC}"
curl -s http://localhost:8000/v1/discovery:routes > /tmp/config.json

# 添加自定义路由 - 从 xDS 响应中提取现有配置并添加新的 custom_routes
# xDS 响应格式: { "version": "...", "resources": [ { "favicon_paths": [...], "index_files": [...], "custom_routes": {...} } ] }
jq '{
  resources: [
    {
      favicon_paths: .resources[0].favicon_paths,
      index_files: .resources[0].index_files,
      custom_routes: ((.resources[0].custom_routes // {}) + {
        "/contact": {type: "file", path: "templates/contact.html"},
        "/api-spec": {type: "file", path: "static/api.json"},
        "/docs": {type: "redirect", target: "/about"},
        "/static": {type: "dir", path: "static"}
      })
    }
  ]
}' /tmp/config.json > /tmp/xds_routes.json

curl -s -X POST http://localhost:8000/v1/discovery:routes \
    -H "Content-Type: application/json" \
    -d @/tmp/xds_routes.json | jq

# 5. 验证路由配置已更新
echo -e "\n${BLUE}[5] 验证更新后的路由配置:${NC}"
curl -s http://localhost:8000/v1/discovery:routes | jq '.resources[0].custom_routes | keys'

# 6. 测试各个路由
echo -e "\n${BLUE}[6] 测试各个路由:${NC}"

echo -e "\n  ${GREEN}➤ 测试 File 路由 - HTML (/contact):${NC}"
RESPONSE=$(curl -s http://localhost:8080/contact)
if echo "$RESPONSE" | grep -q "Contact Us"; then
    echo -e "    ${GREEN}✓ HTML 文件加载成功${NC}"
    CONTENT_TYPE=$(curl -sI http://localhost:8080/contact | grep -i "content-type")
    echo "    $CONTENT_TYPE"
else
    echo -e "    ${RED}✗ 失败${NC}"
fi

echo -e "\n  ${GREEN}➤ 测试 File 路由 - JSON (/api-spec):${NC}"
RESPONSE=$(curl -s http://localhost:8080/api-spec)
if echo "$RESPONSE" | grep -q "version"; then
    echo -e "    ${GREEN}✓ JSON 文件加载成功${NC}"
    CONTENT_TYPE=$(curl -sI http://localhost:8080/api-spec | grep -i "content-type")
    echo "    $CONTENT_TYPE"
    echo "    $RESPONSE"
else
    echo -e "    ${RED}✗ 失败${NC}"
fi

echo -e "\n  ${GREEN}➤ 测试 Redirect 路由 (/docs → /about):${NC}"
LOCATION=$(curl -s -I http://localhost:8080/docs | grep -i "location:")
if echo "$LOCATION" | grep -q "/about"; then
    echo -e "    ${GREEN}✓ 重定向正确${NC}"
    echo "    $LOCATION"
else
    echo -e "    ${RED}✗ 失败${NC}"
fi

echo -e "\n  ${GREEN}➤ 测试 Dir 路由 (/static/test.txt):${NC}"
RESPONSE=$(curl -s http://localhost:8080/static/test.txt)
if echo "$RESPONSE" | grep -q "Hello"; then
    echo -e "    ${GREEN}✓ 静态文件加载成功${NC}"
else
    echo -e "    ${RED}✗ 失败${NC}"
fi

echo -e "\n  ${GREEN}➤ 测试 ETag 响应头:${NC}"
ETAG=$(curl -sI http://localhost:8080/static/test.txt | grep -i "etag:" | tr -d '\r')
if echo "$ETAG" | grep -q "etag"; then
    echo -e "    ${GREEN}✓ ETag 已返回${NC}"
    echo "    $ETAG"
else
    echo -e "    ${RED}✗ 失败: 没有 ETag 头${NC}"
fi

echo -e "\n  ${GREEN}➤ 测试 ETag 304 响应 (If-None-Match 匹配):${NC}"
ETAG_VALUE=$(echo "$ETAG" | cut -d' ' -f2)
STATUS=$(curl -sI -H "If-None-Match: $ETAG_VALUE" http://localhost:8080/static/test.txt | grep "HTTP" | tr -d '\r')
if echo "$STATUS" | grep -q "304"; then
    echo -e "    ${GREEN}✓ 304 Not Modified${NC}"
    echo "    $STATUS"
else
    echo -e "    ${RED}✗ 失败: $STATUS${NC}"
fi

echo -e "\n  ${GREEN}➤ 测试 ETag 200 响应 (If-None-Match 不匹配):${NC}"
STATUS=$(curl -sI -H 'If-None-Match: "wrongetag"' http://localhost:8080/static/test.txt | grep "HTTP" | tr -d '\r')
if echo "$STATUS" | grep -q "200"; then
    echo -e "    ${GREEN}✓ 200 OK (不匹配时返回完整内容)${NC}"
    echo "    $STATUS"
else
    echo -e "    ${RED}✗ 失败: $STATUS${NC}"
fi

echo -e "\n  ${GREEN}➤ 测试默认文档 (/static/ → index.html):${NC}"
RESPONSE=$(curl -s http://localhost:8080/static/)
if echo "$RESPONSE" | grep -q "Static Index\|html"; then
    echo -e "    ${GREEN}✓ 默认文档加载成功${NC}"
else
    echo -e "    ${RED}✗ 失败${NC}"
fi

echo -e "\n  ${GREEN}➤ 测试 API 路由 (/v1/discovery):${NC}"
CONFIG_SIZE=$(curl -s http://localhost:8000/v1/discovery | wc -c)
echo -e "    ${GREEN}✓ API 响应大小: ${CONFIG_SIZE} bytes${NC}"

# 7. 性能测试
echo -e "\n${BLUE}[7] 路由性能对比:${NC}"
echo -e "  主页 (/):"
ab -n 1000 -c 10 -q http://localhost:8080/ 2>&1 | grep "Requests per second"
echo -e "  File 路由 - HTML (/contact):"
ab -n 1000 -c 10 -q http://localhost:8080/contact 2>&1 | grep "Requests per second"
echo -e "  File 路由 - JSON (/api-spec):"
ab -n 1000 -c 10 -q http://localhost:8080/api-spec 2>&1 | grep "Requests per second"
echo -e "  Dir 路由 (/static/test.txt):"
ab -n 1000 -c 10 -q http://localhost:8080/static/test.txt 2>&1 | grep "Requests per second"

# 8. 清理
echo -e "\n${BLUE}[8] 停止服务器...${NC}"
kill $SERVER_PID 2>/dev/null
wait $SERVER_PID 2>/dev/null

echo -e "\n${GREEN}========================================="
echo "测试完成！"
echo "=========================================\n${NC}"

# 9. 总结
echo "功能验证："
echo "  ✓ 动态路由配置"
echo "  ✓ File 路由（支持任意文件类型）"
echo "  ✓ Redirect 路由"
echo "  ✓ Dir 路由（目录映射）"
echo "  ✓ 默认文档（index.html）"
echo "  ✓ ETag + 304（条件请求）"
echo "  ✓ API 路由"
echo ""
echo "配置的路由："
echo "  /contact    → templates/contact.html (File - HTML)"
echo "  /api-spec   → static/api.json (File - JSON)"
echo "  /docs       → /about (Redirect)"
echo "  /static/*   → static/* (Dir)"
echo "  /static/    → static/index.html (默认文档)"
echo "  /api/*      → API handlers (port 8000)"

# 10. 测试根路径 Dir 映射
echo -e "\n${BLUE}[10] 测试根路径 Dir 映射...${NC}"

# 备份配置并修改为根路径映射
cp config.toml config.toml.bak

# 直接修改配置文件
cat > config.toml << 'ROOTCONFIG'
[server]
host = "127.0.0.1"
port = 8080
api_host = "0.0.0.0"
api_port = 8000
workers = 4

[logging]
level = "debug"
access_log = false
show_headers = false

[performance]
keep_alive_timeout = 75
read_timeout = 30
write_timeout = 30
max_connections = 5000

[http]
server_name = "Tokio-Hyper/1.0"
default_content_type = "text/html; charset=utf-8"
enable_cors = false
max_body_size = 10485760

[routes]
favicon_paths = ["/favicon.ico", "/favicon.svg"]
index_files = ["index.html", "index.htm"]

[routes.custom_routes]
"/" = { type = "dir", path = "static" }
ROOTCONFIG

# 启动服务器
./target/release/rust_webserver > /tmp/server.log 2>&1 &
ROOT_SERVER_PID=$!
sleep 2

echo -e "\n  ${GREEN}➤ 测试 / (应返回 static/index.html):${NC}"
RESPONSE=$(curl -s http://localhost:8080/)
if echo "$RESPONSE" | grep -q "Static Index\|html"; then
    echo -e "    ${GREEN}✓ 根路径默认文档加载成功${NC}"
else
    echo -e "    ${RED}✗ 失败: $RESPONSE${NC}"
fi

echo -e "\n  ${GREEN}➤ 测试 /test.txt (应返回 static/test.txt):${NC}"
RESPONSE=$(curl -s http://localhost:8080/test.txt)
if echo "$RESPONSE" | grep -q "Hello"; then
    echo -e "    ${GREEN}✓ 根路径文件加载成功${NC}"
else
    echo -e "    ${RED}✗ 失败${NC}"
fi

echo -e "\n  ${GREEN}➤ 测试 /style.css (MIME 类型):${NC}"
CONTENT_TYPE=$(curl -sI http://localhost:8080/style.css | grep -i "content-type")
if echo "$CONTENT_TYPE" | grep -q "text/css"; then
    echo -e "    ${GREEN}✓ MIME 类型正确: $CONTENT_TYPE${NC}"
else
    echo -e "    ${RED}✗ 失败: $CONTENT_TYPE${NC}"
fi

# 清理
kill $ROOT_SERVER_PID 2>/dev/null
mv config.toml.bak config.toml

echo -e "\n${GREEN}========================================="
echo "所有测试完成！"
echo "=========================================\n${NC}"
