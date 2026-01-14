# 动态路由配置文档

## 概述

本服务器支持动态路由配置，可以通过 API 在运行时修改路由规则，无需重启服务器（除非修改了 server 配置）。

## 路由配置结构

### 基础路由配置

```toml
[routes]
favicon_paths = ["/favicon.ico", "/favicon.svg"]  # Favicon 路径列表
index_files = ["index.html", "index.htm"]         # 默认文档列表
```

### 默认文档

当访问目录路径时（如 `/static/` 或 `/`），服务器会自动查找 `index_files` 中配置的文件：

```toml
[routes]
index_files = ["index.html", "index.htm"]
```

**行为示例**：
- 访问 `/static/` → 返回 `static/index.html`（如果存在）
- 访问 `/` （配置 `"/" = { type = "dir", path = "public" }`）→ 返回 `public/index.html`

**查找顺序**：按配置列表顺序，找到第一个存在的文件即返回。

### 自定义路由

在 `config.toml` 中定义自定义路由：

```toml
[routes.custom_routes]
"/about" = { type = "file", path = "templates/about.html" }
"/old-path" = { type = "redirect", target = "/new-path" }
"/assets" = { type = "dir", path = "public/assets" }
```

## 路由类型

### 1. File（单文件路由）

返回指定的单个文件，自动根据扩展名识别 MIME 类型：

```json
{
  "type": "file",
  "path": "templates/about.html"
}
```

**支持的文件类型**：
- HTML、CSS、JS、JSON
- PNG、JPG、GIF、SVG、WebP、ICO
- PDF、XML、TXT、MD
- 字体文件（WOFF、WOFF2、TTF）
- 音视频（MP4、WebM、MP3、WAV）
- 其他文件返回 `application/octet-stream`

**示例**：
- 配置：`"/about" = { type = "file", path = "templates/about.html" }`
- 访问：`/about` → 返回 `templates/about.html`

### 2. Dir（目录路由）

将 URL 前缀映射到文件目录，支持前缀匹配和默认文档：

```json
{
  "type": "dir",
  "path": "static"
}
```

**特性**：
- 支持前缀匹配（`/static/css/style.css` → `static/css/style.css`）
- 支持默认文档（`/static/` → `static/index.html`）
- 支持根路径映射（`"/" = { type = "dir", path = "public" }`）

**示例**：
- 配置：`"/static" = { type = "dir", path = "public/static" }`
- 访问：`/static/css/style.css` → 读取 `public/static/css/style.css`
- 访问：`/static/` → 读取 `public/static/index.html`（默认文档）

### 3. Redirect（重定向路由）

将请求重定向到另一个 URL：

```json
{
  "type": "redirect",
  "target": "/new-location"
}
```

**示例**：
- 配置：`"/old" = { type = "redirect", target = "/new" }`
- 访问：`/old` → 302 重定向到 `/new`

## 路由优先级

路由匹配按以下优先级顺序：

1. **Favicon 路由** - `favicon_paths` 列表中的精确路径
2. **自定义路由（精确匹配）** - `custom_routes` 中的 file/redirect 类型
3. **自定义路由（前缀匹配）** - `custom_routes` 中的 dir 类型
4. **默认路由** - 返回默认主页

## ETag 与条件请求

### ETag 机制

服务器为所有静态文件自动生成 ETag（基于文件内容的快速哈希），用于客户端缓存验证：

**响应头示例**：
```
HTTP/1.1 200 OK
ETag: "23cc8d56a93cc61c"
Cache-Control: public, max-age=3600
```

### 条件请求（If-None-Match）

客户端可以发送 `If-None-Match` 头来验证缓存：

**请求**：
```bash
curl -H 'If-None-Match: "23cc8d56a93cc61c"' http://localhost:8080/static/test.txt
```

**ETag 匹配时**（304 Not Modified）：
```
HTTP/1.1 304 Not Modified
ETag: "23cc8d56a93cc61c"
Cache-Control: public, max-age=3600
```

**ETag 不匹配时**（200 OK）：
```
HTTP/1.1 200 OK
ETag: "新的ETag值"
Cache-Control: public, max-age=3600
Content-Length: 23
```

### 节省带宽

使用 304 响应可以显著减少带宽消耗：
- 不传输响应体
- 仅返回必要的头信息
- 浏览器使用本地缓存内容

## 通过 API 修改路由

### 查看当前路由配置

```bash
curl http://localhost:8000/v1/discovery:routes | jq '.resources[0]'
```

### 更新路由配置

通过 xDS 风格端点更新路由：

```bash
# POST 路由资源（xDS 格式）
curl -X POST http://localhost:8000/v1/discovery:routes \
  -H "Content-Type: application/json" \
  -d '{"resources": [{
    "favicon_paths": ["/favicon.ico", "/favicon.svg"],
    "index_files": ["index.html"],
    "custom_routes": {
      "/about": {"type": "file", "path": "templates/about.html"},
      "/data": {"type": "file", "path": "static/data.json"},
      "/static": {"type": "dir", "path": "static"}
    }
}]}'
```

## 配置示例

### 多语言网站配置

```toml
[routes.custom_routes]
"/zh" = { type = "file", path = "templates/index-zh.html" }
"/en" = { type = "file", path = "templates/index-en.html" }
"/ja" = { type = "file", path = "templates/index-ja.html" }
```

### 文件下载站点配置

```toml
[routes.custom_routes]
"/downloads" = { type = "dir", path = "public/downloads" }
"/images" = { type = "dir", path = "public/images" }
"/videos" = { type = "dir", path = "public/videos" }
```

### URL 重定向配置

```toml
[routes.custom_routes]
"/old-api" = { type = "redirect", target = "/api/v2" }
"/legacy" = { type = "redirect", target = "/new" }
```

### 混合配置

```toml
[routes.custom_routes]
"/about" = { type = "file", path = "pages/about.html" }
"/api-spec" = { type = "file", path = "docs/openapi.json" }
"/assets" = { type = "dir", path = "public/assets" }
"/old-docs" = { type = "redirect", target = "/docs" }
```

## 注意事项

1. **路径必须以 `/` 开头**
2. **file/redirect 是精确匹配**，dir 是前缀匹配
3. **path 字段相对于服务器工作目录**
4. **修改路由配置立即生效**，无需重启
5. **安全性**：目录路由会进行目录遍历检查，防止访问上级目录
6. **更新配置时需要提供完整的配置对象**，部分更新暂不支持

## API 响应格式

### 成功响应

```json
{
  "status": "ok",
  "message": "Configuration updated"
}
```

### 错误响应

```json
{
  "error": "Invalid JSON: missing field `routes` at line 10 column 3"
}
```

## 性能考虑

- 路由配置使用 `HashMap` 存储，查找时间复杂度 O(1)
- 配置更新使用读写锁 `RwLock`，读操作可并发
- 静态文件使用异步 I/O，不阻塞服务器
