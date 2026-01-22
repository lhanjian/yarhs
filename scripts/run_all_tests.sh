#!/bin/bash
# YARHS Unified Test Script
# Entry script: runs cargo test + integration tests
set -e

# Color definitions
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; }
log_section() { echo -e "\n${YELLOW}════════════════════════════════════════${NC}"; echo -e "${YELLOW}  $1${NC}"; echo -e "${YELLOW}════════════════════════════════════════${NC}"; }

# Change to project root directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

echo ""
echo "╔════════════════════════════════════════╗"
echo "║       YARHS Unified Test Suite            ║"
echo "╚════════════════════════════════════════╝"
echo ""

ERRORS=0

# ============================================
# Phase 1: Unit Tests
# ============================================
log_section "Phase 1/3: Unit Tests (cargo test)"

if cargo test --all 2>&1; then
    log_pass "Unit tests passed"
else
    log_fail "Unit tests failed"
    ERRORS=$((ERRORS + 1))
fi

# ============================================
# Phase 2: Build Release Version
# ============================================
log_section "Phase 2/3: Build Release Version"

if cargo build --release 2>&1; then
    log_pass "Release build successful"
else
    log_fail "Release build failed"
    exit 1
fi

# ============================================
# Phase 3: Integration Tests
# ============================================
log_section "Phase 3/3: Integration Tests"

chmod +x "$SCRIPT_DIR/integration_tests.sh"
if "$SCRIPT_DIR/integration_tests.sh"; then
    log_pass "Integration tests passed"
else
    log_fail "Integration tests failed"
    ERRORS=$((ERRORS + 1))
fi

# ============================================
# Results Summary
# ============================================
echo ""
echo "╔════════════════════════════════════════╗"
echo "║           Final Results                 ║"
echo "╚════════════════════════════════════════╝"
echo ""

if [ "$ERRORS" -eq 0 ]; then
    echo -e "${GREEN}✅ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ $ERRORS phase(s) failed${NC}"
    exit 1
fi
