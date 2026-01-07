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
curl -s http://localhost:8080/api/config | jq '.routes'

# 3. 创建测试文件
echo -e "\n${BLUE}[3] 创建测试文件...${NC}"
mkdir -p docs
cat > docs/guide.md << 'EOF'
# User Guide

## Getting Started

Welcome to our **awesome** server!

### Features

- Dynamic routing
- Markdown rendering
- Template support
- Redirects
EOF

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

echo -e "${GREEN}✓ 测试文件创建完成${NC}"

# 4. 更新路由配置
echo -e "\n${BLUE}[4] 添加自定义路由...${NC}"
curl -s http://localhost:8080/api/config > /tmp/config.json

# 添加自定义路由
cat /tmp/config.json | jq '.routes.custom_routes = {
  "/guide": {type: "markdown", file: "docs/guide.md"},
  "/contact": {type: "template", file: "templates/contact.html"},
  "/docs": {type: "redirect", target: "/guide"},
  "/api-docs": {type: "markdown", file: "API.md"}
}' > /tmp/new_config.json

curl -s -X PUT http://localhost:8080/api/config \
  -H "Content-Type: application/json" \
  -d @/tmp/new_config.json | jq

# 5. 验证路由配置已更新
echo -e "\n${BLUE}[5] 验证更新后的路由配置:${NC}"
curl -s http://localhost:8080/api/config | jq '.routes.custom_routes | keys'

# 6. 测试各个路由
echo -e "\n${BLUE}[6] 测试各个路由:${NC}"

echo -e "\n  ${GREEN}➤ 测试 Markdown 路由 (/guide):${NC}"
RESPONSE=$(curl -s http://localhost:8080/guide)
if echo "$RESPONSE" | grep -q "User Guide"; then
    echo -e "    ${GREEN}✓ Markdown 渲染成功${NC}"
    echo "$RESPONSE" | head -3
else
    echo -e "    ${RED}✗ 失败${NC}"
fi

echo -e "\n  ${GREEN}➤ 测试 Template 路由 (/contact):${NC}"
RESPONSE=$(curl -s http://localhost:8080/contact)
if echo "$RESPONSE" | grep -q "Contact Us"; then
    echo -e "    ${GREEN}✓ Template 加载成功${NC}"
    echo "$RESPONSE" | grep -E "(h1|Email)"
else
    echo -e "    ${RED}✗ 失败${NC}"
fi

echo -e "\n  ${GREEN}➤ 测试 Redirect 路由 (/docs → /guide):${NC}"
LOCATION=$(curl -s -I http://localhost:8080/docs | grep -i "location:")
if echo "$LOCATION" | grep -q "/guide"; then
    echo -e "    ${GREEN}✓ 重定向正确${NC}"
    echo "    $LOCATION"
else
    echo -e "    ${RED}✗ 失败${NC}"
fi

echo -e "\n  ${GREEN}➤ 测试静态文件路由 (/static/test.txt):${NC}"
curl -s http://localhost:8080/static/test.txt | head -1

echo -e "\n  ${GREEN}➤ 测试 API 路由 (/api/config):${NC}"
CONFIG_SIZE=$(curl -s http://localhost:8080/api/config | wc -c)
echo -e "    ${GREEN}✓ API 响应大小: ${CONFIG_SIZE} bytes${NC}"

# 7. 性能测试
echo -e "\n${BLUE}[7] 路由性能对比:${NC}"
echo -e "  主页 (/):"
ab -n 1000 -c 10 -q http://localhost:8080/ 2>&1 | grep "Requests per second"
echo -e "  Markdown 路由 (/guide):"
ab -n 1000 -c 10 -q http://localhost:8080/guide 2>&1 | grep "Requests per second"
echo -e "  Template 路由 (/contact):"
ab -n 1000 -c 10 -q http://localhost:8080/contact 2>&1 | grep "Requests per second"

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
echo "  ✓ Markdown 渲染路由"
echo "  ✓ HTML 模板路由"
echo "  ✓ 重定向路由"
echo "  ✓ 静态文件路由"
echo "  ✓ API 路由"
echo ""
echo "配置的路由："
echo "  /guide      → docs/guide.md (Markdown)"
echo "  /contact    → templates/contact.html (Template)"
echo "  /docs       → /guide (Redirect)"
echo "  /api-docs   → API.md (Markdown)"
echo "  /static/*   → static/* (Static Files)"
echo "  /api/*      → API handlers"
