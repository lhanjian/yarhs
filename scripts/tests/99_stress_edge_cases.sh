#!/bin/bash
# ============================================================
# Stress & Edge Case Tests
# 复杂配置更新、边界情况、并发测试
# 
# 这些测试用于发现那些"很难受的问题"：
# - 配置边界值
# - 并发更新冲突
# - 大量路由配置
# - 快速配置切换
# - 无效配置拒绝
# - 版本冲突检测
# ============================================================

log_section "99. Stress & Edge Case Tests"

# Create temp directory for this test
STRESS_DIR="/tmp/yarhs_stress_$$"
mkdir -p "$STRESS_DIR"

# ============================================================
# Part 1: 虚拟主机边界测试
# ============================================================
log_info "=== Part 1: VirtualHost Edge Cases ==="

# Test 1.1: 空域名列表（应该被拒绝）
RESPONSE=$(curl -s --max-time 10 -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "virtual_hosts": [{
                "name": "empty-domains",
                "domains": [],
                "routes": [{"name": "r1", "match": {"prefix": "/"}, "type": "dir", "path": "/tmp"}]
            }]
        }]
    }')
STATUS=$(echo "$RESPONSE" | jq -r '.status')
if [ "$STATUS" = "NACK" ]; then
    log_pass "Reject empty domains list"
else
    log_fail "Reject empty domains list (expected NACK, got: $STATUS)"
fi

# Test 1.2: 空名称（应该被拒绝）
RESPONSE=$(curl -s --max-time 10 -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "virtual_hosts": [{
                "name": "",
                "domains": ["test.local"],
                "routes": [{"name": "r1", "match": {"prefix": "/"}, "type": "dir", "path": "/tmp"}]
            }]
        }]
    }')
STATUS=$(echo "$RESPONSE" | jq -r '.status')
if [ "$STATUS" = "NACK" ]; then
    log_pass "Reject empty vhost name"
else
    log_fail "Reject empty vhost name (expected NACK, got: $STATUS)"
fi

# Test 1.3: 大量虚拟主机（100个）
log_info "Generating 100 virtual hosts..."
VHOSTS_JSON="["
for i in $(seq 1 100); do
    mkdir -p "$STRESS_DIR/site-$i"
    echo "Site $i Content" > "$STRESS_DIR/site-$i/index.html"
    if [ $i -gt 1 ]; then VHOSTS_JSON+=","; fi
    VHOSTS_JSON+='{
        "name": "site-'$i'",
        "domains": ["site-'$i'.local"],
        "routes": [{"name": "root", "match": {"prefix": "/"}, "type": "dir", "path": "'$STRESS_DIR'/site-'$i'"}]
    }'
done
VHOSTS_JSON+="]"

RESPONSE=$(curl -s --max-time 30 -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d "{\"resources\": [{\"virtual_hosts\": $VHOSTS_JSON}]}")
assert_json_field "Configure 100 virtual hosts" "$RESPONSE" ".status" "ACK"

# Test 1.4: 验证第50个虚拟主机能正常工作
SITE50_RESPONSE=$(curl -s --max-time 10 -H "Host: site-50.local" "$BASE_URL/index.html")
assert_contains "Route to site-50" "$SITE50_RESPONSE" "Site 50 Content"

# Test 1.5: 验证第100个虚拟主机能正常工作
SITE100_RESPONSE=$(curl -s --max-time 10 -H "Host: site-100.local" "$BASE_URL/index.html")
assert_contains "Route to site-100" "$SITE100_RESPONSE" "Site 100 Content"

# ============================================================
# Part 2: 复杂路由匹配测试
# ============================================================
log_info "=== Part 2: Complex Route Matching ==="

# 先清空之前的配置
curl -s --max-time 10 -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{"resources": [{"virtual_hosts": []}]}' > /dev/null

# 创建复杂的虚拟主机配置
mkdir -p "$STRESS_DIR/complex/api/v1" "$STRESS_DIR/complex/api/v2" "$STRESS_DIR/complex/static"
echo '{"version": "1.0"}' > "$STRESS_DIR/complex/api/v1/info.json"
echo '{"version": "2.0"}' > "$STRESS_DIR/complex/api/v2/info.json"
echo "Static Content" > "$STRESS_DIR/complex/static/page.html"

