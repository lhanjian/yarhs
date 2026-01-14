#!/bin/bash
# 集成测试脚本 - 仅运行服务器集成测试（不含 cargo test）
set -e

# 颜色定义
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

# 错误计数
ERRORS=0

# 辅助函数
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_error() { echo -e "${RED}[FAIL]${NC} $1"; ERRORS=$((ERRORS + 1)); }

# 清理函数
cleanup() {
    log_info "清理测试环境..."
    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    if [ -f config.toml.bak ]; then
        mv config.toml.bak config.toml
    fi
    rm -f /tmp/config.json /tmp/xds_routes.json /tmp/server.log
    rm -f templates/contact.html static/api.json
}

trap cleanup EXIT

echo ""
echo "========================================"
echo "   YARHS 集成测试"
echo "========================================"
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
    log_error "服务器启动失败"
    cat /tmp/server.log
    exit 1
fi
log_success "服务器启动成功 (PID: $SERVER_PID)"

# ============================================
# 基础功能测试
# ============================================
log_info "测试静态文件服务..."
RESPONSE=$(curl -s "$BASE_URL/static/test.txt")
if echo "$RESPONSE" | grep -q "Hello"; then
    log_success "静态文件服务正常"
else
    log_error "静态文件服务失败"
fi

log_info "测试路由功能..."
STATUS=$(curl -s -w "%{http_code}" "$BASE_URL/" -o /dev/null)
if [ "$STATUS" = "200" ]; then
    log_success "主页路由正常 (HTTP $STATUS)"
else
    log_error "主页路由失败 (HTTP $STATUS)"
fi

STATUS=$(curl -s -w "%{http_code}" "$BASE_URL/about" -o /dev/null)
if [ "$STATUS" = "200" ]; then
    log_success "File 路由正常 (HTTP $STATUS)"
else
    log_error "File 路由失败 (HTTP $STATUS)"
fi

log_info "测试 xDS API..."
RESPONSE=$(curl -s "$API_URL/v1/discovery")
if echo "$RESPONSE" | grep -q "version_info"; then
    log_success "xDS discovery API 正常"
else
    log_error "xDS discovery API 失败"
fi

RESPONSE=$(curl -s "$API_URL/v1/discovery:routes")
if echo "$RESPONSE" | grep -q "resources"; then
    log_success "xDS routes API 正常"
else
    log_error "xDS routes API 失败"
fi

log_info "测试 ETag/304..."
ETAG=$(curl -sI "$BASE_URL/static/test.txt" | grep -i "etag:" | cut -d' ' -f2 | tr -d '\r')
if [ -n "$ETAG" ]; then
    log_success "ETag 响应头正常: $ETAG"
    STATUS=$(curl -sI -H "If-None-Match: $ETAG" "$BASE_URL/static/test.txt" | grep "HTTP" | cut -d' ' -f2)
    if [ "$STATUS" = "304" ]; then
        log_success "304 Not Modified 正常"
    else
        log_error "304 响应失败 (HTTP $STATUS)"
    fi
else
    log_error "ETag 响应头缺失"
fi

log_info "测试请求体大小限制..."
STATUS=$(curl -s -w "%{http_code}" -X POST -H "Content-Length: 20000000" "$BASE_URL/" -o /dev/null)
if [ "$STATUS" = "413" ]; then
    log_success "请求体大小限制正常 (HTTP $STATUS)"
else
    log_error "请求体大小限制失败 (HTTP $STATUS)"
fi

# ============================================
# 动态路由配置测试
# ============================================
log_info "创建测试文件..."
mkdir -p templates static
cat > templates/contact.html << 'EOF'
<!DOCTYPE html>
<html>
<head><title>Contact Us</title></head>
<body><h1>Contact Us</h1></body>
</html>
EOF
echo '{"name": "test", "version": "1.0"}' > static/api.json
log_success "测试文件创建完成"

log_info "获取当前路由配置..."
curl -s "$API_URL/v1/discovery:routes" > /tmp/config.json

log_info "添加自定义路由..."
jq '{
  resources: [
    {
      favicon_paths: .resources[0].favicon_paths,
      index_files: .resources[0].index_files,
      custom_routes: ((.resources[0].custom_routes // {}) + {
        "/contact": {type: "file", path: "templates/contact.html"},
        "/api-spec": {type: "file", path: "static/api.json"},
        "/docs": {type: "redirect", target: "/about"}
      })
    }
  ]
}' /tmp/config.json > /tmp/xds_routes.json

RESPONSE=$(curl -s -X POST "$API_URL/v1/discovery:routes" \
    -H "Content-Type: application/json" \
    -d @/tmp/xds_routes.json)

if echo "$RESPONSE" | grep -q '"status": *"ACK"'; then
    log_success "路由配置更新成功"
else
    log_error "路由配置更新失败: $RESPONSE"
fi

log_info "测试动态添加的路由..."

RESPONSE=$(curl -s "$BASE_URL/contact")
if echo "$RESPONSE" | grep -q "Contact Us"; then
    log_success "File 路由 (/contact) 正常"
else
    log_error "File 路由 (/contact) 失败"
fi

RESPONSE=$(curl -s "$BASE_URL/api-spec")
if echo "$RESPONSE" | grep -q "version"; then
    log_success "JSON 路由 (/api-spec) 正常"
else
    log_error "JSON 路由 (/api-spec) 失败"
fi

LOCATION=$(curl -s -I "$BASE_URL/docs" | grep -i "location:" | tr -d '\r')
if echo "$LOCATION" | grep -q "/about"; then
    log_success "Redirect 路由 (/docs) 正常"
else
    log_error "Redirect 路由 (/docs) 失败"
fi

# ============================================
# 结果汇总
# ============================================
log_info "停止服务器..."
kill "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
SERVER_PID=""
log_success "服务器已停止"

echo ""
echo "========================================"
echo "   测试结果汇总"
echo "========================================"
echo ""

if [ "$ERRORS" -eq 0 ]; then
    echo -e "${GREEN}✓ 所有测试通过！${NC}"
    exit 0
else
    echo -e "${RED}✗ $ERRORS 个测试失败${NC}"
    exit 1
fi
