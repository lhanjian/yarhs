#!/bin/bash
# ============================================================
# Concurrent & Robustness Tests (Part 2)
# 并发测试、输入验证、特殊字符处理
# 
# 从 99_stress_edge_cases.sh 拆分出来的测试：
# - Part 5: 并发更新测试
# - Part 6: 无效输入处理
# - Part 7: 特殊字符和编码
# - Part 8: 资源清理
# ============================================================

log_section "98. Concurrent & Robustness Tests"

# Create temp directory for this test
ROBUST_DIR="/tmp/yarhs_robust_$$"
mkdir -p "$ROBUST_DIR"

# ============================================================
# Part 1: 并发更新测试
# ============================================================
log_info "=== Part 1: Concurrent Updates ==="

# 启动 5 个并发更新进程
CONCURRENT_RESULTS="$ROBUST_DIR/concurrent_results"
mkdir -p "$CONCURRENT_RESULTS"

for i in $(seq 1 5); do
    (
        for j in $(seq 1 5); do
            RESP=$(curl -s -X POST "$API_URL/v1/discovery:vhosts" \
                -H "Content-Type: application/json" \
                -d "{
                    \"resources\": [{
                        \"virtual_hosts\": [{
                            \"name\": \"concurrent-$i-$j\",
                            \"domains\": [\"concurrent.local\"],
                            \"routes\": [{\"name\": \"r\", \"match\": {\"prefix\": \"/\"}, \"type\": \"direct\", \"status\": 200, \"body\": \"$i-$j\"}]
                        }]
                    }]
                }")
            echo "$RESP" >> "$CONCURRENT_RESULTS/worker-$i.log"
        done
    ) &
done

# 等待所有进程完成
wait

