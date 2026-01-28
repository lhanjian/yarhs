#!/bin/bash
# 98 号测试调试脚本
set -x  # 启用详细输出

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

BASE_URL="http://127.0.0.1:8080"
API_URL="http://127.0.0.1:9090"

# 清理函数
cleanup() {
    echo "清理中..."
    if [ -n "$SERVER_PID" ]; then
        kill -9 "$SERVER_PID" 2>/dev/null || true
    fi
    pkill -9 -f rust_webserver 2>/dev/null || true
    rm -rf /tmp/yarhs_robust_debug
}
trap cleanup EXIT

# 先杀掉可能存在的进程
pkill -9 -f rust_webserver 2>/dev/null || true
sleep 1

echo "=== 启动服务器 ==="
./target/release/rust_webserver > /tmp/server_debug.log 2>&1 &
SERVER_PID=$!
echo "Server PID: $SERVER_PID"
sleep 2

# 验证服务器启动
if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "服务器启动失败！"
    cat /tmp/server_debug.log
    exit 1
fi

echo "=== 测试基本连接 ==="
curl -s --max-time 5 "$BASE_URL/static/test.txt"
echo ""

echo "=== 测试 API 连接 ==="
curl -s --max-time 5 "$API_URL/v1/discovery" | head -c 200
echo ""

echo "=== Part 1: 并发更新测试 (单个请求先测试) ==="
ROBUST_DIR="/tmp/yarhs_robust_debug"
mkdir -p "$ROBUST_DIR"

# 先发一个请求看看
echo "发送单个测试请求..."
SINGLE_RESP=$(curl -s --max-time 10 -X POST "$API_URL/v1/discovery:vhosts" \
    -H "Content-Type: application/json" \
    -d '{
        "resources": [{
            "virtual_hosts": [{
                "name": "test-single",
                "domains": ["test.local"],
                "routes": [{"name": "r", "match": {"prefix": "/"}, "type": "direct", "status": 200, "body": "test"}]
            }]
        }]
    }')
echo "单个请求响应: $SINGLE_RESP"

echo ""
echo "=== Part 1: 并发更新测试 (5x5=25 个并发请求) ==="
mkdir -p "$ROBUST_DIR/concurrent_results"

# 启动并发进程
for i in $(seq 1 5); do
    (
        for j in $(seq 1 5); do
            RESP=$(curl -s --max-time 10 -X POST "$API_URL/v1/discovery:vhosts" \
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
            echo "$RESP" >> "$ROBUST_DIR/concurrent_results/worker-$i.log"
        done
    ) &
done

echo "等待所有并发进程..."
wait

echo ""
echo "=== 检查并发结果 ==="
for f in "$ROBUST_DIR/concurrent_results"/*.log; do
    if [ -f "$f" ]; then
        echo "--- $(basename $f) ---"
        cat "$f"
    fi
done

echo ""
echo "=== 统计 ==="
TOTAL=$(cat "$ROBUST_DIR/concurrent_results"/*.log 2>/dev/null | wc -l)
ACK_COUNT=$(grep -c '"ACK"' "$ROBUST_DIR/concurrent_results"/*.log 2>/dev/null || echo "0")
NACK_COUNT=$(grep -c '"NACK"' "$ROBUST_DIR/concurrent_results"/*.log 2>/dev/null || echo "0")
echo "总请求: $TOTAL, ACK: $ACK_COUNT, NACK: $NACK_COUNT"

echo ""
echo "=== 调试完成 ==="
