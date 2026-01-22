#!/bin/bash
# YARHS 统一测试脚本
# 入口脚本：运行 cargo test + 集成测试
set -e

# 颜色定义
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; }
log_section() { echo -e "\n${YELLOW}════════════════════════════════════════${NC}"; echo -e "${YELLOW}  $1${NC}"; echo -e "${YELLOW}════════════════════════════════════════${NC}"; }

# 切换到项目根目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

echo ""
echo "╔════════════════════════════════════════╗"
echo "║       YARHS 统一测试套件               ║"
echo "╚════════════════════════════════════════╝"
echo ""

ERRORS=0

# ============================================
# 阶段 1: 单元测试
# ============================================
log_section "阶段 1/3: 单元测试 (cargo test)"

if cargo test --all 2>&1; then
    log_pass "单元测试通过"
else
    log_fail "单元测试失败"
    ERRORS=$((ERRORS + 1))
fi

# ============================================
# 阶段 2: 构建 Release 版本
# ============================================
log_section "阶段 2/3: 构建 Release 版本"

if cargo build --release 2>&1; then
    log_pass "Release 构建成功"
else
    log_fail "Release 构建失败"
    exit 1
fi

# ============================================
# 阶段 3: 集成测试
# ============================================
log_section "阶段 3/3: 集成测试"

chmod +x "$SCRIPT_DIR/integration_tests.sh"
if "$SCRIPT_DIR/integration_tests.sh"; then
    log_pass "集成测试通过"
else
    log_fail "集成测试失败"
    ERRORS=$((ERRORS + 1))
fi

# ============================================
# 结果汇总
# ============================================
echo ""
echo "╔════════════════════════════════════════╗"
echo "║           最终结果                     ║"
echo "╚════════════════════════════════════════╝"
echo ""

if [ "$ERRORS" -eq 0 ]; then
    echo -e "${GREEN}✅ 所有测试通过！${NC}"
    exit 0
else
    echo -e "${RED}❌ $ERRORS 个阶段失败${NC}"
    exit 1
fi
