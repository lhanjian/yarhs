#!/bin/bash
# 服务器功能测试脚本

BASE_URL="http://127.0.0.1:8080"

echo "======================================"
echo "🧪 Rust Web Server 功能测试"
echo "======================================"
echo ""

# 测试静态文件
echo "1️⃣  静态文件服务测试"
echo "   TXT: $(curl -s $BASE_URL/static/test.txt | head -c 25)..."
echo "   HTML: $(curl -s $BASE_URL/static/test.html | grep -o '<h1>.*</h1>')"
echo "   JSON: $(curl -s $BASE_URL/static/data.json | head -c 40)..."
echo "   CSS: $(curl -s $BASE_URL/static/style.css | head -1)"
echo ""

# 测试请求体限制
echo "2️⃣  请求体大小限制测试"
HTTP_CODE=$(curl -s -w "%{http_code}" -X POST -H "Content-Length: 20000000" $BASE_URL/ -o /dev/null)
echo "   超大请求 (20MB): HTTP $HTTP_CODE $([ "$HTTP_CODE" = "413" ] && echo "✓" || echo "✗")"
echo ""

# 测试性能 - 并发
echo "3️⃣  并发性能测试"
START=$(date +%s%N)
for i in {1..20}; do
  curl -s $BASE_URL/ > /dev/null &
done
wait
END=$(date +%s%N)
ELAPSED=$(( (END - START) / 1000000 ))
echo "   20个并发请求: ${ELAPSED}ms"
echo ""

# 测试路由
echo "4️⃣  路由功能测试"
echo "   主页 (/): $(curl -s $BASE_URL/ | grep -o '<title>.*</title>' | head -1)"
echo "   模板 (/template): $(curl -s -w "HTTP %{http_code}" $BASE_URL/template -o /dev/null)"
echo "   Favicon: $(curl -s -I $BASE_URL/favicon.svg | grep -i 'content-type' | cut -d' ' -f2)"
echo "   配置API: $(curl -s $BASE_URL/api/config | grep -o '"level"' | wc -l) fields"
echo ""

echo "======================================"
echo "✅ 测试完成"
echo "======================================"
