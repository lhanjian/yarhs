#!/bin/bash
# YARHS 集成测试主入口
# 启动服务器并执行所有测试模块
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

# 加载共享模块
source "$SCRIPT_DIR/tests/common.sh"

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

echo ""
echo "╔════════════════════════════════════════╗"
echo "║       YARHS 集成测试套件               ║"
echo "╚════════════════════════════════════════╝"
echo ""

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
# 执行各测试模块
# ============================================
for test_file in "$SCRIPT_DIR/tests"/[0-9]*.sh; do
    if [ -f "$test_file" ]; then
        source "$test_file"
    fi
done

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
echo "║    ✓ Range 请求 (断点续传)            ║"
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