COMPLEX_CONFIG='{
    "resources": [{
        "virtual_hosts": [
            {
                "name": "api-versioned",
                "domains": ["api.complex.local"],
                "routes": [
                    {
                        "name": "api-v2",
                        "match": {"prefix": "/v2"},
                        "type": "dir",
                        "path": "'$STRESS_DIR'/complex/api/v2"
                    },
                    {
                        "name": "api-v1",
                        "match": {"prefix": "/v1"},
                        "type": "dir",
                        "path": "'$STRESS_DIR'/complex/api/v1"
                    },
                    {
                        "name": "api-redirect-latest",
                        "match": {"path": "/latest"},
                        "type": "redirect",
                        "target": "/v2/info.json",
                        "code": 302
                    },
                    {
                        "name": "api-default",
                        "match": {"prefix": "/"},
                        "type": "direct",
                        "status": 404,
                        "body": "{\"error\": \"Not Found\"}",
                        "content_type": "application/json"
                    }
                ]
            },
            {
                "name": "wildcard-test",
                "domains": ["*.wildcard.local"],
                "routes": [
                    {
                        "name": "wildcard-root",
                        "match": {"prefix": "/"},
                        "type": "dir",
                        "path": "'$STRESS_DIR'/complex/static"
                    }
                ]
            },
            {
                "name": "catch-all",
                "domains": ["*"],
                "routes": [
                    {
                        "name": "fallback",
                        "match": {"prefix": "/"},
                        "type": "direct",
                        "status": 421,
                        "body": "Misdirected Request"
                    }
                ]
            }
        ]
    }]
}'

RESPONSE=$(curl -s --max-time 10 -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d "$COMPLEX_CONFIG")
assert_json_field "Configure complex routes" "$RESPONSE" ".status" "ACK"

# Test 2.1: API v1 路由
API_V1=$(curl -s --max-time 10 -H "Host: api.complex.local" "$BASE_URL/v1/info.json")
assert_contains "API v1 route" "$API_V1" '"version": "1.0"'

# Test 2.2: API v2 路由
API_V2=$(curl -s --max-time 10 -H "Host: api.complex.local" "$BASE_URL/v2/info.json")
assert_contains "API v2 route" "$API_V2" '"version": "2.0"'

# Test 2.3: 重定向到最新版本
REDIRECT_STATUS=$(curl -sI --max-time 10 -H "Host: api.complex.local" "$BASE_URL/latest" | grep "HTTP" | cut -d' ' -f2 | tr -d '\r')
if [ "$REDIRECT_STATUS" = "302" ]; then
    log_pass "Redirect to latest API (HTTP 302)"
else
    log_fail "Redirect to latest API (expected 302, got: $REDIRECT_STATUS)"
fi

# Test 2.4: API 默认 404 响应
API_404=$(curl -s --max-time 10 -H "Host: api.complex.local" "$BASE_URL/unknown")
assert_contains "API custom 404" "$API_404" '"error": "Not Found"'

# Test 2.5: 通配符域名匹配
WILDCARD=$(curl -s --max-time 10 -H "Host: sub.wildcard.local" "$BASE_URL/page.html")
assert_contains "Wildcard domain match" "$WILDCARD" "Static Content"

WILDCARD2=$(curl -s --max-time 10 -H "Host: another.wildcard.local" "$BASE_URL/page.html")
assert_contains "Wildcard domain match 2" "$WILDCARD2" "Static Content"

# Test 2.6: Catch-all 处理未知域名
CATCHALL=$(curl -s --max-time 10 -H "Host: unknown.domain.local" "$BASE_URL/")
assert_contains "Catch-all for unknown domain" "$CATCHALL" "Misdirected Request"

# ============================================================
# Part 3: 版本冲突测试（乐观锁）
# ============================================================
log_info "=== Part 3: Version Conflict (Optimistic Locking) ==="

# 获取当前版本
CURRENT=$(curl -s --max-time 10 "$API_URL/v1/discovery:vhosts")
CURRENT_VERSION=$(echo "$CURRENT" | jq -r '.version_info')
log_info "Current vhosts version: $CURRENT_VERSION"

# Test 3.1: 使用正确版本更新
RESPONSE=$(curl -s --max-time 10 -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d "{
        \"version_info\": \"$CURRENT_VERSION\",
        \"resources\": [{\"virtual_hosts\": [{
            \"name\": \"version-test\",
            \"domains\": [\"version.local\"],
            \"routes\": [{\"name\": \"r\", \"match\": {\"prefix\": \"/\"}, \"type\": \"direct\", \"status\": 200, \"body\": \"Version Test OK\"}]
        }]}]
    }")
