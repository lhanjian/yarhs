# 动态路由配置文档

## 概述

本服务器支持动态路由配置，可以通过 API 在运行时修改路由规则，无需重启服务器（除非修改了 server 配置）。

## 路由配置结构

### 基础路由配置

```toml
[routes]
api_prefix = "/api"                          # API 路由前缀
static_prefix = "/static"                    # 静态文件路由前缀
favicon_paths = ["/favicon.ico", "/favicon.svg"]  # Favicon 路径列表
```

### 自定义路由

在 `config.toml` 中定义自定义路由：

```toml
[routes.custom_routes]
"/docs" = { type = "markdown", file = "docs/README.md" }
"/about" = { type = "template", file = "templates/about.html" }
"/old-path" = { type = "redirect", target = "/new-path" }
"/files" = { type = "static", dir = "uploads" }
```

## 路由类型

### 1. Static（静态文件目录）

将特定路径映射到一个静态文件目录：

```json
{
  "type": "static",
  "dir": "uploads"
}
```

**示例**：
- 配置：`"/downloads" = { type = "static", dir = "public/downloads" }`
- 访问：`/downloads/file.txt` → 读取 `public/downloads/file.txt`

### 2. Template（HTML 模板）

直接渲染 HTML 模板文件：

```json
{
  "type": "template",
  "file": "templates/about.html"
}
```

**示例**：
- 配置：`"/about" = { type = "template", file = "templates/about.html" }`
- 访问：`/about` → 返回 `templates/about.html` 的内容

### 3. Markdown（Markdown 渲染）

将 Markdown 文件渲染为 HTML：

```json
{
  "type": "markdown",
  "file": "docs/guide.md"
}
```

**示例**：
- 配置：`"/guide" = { type = "markdown", file = "docs/guide.md" }`
- 访问：`/guide` → 读取并渲染 `docs/guide.md` 为 HTML

### 4. Redirect（重定向）

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

1. **API 路由** - 以 `api_prefix` 开头的路径（如 `/api/*`）
2. **Favicon 路由** - `favicon_paths` 列表中的精确路径
3. **自定义路由** - `custom_routes` 中定义的精确匹配路径
4. **静态文件** - 以 `static_prefix` 开头的路径（如 `/static/*`）
5. **默认路由** - Markdown 主页（`API.md`）

## 通过 API 修改路由

### 查看当前路由配置

```bash
curl http://localhost:8080/api/config | jq .routes
```

### 更新路由配置

**完整配置示例**：

```bash
curl -X PUT http://localhost:8080/api/config \
  -H "Content-Type: application/json" \
  -d '{
    "logging": {
      "level": "info",
      "access_log": true,
      "show_headers": false
    },
    "http": {
      "default_content_type": "text/html; charset=utf-8",
      "server_name": "Tokio-Hyper/1.0",
      "enable_cors": true
    },
    "resources": {
      "template_dir": "templates"
    },
    "routes": {
      "api_prefix": "/api",
      "static_prefix": "/static",
      "favicon_paths": ["/favicon.ico", "/favicon.svg"],
      "custom_routes": {
        "/docs": {
          "type": "markdown",
          "file": "API.md"
        },
        "/readme": {
          "type": "markdown",
          "file": "README.md"
        },
        "/admin": {
          "type": "template",
          "file": "templates/admin.html"
        },
        "/home": {
          "type": "redirect",
          "target": "/"
        }
      }
    }
  }'
```

### 动态添加路由示例

**添加文档路由**：

```bash
# 创建文档文件
mkdir -p docs
echo "# User Guide\n\nWelcome!" > docs/user-guide.md

# 更新路由配置（需要包含完整配置）
curl -X PUT http://localhost:8080/api/config \
  -H "Content-Type: application/json" \
  -d '{
    "logging": {"level": "info", "access_log": true, "show_headers": false},
    "http": {"default_content_type": "text/html; charset=utf-8", "server_name": "Tokio-Hyper/1.0", "enable_cors": true},
    "resources": {"template_dir": "templates"},
    "routes": {
      "api_prefix": "/api",
      "static_prefix": "/static",
      "favicon_paths": ["/favicon.ico", "/favicon.svg"],
      "custom_routes": {
        "/user-guide": {"type": "markdown", "file": "docs/user-guide.md"}
      }
    }
  }'

# 访问新路由
curl http://localhost:8080/user-guide
```

**添加重定向**：

```bash
curl -X PUT http://localhost:8080/api/config \
  -H "Content-Type: application/json" \
  -d '{
    ...
    "routes": {
      ...
      "custom_routes": {
        "/github": {"type": "redirect", "target": "https://github.com"}
      }
    }
  }'
```

## 配置示例

### 文档站点配置

```toml
[routes]
api_prefix = "/api"
static_prefix = "/assets"
favicon_paths = ["/favicon.ico"]

[routes.custom_routes]
"/guide" = { type = "markdown", file = "docs/guide.md" }
"/tutorial" = { type = "markdown", file = "docs/tutorial.md" }
"/changelog" = { type = "markdown", file = "CHANGELOG.md" }
```

### 多语言网站配置

```toml
[routes.custom_routes]
"/zh" = { type = "template", file = "templates/index-zh.html" }
"/en" = { type = "template", file = "templates/index-en.html" }
"/ja" = { type = "template", file = "templates/index-ja.html" }
```

### 文件下载站点配置

```toml
[routes.custom_routes]
"/downloads" = { type = "static", dir = "public/downloads" }
"/images" = { type = "static", dir = "public/images" }
"/videos" = { type = "static", dir = "public/videos" }
```

## 注意事项

1. **路径必须以 `/` 开头**
2. **自定义路由是精确匹配**，不支持通配符
3. **文件路径相对于服务器工作目录**
4. **修改路由配置立即生效**，无需重启
5. **安全性**：静态文件路径会进行目录遍历检查，防止访问上级目录
6. **更新配置时需要提供完整的配置对象**，部分更新暂不支持

## 测试路由配置

创建测试文件并配置路由：

```bash
# 1. 创建测试文件
mkdir -p docs
echo "# Test Document" > docs/test.md

# 2. 获取当前配置
curl -s http://localhost:8080/api/config > current_config.json

# 3. 编辑配置添加路由
# 在 routes.custom_routes 中添加：
# "/test": {"type": "markdown", "file": "docs/test.md"}

# 4. 更新配置
curl -X PUT http://localhost:8080/api/config \
  -H "Content-Type: application/json" \
  -d @current_config.json

# 5. 测试新路由
curl http://localhost:8080/test
```

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
- Markdown 渲染结果会被缓存（主页）
- 静态文件使用异步 I/O，不阻塞服务器
