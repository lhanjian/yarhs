#!/bin/bash
# xDS API 测试
# 此脚本由 integration_tests.sh 调用，common.sh 已加载

log_section "6. xDS API"

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
