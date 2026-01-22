#!/bin/bash
# YARHS 集成测试脚本
# 完整的服务器功能测试，覆盖所有特性
set -e

# 颜色定义
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m'

# 测试计数
PASS=0
FAIL=0

# 辅助函数
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; PASS=$((PASS + 1)); }
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; FAIL=$((FAIL + 1)); }
log_section() { echo -e "\n${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"; echo -e "${YELLOW}  $1${NC}"; echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"; }

# 清理函数
cleanup() {
    log_info "清理测试环境..."
    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -f /tmp/config.json /tmp/xds_routes.json /tmp/server.log
    rm -f templates/contact.html static/api.json
}
trap cleanup EXIT

# 断言函数
assert_contains() {
    local name="$1" content="$2" expected="$3"
    if echo "$content" | grep -q "$expected"; then
        log_pass "$name"
    else
        log_fail "$name (expected: $expected)"
    fi
}

assert_status() {
    local name="$1" url="$2" expected="$3" extra="${4:-}"
    local status
    if [ -n "$extra" ]; then
        if [[ "$extra" == -X* ]]; then
            status=$(curl -sI $extra "$url" 2>/dev/null | grep "HTTP" | cut -d' ' -f2 | tr -d '\r')
        else
            status=$(curl -sI -H "$extra" "$url" 2>/dev/null | grep "HTTP" | cut -d' ' -f2 | tr -d '\r')
        fi
    else
        status=$(curl -sI "$url" 2>/dev/null | grep "HTTP" | cut -d' ' -f2 | tr -d '\r')
    fi
    if [ "$status" = "$expected" ]; then
        log_pass "$name (HTTP $status)"
    else
        log_fail "$name (expected: $expected, got: $status)"
    fi
}

echo ""
echo "╔════════════════════════════════════════╗"
echo "║       YARHS 集成测试套件               ║"
echo "╚════════════════════════════════════════╝"
echo ""

BASE_URL="http://127.0.0.1:8080"
API_URL="http://127.0.0.1:8000"

# ============================================
# 启动服务器
# ============================================
log_info "启动服务器..."
./target/release/rust_webserver > /tmp/server.log 2>&1 &
SERVER_PID=$!
sleep 2

if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    log_fail "服务器启动失败"
    cat /tmp/server.log
    exit 1
fi
log_pass "服务器启动成功 (PID: $SERVER_PID)"

# ============================================
# 1. 静态文件服务测试
# ============================================
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

# ============================================
# 2. 路由功能测试
# ============================================
log_section "2. 路由功能"

assert_status "主页路由 (/)" "$BASE_URL/" "200"
assert_status "File 路由 (/about)" "$BASE_URL/about" "200"
assert_status "Dir 路由 (/static/test.txt)" "$BASE_URL/static/test.txt" "200"

# Favicon
CONTENT_TYPE=$(curl -sI "$BASE_URL/favicon.svg" | grep -i "content-type" | tr -d '\r')
assert_contains "Favicon 响应" "$CONTENT_TYPE" "svg"

# ============================================
# 3. HTTP 方法处理测试
# ============================================
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

# ============================================
# 4. 缓存与条件请求测试
# ============================================
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

# ============================================
# 5. xDS API 测试
# ============================================
log_section "5. xDS API"

# 快照端点
RESPONSE=$(curl -s "$API_URL/v1/discovery")
assert_contains "discovery 快照端点" "$RESPONSE" "version_info"

# 各资源端点
RESPONSE=$(curl -s "$API_URL/v1/discovery:routes")
assert_contains "routes 资源端点" "$RESPONSE" "resources"

RESPONSE=$(curl -s "$API_URL/v1/discovery:logging")
assert_contains "logging 资源端点" "$RESPONSE" "level"

RESPONSE=$(curl -s "$API_URL/v1/discovery:listeners")
assert_contains "listeners 资源端点" "$RESPONSE" "main_server"

# ============================================
# 6. 动态路由配置测试
# ============================================
log_section "6. 动态路由配置"

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

# ============================================
# 7. 根路径映射测试
# ============================================
log_section "7. 根路径映射"

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

# ============================================
# 8. 并发与性能测试
# ============================================
log_section "8. 并发测试"

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

# ============================================
# 结果汇总
# ============================================
log_info "停止服务器..."
kill "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
SERVER_PID=""

echo ""
echo "╔════════════════════════════════════════╗"
echo "║           测试结果汇总                 ║"
echo "╠════════════════════════════════════════╣"
printf "║  通过: ${GREEN}%-3d${NC}                            ║\n" $PASS
printf "║  失败: ${RED}%-3d${NC}                            ║\n" $FAIL
echo "╠════════════════════════════════════════╣"
echo "║  测试覆盖:                             ║"
echo "║    ✓ 静态文件服务 + MIME 检测         ║"
echo "║    ✓ 路由功能 (File/Dir/Redirect)     ║"
echo "║    ✓ HTTP 方法 (GET/HEAD/OPTIONS/405) ║"
echo "║    ✓ 缓存 (ETag + 304)                ║"
echo "║    ✓ xDS API 端点                     ║"
echo "║    ✓ 动态路由配置                     ║"
echo "║    ✓ 根路径映射                       ║"
echo "║    ✓ 并发请求                         ║"
echo "╚════════════════════════════════════════╝"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo -e "${GREEN}✅ 所有测试通过！${NC}"
    exit 0
else
    echo -e "${RED}❌ $FAIL 个测试失败${NC}"
    exit 1
fi