# 检查结果：至少要有一些成功的
TOTAL_REQUESTS=$(cat "$CONCURRENT_RESULTS"/*.log 2>/dev/null | grep -c "status" || echo "0")
ACK_COUNT=$(cat "$CONCURRENT_RESULTS"/*.log 2>/dev/null | grep -c '"ACK"' || echo "0")
NACK_COUNT=$(cat "$CONCURRENT_RESULTS"/*.log 2>/dev/null | grep -c '"NACK"' || echo "0")

log_info "Concurrent results: $ACK_COUNT ACK, $NACK_COUNT NACK out of $TOTAL_REQUESTS"
if [ "$ACK_COUNT" -gt 0 ]; then
    log_pass "Concurrent updates handled ($ACK_COUNT successful)"
else
    log_fail "Concurrent updates (no successful updates)"
fi

# ============================================================
# Part 2: 无效 JSON 和恶意输入测试
# ============================================================
log_info "=== Part 2: Invalid Input Handling ==="

# Test 2.1: 无效 JSON
INVALID_JSON=$(curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{invalid json}')
assert_json_has "Reject invalid JSON" "$INVALID_JSON" ".error_detail"

# Test 2.2: 缺少必需字段
MISSING_FIELD=$(curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{"resources": [{"virtual_hosts": [{"name": "test"}]}]}')
STATUS=$(echo "$MISSING_FIELD" | jq -r '.status')
if [ "$STATUS" = "NACK" ]; then
    log_pass "Reject missing required field (domains)"
else
    log_fail "Reject missing required field (expected NACK)"
fi

# Test 2.3: 无效的路由类型
INVALID_TYPE=$(curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "virtual_hosts": [{
                "name": "test",
                "domains": ["test.local"],
                "routes": [{"name": "r", "match": {"prefix": "/"}, "type": "invalid_type", "path": "/tmp"}]
            }]
        }]
    }')
STATUS=$(echo "$INVALID_TYPE" | jq -r '.status')
if [ "$STATUS" = "NACK" ]; then
    log_pass "Reject invalid route type"
else
    log_fail "Reject invalid route type (expected NACK)"
fi

# Test 2.4: 超长域名
LONG_DOMAIN=$(printf 'a%.0s' {1..300})
LONG_DOMAIN_RESP=$(curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d "{
        \"resources\": [{
            \"virtual_hosts\": [{
                \"name\": \"long\",
                \"domains\": [\"$LONG_DOMAIN.local\"],
                \"routes\": [{\"name\": \"r\", \"match\": {\"prefix\": \"/\"}, \"type\": \"direct\", \"status\": 200, \"body\": \"ok\"}]
            }]
        }]
    }")
# 应该接受但可能匹配不到
if echo "$LONG_DOMAIN_RESP" | jq -e '.status' >/dev/null 2>&1; then
    log_pass "Handle very long domain name"
else
    log_fail "Handle very long domain name"
fi

# Test 2.5: 空 resources 数组
EMPTY_RESOURCES=$(curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{"resources": []}')
assert_json_has "Reject empty resources" "$EMPTY_RESOURCES" ".error_detail"

# Test 2.6: resources 不是数组
NON_ARRAY=$(curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{"resources": "not an array"}')
assert_json_has "Reject non-array resources" "$NON_ARRAY" ".error_detail"

# ============================================================
# Part 3: 特殊字符和编码测试
# ============================================================
log_info "=== Part 3: Special Characters ==="

# Test 3.1: 路径中包含特殊字符
mkdir -p "$ROBUST_DIR/special/path with spaces"
echo "Special Content" > "$ROBUST_DIR/special/path with spaces/file.txt"

SPECIAL_CONFIG='{
    "resources": [{
        "virtual_hosts": [{
            "name": "special",
            "domains": ["special.local"],
            "routes": [{
                "name": "special-path",
                "match": {"prefix": "/"},
                "type": "dir",
                "path": "'$ROBUST_DIR'/special/path with spaces"
            }]
        }]
    }]
}'

RESPONSE=$(curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d "$SPECIAL_CONFIG")
assert_json_field "Config with spaces in path" "$RESPONSE" ".status" "ACK"

# Test 3.2: 验证带空格路径可以访问
SPECIAL_CONTENT=$(curl -s -H "Host: special.local" "$BASE_URL/file.txt")
assert_contains "Access file in path with spaces" "$SPECIAL_CONTENT" "Special Content"

# Test 3.3: Unicode 域名（应该按原样处理）
UNICODE_CONFIG='{
    "resources": [{
        "virtual_hosts": [{
            "name": "unicode",
            "domains": ["中文.local", "日本語.local"],
            "routes": [{"name": "u", "match": {"prefix": "/"}, "type": "direct", "status": 200, "body": "Unicode OK"}]
        }]
    }]
}'

RESPONSE=$(curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d "$UNICODE_CONFIG")
assert_json_field "Config with unicode domains" "$RESPONSE" ".status" "ACK"

# Test 3.4: 路径中包含中文
mkdir -p "$ROBUST_DIR/中文目录"
echo "Chinese Path Content" > "$ROBUST_DIR/中文目录/文件.txt"

CHINESE_PATH_CONFIG='{
    "resources": [{
        "virtual_hosts": [{
            "name": "chinese-path",
            "domains": ["chinese.local"],
            "routes": [{
                "name": "chinese",
                "match": {"prefix": "/"},
                "type": "dir",
                "path": "'$ROBUST_DIR'/中文目录"
            }]
        }]
    }]
}'

RESPONSE=$(curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d "$CHINESE_PATH_CONFIG")
assert_json_field "Config with chinese path" "$RESPONSE" ".status" "ACK"

# ============================================================
# Part 4: 边界值测试
# ============================================================
log_info "=== Part 4: Boundary Values ==="

# Test 4.1: 单字符域名
SINGLE_CHAR=$(curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "virtual_hosts": [{
                "name": "single",
                "domains": ["a"],
                "routes": [{"name": "r", "match": {"prefix": "/"}, "type": "direct", "status": 200, "body": "single"}]
            }]
        }]
    }')
assert_json_field "Single char domain" "$SINGLE_CHAR" ".status" "ACK"

# Test 4.2: 空路由列表（应该接受，但匹配不到任何请求）
EMPTY_ROUTES=$(curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "virtual_hosts": [{
                "name": "empty-routes",
                "domains": ["empty.local"],
                "routes": []
            }]
        }]
    }')
assert_json_field "Empty routes list" "$EMPTY_ROUTES" ".status" "ACK"

# 验证空路由返回 404
EMPTY_ROUTE_RESP=$(curl -s -w "%{http_code}" -H "Host: empty.local" "$BASE_URL/" -o /dev/null)
if [ "$EMPTY_ROUTE_RESP" = "404" ]; then
    log_pass "Empty routes returns 404"
else
    log_fail "Empty routes returns 404 (got: $EMPTY_ROUTE_RESP)"
fi

# Test 4.3: 多个相同域名（后者覆盖）
DUPLICATE_DOMAINS=$(curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "virtual_hosts": [
                {
                    "name": "first",
                    "domains": ["dup.local"],
                    "routes": [{"name": "r", "match": {"prefix": "/"}, "type": "direct", "status": 200, "body": "first"}]
                },
                {
                    "name": "second",
                    "domains": ["dup.local"],
                    "routes": [{"name": "r", "match": {"prefix": "/"}, "type": "direct", "status": 200, "body": "second"}]
                }
            ]
        }]
    }')
assert_json_field "Duplicate domains config" "$DUPLICATE_DOMAINS" ".status" "ACK"

# 验证第一个匹配的被使用
DUP_RESP=$(curl -s -H "Host: dup.local" "$BASE_URL/")
assert_contains "First matching vhost used" "$DUP_RESP" "first"

# ============================================================
# Part 5: 资源清理和状态恢复
# ============================================================
log_info "=== Part 5: Cleanup & State Verification ==="

# 清空虚拟主机配置
CLEANUP=$(curl -s -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{"resources": [{"virtual_hosts": []}]}')
assert_json_field "Cleanup virtual hosts" "$CLEANUP" ".status" "ACK"

# 验证回退到传统路由
FALLBACK=$(curl -s "$BASE_URL/static/test.txt")
assert_contains "Fallback to legacy routes after cleanup" "$FALLBACK" "Hello"

# 验证 API 快照正常
SNAPSHOT=$(curl -s "$API_URL/v1/discovery")
assert_json_has "Snapshot still accessible" "$SNAPSHOT" ".resources.virtual_hosts"

# 验证版本号递增
VHOSTS_VERSION=$(curl -s "$API_URL/v1/discovery:vhosts" | jq -r '.version_info')
if [ -n "$VHOSTS_VERSION" ] && [ "$VHOSTS_VERSION" != "null" ]; then
    log_pass "Version info available: $VHOSTS_VERSION"
else
    log_fail "Version info not available"
fi

# 清理临时文件
rm -rf "$ROBUST_DIR"

log_info "Concurrent & robustness tests completed"