assert_json_field "Update with correct version" "$RESPONSE" ".status" "ACK"

# 验证更新后的配置实际生效
VERSION_TEST=$(curl -s --max-time 10 -H "Host: version.local" "$BASE_URL/")
assert_contains "Version update applied" "$VERSION_TEST" "Version Test OK"

# 获取新版本
NEW_VERSION=$(echo "$RESPONSE" | jq -r '.version_info')
log_info "New vhosts version: $NEW_VERSION"

# Test 3.2: 使用旧版本更新（应该冲突）
CONFLICT_RESPONSE=$(curl -s --max-time 10 -w "\n%{http_code}" -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d "{
        \"version_info\": \"$CURRENT_VERSION\",
        \"resources\": [{\"virtual_hosts\": []}]
    }")
CONFLICT_STATUS=$(echo "$CONFLICT_RESPONSE" | tail -1)
if [ "$CONFLICT_STATUS" = "409" ]; then
    log_pass "Reject stale version (HTTP 409)"
else
    log_fail "Reject stale version (expected 409, got: $CONFLICT_STATUS)"
fi

# ============================================================
# Part 4: 快速配置切换测试
# ============================================================
log_info "=== Part 4: Rapid Config Switching ==="

# 准备两个不同的配置
CONFIG_A='{
    "resources": [{
        "virtual_hosts": [{
            "name": "config-a",
            "domains": ["test.local"],
            "routes": [{"name": "a", "match": {"prefix": "/"}, "type": "direct", "status": 200, "body": "Config A"}]
        }]
    }]
}'

CONFIG_B='{
    "resources": [{
        "virtual_hosts": [{
            "name": "config-b",
            "domains": ["test.local"],
            "routes": [{"name": "b", "match": {"prefix": "/"}, "type": "direct", "status": 200, "body": "Config B"}]
        }]
    }]
}'

# 快速切换 20 次
SWITCH_ERRORS=0
for i in $(seq 1 10); do
    # 切换到 A
    RESP_A=$(curl -s --max-time 10 -X POST "$API_URL/v1/discovery:vhosts" -H "Content-Type: application/json" -d "$CONFIG_A")
    if [ "$(echo "$RESP_A" | jq -r '.status')" != "ACK" ]; then
        SWITCH_ERRORS=$((SWITCH_ERRORS + 1))
    fi
    
    # 验证 A
    CONTENT=$(curl -s --max-time 10 -H "Host: test.local" "$BASE_URL/")
    if ! echo "$CONTENT" | grep -q "Config A"; then
        SWITCH_ERRORS=$((SWITCH_ERRORS + 1))
    fi
    
    # 切换到 B
    RESP_B=$(curl -s --max-time 10 -X POST "$API_URL/v1/discovery:vhosts" -H "Content-Type: application/json" -d "$CONFIG_B")
    if [ "$(echo "$RESP_B" | jq -r '.status')" != "ACK" ]; then
        SWITCH_ERRORS=$((SWITCH_ERRORS + 1))
    fi
    
    # 验证 B
    CONTENT=$(curl -s --max-time 10 -H "Host: test.local" "$BASE_URL/")
    if ! echo "$CONTENT" | grep -q "Config B"; then
        SWITCH_ERRORS=$((SWITCH_ERRORS + 1))
    fi
done

if [ "$SWITCH_ERRORS" -eq 0 ]; then
    log_pass "Rapid config switching (20 switches, 0 errors)"
else
    log_fail "Rapid config switching ($SWITCH_ERRORS errors)"
fi

# ============================================================
# Part 5: 清理和状态恢复
# ============================================================
log_info "=== Part 5: Cleanup & State Recovery ==="

# 清空虚拟主机配置
CLEANUP=$(curl -s --max-time 10 -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{"resources": [{"virtual_hosts": []}]}')
assert_json_field "Cleanup virtual hosts" "$CLEANUP" ".status" "ACK"

# 验证回退到传统路由
FALLBACK=$(curl -s --max-time 10 "$BASE_URL/static/test.txt")
assert_contains "Fallback to legacy routes after cleanup" "$FALLBACK" "Hello"

# 清理临时文件
rm -rf "$STRESS_DIR"

log_info "Stress edge case tests completed"
log_info "NOTE: Concurrent and robustness tests are in 98_concurrent_robustness.sh"
