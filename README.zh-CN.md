# 🚀 Rust 高性能异步 Webserver

[English](README.md)

一个功能完整的生产级 Rust HTTP 服务器，具备**动态路由配置**、**xDS 风格 API**、零停机热重启、性能优化等企业级特性。

## ✨ 核心特性

### 1. xDS 风格配置 API 🆕
- ✅ **资源发现端点** - 类 Envoy xDS 协议，按资源类型独立管理
- ✅ **版本控制** - 乐观锁防止并发冲突
- ✅ **增量更新** - 只更新需要的资源，不影响其他配置

```bash
# xDS 风格 API (推荐)
curl http://localhost:8000/v1/discovery           # 获取所有资源快照
curl http://localhost:8000/v1/discovery:routes    # 获取路由配置
curl http://localhost:8000/v1/discovery:logging   # 获取日志配置

# 更新单个资源
curl -X POST http://localhost:8000/v1/discovery:logging \
  -H "Content-Type: application/json" \
  -d '{"resources": [{"level": "debug", "access_log": true, "show_headers": true}]}'
```

### 2. 动态路由配置
- ✅ **运行时修改路由** - 通过 API 动态添加/修改路由，无需重启
- ✅ **多种路由类型** - 支持单文件、静态目录、重定向
- ✅ **默认文档** - 访问目录自动返回 index.html
- ✅ **ETag + 304** - 条件请求支持，节省带宽
- ✅ **精确匹配** - 自定义路由精确匹配，静态文件支持前缀匹配
- ✅ **优先级控制** - Favicon > 自定义 > 静态文件 > 默认主页
- 使用 `SO_REUSEPORT` 实现同端口并发监听
- 双循环并发模型：新监听器立即启动，旧监听器优雅排空
- 支持同端口重启和跨端口迁移

### 3. 智能缓存系统
- 配置热更新缓存
- 原子操作避免锁竞争
- **ETag 支持** - 基于内容哈希的 ETag 生成
- **条件请求** - If-None-Match 匹配时返回 304 Not Modified

### 4. 高性能
- **40k+ QPS** (静态文件)
- **63k+ QPS** (API 接口)
- 全异步 I/O，基于 Tokio + Hyper

## 📦 项目结构

```
yarhs/
├── src/
│   ├── main.rs           - 入口点
│   ├── handler.rs        - 动态路由处理
│   ├── response.rs       - 响应构建、ETag、缓存
│   ├── logger.rs         - 日志工具
│   ├── api/              - xDS 风格配置 API 模块
│   │   ├── mod.rs        - 模块导出与路由
│   │   ├── types.rs      - xDS 类型定义
│   │   ├── handlers.rs   - GET/POST 请求处理
│   │   ├── updaters.rs   - 资源更新函数
│   │   └── response.rs   - API 响应构建
│   ├── config/           - 配置管理模块
│   │   ├── mod.rs        - 配置加载
│   │   ├── types.rs      - 配置类型定义
│   │   ├── state.rs      - AppState 共享状态
│   │   └── version.rs    - xDS 版本管理
│   └── server/           - 服务器核心模块
│       ├── mod.rs        - 模块导出
│       ├── listener.rs   - TCP 监听器 (SO_REUSEPORT)
│       ├── connection.rs - 连接处理
│       ├── loop.rs       - 服务器主循环
│       └── restart.rs    - 热重启逻辑
├── scripts/              - 测试脚本
│   ├── run_all_tests.sh  - 统一测试脚本
│   └── integration_tests.sh - 集成测试
├── static/               - 静态资源
├── templates/            - HTML 模板
├── config.toml           - 服务器配置
├── API.md                - xDS API 文档
├── CONFIG.md             - 配置文档
└── ROUTES.md             - 路由配置文档
```

## 🎯 路由配置

### 配置示例

**config.toml:**
```toml
[routes]
favicon_paths = ["/favicon.ico", "/favicon.svg"]

[routes.custom_routes]
"/about" = { type = "file", path = "templates/about.html" }
"/static" = { type = "dir", path = "static" }
"/old-url" = { type = "redirect", target = "/new-url" }
```

### 路由类型

| 类型 | 说明 | 示例 |
|------|------|------|
| `file` | 返回单个文件（自动识别 MIME） | `{"type": "file", "path": "templates/about.html"}` |
| `dir` | 目录映射（前缀匹配） | `{"type": "dir", "path": "static"}` |
| `redirect` | HTTP 302 重定向 | `{"type": "redirect", "target": "/new"}` |

### API 操作

```bash
# === xDS 风格 API (推荐) ===

# 查看所有配置快照
curl http://localhost:8000/v1/discovery

# 查看特定资源
curl http://localhost:8000/v1/discovery:routes
curl http://localhost:8000/v1/discovery:logging

# 更新路由配置
curl -X POST http://localhost:8000/v1/discovery:routes \
  -H "Content-Type: application/json" \
  -d '{
    "resources": [{
      "favicon_paths": ["/favicon.ico"],
      "index_files": ["index.html"],
      "custom_routes": {
        "/about": {"type": "file", "path": "templates/about.html"}
      }
    }]
  }'

# 查看完整 API 文档
curl http://localhost:8080/  # 默认显示 API.md
```

