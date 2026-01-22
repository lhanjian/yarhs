#!/bin/bash
# Range 请求测试 (断点续传)
# 此脚本由 integration_tests.sh 调用，common.sh 已加载

log_section "5. Range 请求 (断点续传)"

# Accept-Ranges 头
ACCEPT_RANGES=$(curl -sI "$BASE_URL/static/test.txt" | grep -i "accept-ranges:" | tr -d '\r')
assert_contains "Accept-Ranges 头" "$ACCEPT_RANGES" "bytes"

# 固定范围请求 (前10字节)
RANGE_STATUS=$(curl -sI -H "Range: bytes=0-9" "$BASE_URL/static/test.txt" | grep "HTTP" | cut -d' ' -f2 | tr -d '\r')
CONTENT_RANGE=$(curl -sI -H "Range: bytes=0-9" "$BASE_URL/static/test.txt" | grep -i "content-range:" | tr -d '\r')
if [ "$RANGE_STATUS" = "206" ]; then
    log_pass "Range 请求返回 206 Partial Content"
else
    log_fail "Range 请求返回 206 (got: $RANGE_STATUS)"
fi
assert_contains "Content-Range 头" "$CONTENT_RANGE" "bytes 0-9/"

# 开放范围请求 (从第5字节到末尾)
CONTENT_RANGE=$(curl -sI -H "Range: bytes=5-" "$BASE_URL/static/test.txt" | grep -i "content-range:" | tr -d '\r')
assert_contains "开放范围请求 (bytes=5-)" "$CONTENT_RANGE" "bytes 5-"

# 后缀范围请求 (最后5字节)
CONTENT_RANGE=$(curl -sI -H "Range: bytes=-5" "$BASE_URL/static/test.txt" | grep -i "content-range:" | tr -d '\r')
assert_contains "后缀范围请求 (bytes=-5)" "$CONTENT_RANGE" "bytes"

# 验证返回内容长度
RANGE_LENGTH=$(curl -sI -H "Range: bytes=0-9" "$BASE_URL/static/test.txt" | grep -i "content-length:" | cut -d' ' -f2 | tr -d '\r')
if [ "$RANGE_LENGTH" = "10" ]; then
    log_pass "Range Content-Length 正确 (10)"
else
    log_fail "Range Content-Length 错误 (expected: 10, got: $RANGE_LENGTH)"
fi

# 416 Range Not Satisfiable (超出文件大小)
assert_status "无效范围返回 416 (bytes=99999-)" "$BASE_URL/static/test.txt" "416" "Range: bytes=99999-"

# 416 Content-Range 头格式
CONTENT_RANGE_416=$(curl -sI -H "Range: bytes=99999-" "$BASE_URL/static/test.txt" | grep -i "content-range:" | tr -d '\r')
assert_contains "416 Content-Range 格式 (bytes */size)" "$CONTENT_RANGE_416" "bytes \*/"
