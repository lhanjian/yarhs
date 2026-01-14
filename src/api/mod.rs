// API 模块入口
// xDS 风格的配置管理 API

mod handlers;
mod response;
mod types;
mod updaters;

use std::convert::Infallible;
use std::sync::Arc;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Request, Response, Method};

use crate::config::{AppState, ResourceType};
use crate::logger;

// 重新导出公共类型
pub use response::*;

/// API 路由处理器
/// 
/// 根据请求路径和方法分发到对应的处理函数
pub async fn handle_api_config(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = req.uri().path();
    let method = req.method().clone();
    
    // xDS style routes
    match (method, path) {
        // 获取所有资源快照
        (Method::GET, "/v1/discovery") => {
            handlers::handle_snapshot(state).await
        }
        // 发现特定类型资源 (Listener)
        (Method::GET, "/v1/discovery:listeners") => {
            handlers::handle_discovery_get(state, ResourceType::Listener).await
        }
        (Method::POST, "/v1/discovery:listeners") => {
            handlers::handle_discovery_post(req, state, ResourceType::Listener).await
        }
        // 发现路由资源
        (Method::GET, "/v1/discovery:routes") => {
            handlers::handle_discovery_get(state, ResourceType::Route).await
        }
        (Method::POST, "/v1/discovery:routes") => {
            handlers::handle_discovery_post(req, state, ResourceType::Route).await
        }
        // 发现 HTTP 配置
        (Method::GET, "/v1/discovery:http") => {
            handlers::handle_discovery_get(state, ResourceType::Http).await
        }
        (Method::POST, "/v1/discovery:http") => {
            handlers::handle_discovery_post(req, state, ResourceType::Http).await
        }
        // 发现日志配置
        (Method::GET, "/v1/discovery:logging") => {
            handlers::handle_discovery_get(state, ResourceType::Logging).await
        }
        (Method::POST, "/v1/discovery:logging") => {
            handlers::handle_discovery_post(req, state, ResourceType::Logging).await
        }
        // 发现性能配置
        (Method::GET, "/v1/discovery:performance") => {
            handlers::handle_discovery_get(state, ResourceType::Performance).await
        }
        (Method::POST, "/v1/discovery:performance") => {
            handlers::handle_discovery_post(req, state, ResourceType::Performance).await
        }
        // 未知路由
        _ => {
            logger::log_api_request(req.method().as_str(), path, 404);
            Ok(not_found())
        }
    }
}