## 🚀 快速开始

### 编译运行

```bash
# 开发模式
cargo run

# 生产构建
cargo build --release
./target/release/rust_webserver
```

### 测试路由功能

```bash
# 运行所有测试（单元测试 + 集成测试）
./scripts/run_all_tests.sh

# 仅运行集成测试
./scripts/integration_tests.sh
```

测试脚本会：
1. 启动服务器
2. 创建测试文件（HTML 模板）
3. 动态配置路由
4. 测试所有路由类型
5. 性能对比测试
6. 自动清理

### 性能测试

```bash
# 使用 wrk 测试
wrk -t4 -c100 -d30s http://127.0.0.1:8080/

# 使用 ApacheBench 测试
ab -n 10000 -c 100 http://127.0.0.1:8080/
```

## 📚 技术栈

- **Tokio 1.41** - 异步运行时
- **Hyper 1.5** - HTTP 服务器
- **socket2 0.6** - Socket 底层控制（SO_REUSEPORT）
- **serde + serde_json** - JSON 序列化
- **config 0.14** - TOML 配置管理

## 🔧 配置项

完整配置见 [CONFIG.md](CONFIG.md)

主要配置：
- `server` - 服务器地址和端口
- `logging` - 日志级别、访问日志
- `http` - HTTP 响应头、CORS
- `performance` - 超时、连接限制
- `routes` - 路由配置（动态） 🆕

## 📖 文档

- [API.md](API.md) - 动态配置 API 文档
- [CONFIG.md](CONFIG.md) - 配置项详细说明
- [ROUTES.md](ROUTES.md) - 路由配置完整指南 🆕

## ⚡ 性能数据

基准测试 (wrk 4 线程 100 连接 30 秒):

| 路由类型 | QPS | 说明 |
|---------|-----|------|
| API 接口 | ~63,000 | 纯 JSON 响应 |
| 静态文件 | ~40,000 | 异步文件读取 |
| File 路由 | ~35,000 | 单文件加载 |

## 🎨 实际应用场景

### 1. 文档站点
```toml
[routes.custom_routes]
"/guide" = { type = "file", path = "docs/guide.html" }
"/api" = { type = "file", path = "docs/api.html" }
"/changelog" = { type = "file", path = "docs/changelog.html" }
```

### 2. 多语言网站
```toml
[routes.custom_routes]
"/zh" = { type = "file", path = "templates/index-zh.html" }
"/en" = { type = "file", path = "templates/index-en.html" }
"/ja" = { type = "file", path = "templates/index-ja.html" }
```

### 3. 文件下载站
```toml
[routes.custom_routes]
"/downloads" = { type = "dir", path = "public/downloads" }
"/images" = { type = "dir", path = "public/images" }
```

### 4. URL 重定向
```toml
[routes.custom_routes]
"/old-api" = { type = "redirect", target = "/api/v2" }
"/docs-v1" = { type = "redirect", target = "/docs/v2" }
```

## 🛡️ 代码质量

- ✅ **零编译警告** - 生产就绪代码
- ✅ **无竞争条件** - 原子操作保证线程安全
- ✅ **完善错误处理** - 所有 I/O 都有错误处理
- ✅ **类型安全** - 充分利用 Rust 类型系统
- ✅ **详尽注释** - 关键逻辑都有注释

## 💡 创新点

1. **动态路由系统** - 业界少见的运行时路由配置
2. **SO_REUSEPORT 零停机** - 先进的热更新方案
3. **多层缓存优化** - 10x+ 性能提升
4. **类型驱动设计** - Enum + Serde 实现灵活路由
5. **生产级架构** - 完整的监控、日志、性能优化

## 📝 更新日志

### v0.2.0 (2026-01-07) 🆕

- ✨ 新增动态路由配置功能
- ✨ 支持 3 种路由类型（File、Dir、Redirect）
- ✨ 通过 API 运行时修改路由
- 📚 新增 ROUTES.md 路由配置文档
- 🧪 新增 test_routes.sh 测试脚本

### v0.2.1 (2026-01-14)

- ✨ 新增默认文档（index_files）支持
- ✨ 新增 ETag + 304 条件请求支持
- 🔧 路由类型重命名（template→file，static→dir）
- 🏗️ 模块化重构：拆分为 `api/`、`config/`、`server/` 目录
- 🧹 启用 Clippy pedantic + nursery 严格检查
- 🧪 新增统一测试脚本 `scripts/run_all_tests.sh`
- 🗑️ 移除旧版 `/api/config` 端点，统一使用 xDS 风格 API

### v0.1.0 (2026-01-06)

- ✨ 基础 HTTP 服务器功能
- ✨ 零停机热重启（SO_REUSEPORT）
- ✨ 动态配置 API
- ⚡ 性能优化（30k+ QPS）
- 📚 完整文档（API.md、CONFIG.md）

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可

MIT License

---

**项目状态**: ✅ 生产就绪 | 🚀 持续优化

**最后更新**: 2026-01-14
