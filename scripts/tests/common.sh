#!/bin/bash
# YARHS Test Shared Module
# Provides colors, logging, assertions and other common functions

# Color definitions
export GREEN='\033[0;32m'
export BLUE='\033[0;34m'
export RED='\033[0;31m'
export YELLOW='\033[0;33m'
export NC='\033[0m'

# Test counters (exported as environment variables for sub-scripts to accumulate)
export PASS=${PASS:-0}
export FAIL=${FAIL:-0}

# URL configuration
export BASE_URL="http://127.0.0.1:8080"
export API_URL="http://127.0.0.1:8000"

# Log functions
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; PASS=$((PASS + 1)); }
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; FAIL=$((FAIL + 1)); }
log_section() { 
    echo -e "\n${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}  $1${NC}"
    echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

# Assertion function: check string contains
assert_contains() {
    local name="$1" content="$2" expected="$3"
    if echo "$content" | grep -q "$expected"; then
        log_pass "$name"
    else
        log_fail "$name (expected: $expected)"
    fi
}

# Assertion function: check HTTP status code
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

# Export functions for sub-scripts to use
export -f log_info log_pass log_fail log_section assert_contains assert_status
