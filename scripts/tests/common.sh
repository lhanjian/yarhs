#!/bin/bash
# YARHS 测试共享模块
# 提供颜色、日志、断言等通用函数

# 颜色定义
export GREEN='\033[0;32m'
export BLUE='\033[0;34m'
export RED='\033[0;31m'
export YELLOW='\033[0;33m'
export NC='\033[0m'

# 测试计数 (导出为环境变量以便子脚本累加)
export PASS=${PASS:-0}
export FAIL=${FAIL:-0}

# URL 配置
export BASE_URL="http://127.0.0.1:8080"
export API_URL="http://127.0.0.1:8000"

# 日志函数
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; PASS=$((PASS + 1)); }
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; FAIL=$((FAIL + 1)); }
log_section() { 
    echo -e "\n${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}  $1${NC}"
    echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

# 断言函数: 检查字符串包含
assert_contains() {
    local name="$1" content="$2" expected="$3"
    if echo "$content" | grep -q "$expected"; then
        log_pass "$name"
    else
        log_fail "$name (expected: $expected)"
    fi
}

# 断言函数: 检查 HTTP 状态码
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

# 导出函数供子脚本使用
export -f log_info log_pass log_fail log_section assert_contains assert_status
