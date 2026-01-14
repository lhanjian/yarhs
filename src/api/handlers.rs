// xDS Discovery 处理器模块

use std::convert::Infallible;
use std::sync::Arc;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Request, Response, StatusCode};
use serde::Deserialize;

use crate::config::{AppState, ResourceType};
use crate::logger;
use super::types::{
    DiscoveryResponse, Resource, SnapshotResponse, ResourceSnapshot,
    VersionedValue, ListenerResource, ServerEndpoint, RouteResource,
};
use super::response::{json_response, bad_request, conflict_response};
use super::updaters;

/// 获取所有资源快照
pub async fn handle_snapshot(state: Arc<AppState>) -> Result<Response<Full<Bytes>>, Infallible> {
    let dynamic_config = state.dynamic_config.read().await;
    
    let (listener_ver, listener_nonce) = state.xds_versions.get_version(ResourceType::Listener);
    let (route_ver, route_nonce) = state.xds_versions.get_version(ResourceType::Route);
    let (http_ver, http_nonce) = state.xds_versions.get_version(ResourceType::Http);
    let (logging_ver, logging_nonce) = state.xds_versions.get_version(ResourceType::Logging);
    let (perf_ver, perf_nonce) = state.xds_versions.get_version(ResourceType::Performance);
    
    let snapshot = SnapshotResponse {
        version_info: format!("{}", std::cmp::max(
            std::cmp::max(listener_ver, route_ver),
            std::cmp::max(std::cmp::max(http_ver, logging_ver), perf_ver)
        )),
        resources: ResourceSnapshot {
            listener: VersionedValue {
                version_info: listener_ver.to_string(),
                nonce: listener_nonce.to_string(),
                value: ListenerResource {
                    main_server: ServerEndpoint {
                        host: dynamic_config.server.host.clone(),
                        port: dynamic_config.server.port,
                    },
                    api_server: ServerEndpoint {
                        host: dynamic_config.server.api_host.clone(),
                        port: dynamic_config.server.api_port,
                    },
                },
            },
            route: VersionedValue {
                version_info: route_ver.to_string(),
                nonce: route_nonce.to_string(),
                value: RouteResource {
                    favicon_paths: dynamic_config.routes.favicon_paths.clone(),
                    index_files: dynamic_config.routes.index_files.clone(),
                    custom_routes: dynamic_config.routes.custom_routes.clone(),
                },
            },
            http: VersionedValue {
                version_info: http_ver.to_string(),
                nonce: http_nonce.to_string(),
                value: (*dynamic_config.http).clone(),
            },
            logging: VersionedValue {
                version_info: logging_ver.to_string(),
                nonce: logging_nonce.to_string(),
                value: dynamic_config.logging.clone(),
            },
            performance: VersionedValue {
                version_info: perf_ver.to_string(),
                nonce: perf_nonce.to_string(),
                value: dynamic_config.performance.clone(),
            },
        },
    };
    
    logger::log_api_request("GET", "/v1/discovery", 200);
    json_response(StatusCode::OK, &snapshot)
}

/// GET 方式获取资源（简单查询）
pub async fn handle_discovery_get(
    state: Arc<AppState>,
    resource_type: ResourceType,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let dynamic_config = state.dynamic_config.read().await;
    let (version, nonce) = state.xds_versions.get_version(resource_type);
    
    let type_url = format!("type.yarhs.io/{resource_type}");
    let path = format!("/v1/discovery:{}", resource_type.to_string().to_lowercase() + "s");
    
    let resources = match resource_type {
        ResourceType::Listener => {
            vec![Resource {
                type_url: type_url.clone(),
                name: "main".to_string(),
                value: serde_json::json!({
                    "main_server": {
                        "host": dynamic_config.server.host,
                        "port": dynamic_config.server.port
                    },
                    "api_server": {
                        "host": dynamic_config.server.api_host,
                        "port": dynamic_config.server.api_port
                    }
                }),
            }]
        }
        ResourceType::Route => {
            vec![Resource {
                type_url: type_url.clone(),
                name: "default".to_string(),
                value: serde_json::json!({
                    "favicon_paths": &dynamic_config.routes.favicon_paths,
                    "index_files": &dynamic_config.routes.index_files,
                    "custom_routes": &dynamic_config.routes.custom_routes
                }),
            }]
        }
        ResourceType::Http => {
            vec![Resource {
                type_url: type_url.clone(),
                name: "default".to_string(),
                value: serde_json::to_value(&*dynamic_config.http).unwrap_or_default(),
            }]
        }
        ResourceType::Logging => {
            vec![Resource {
                type_url: type_url.clone(),
                name: "default".to_string(),
                value: serde_json::to_value(&dynamic_config.logging).unwrap_or_default(),
            }]
        }
        ResourceType::Performance => {
            vec![Resource {
                type_url: type_url.clone(),
                name: "default".to_string(),
                value: serde_json::to_value(&dynamic_config.performance).unwrap_or_default(),
            }]
        }
    };
    
    let response = DiscoveryResponse {
        version_info: version.to_string(),
        resources,
        nonce: nonce.to_string(),
        type_url,
    };
    
    logger::log_api_request("GET", &path, 200);
    json_response(StatusCode::OK, &response)
}

/// POST 方式更新资源（xDS 标准方式）
pub async fn handle_discovery_post(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
    resource_type: ResourceType,
) -> Result<Response<Full<Bytes>>, Infallible> {
    use http_body_util::BodyExt;
    
    /// xDS 更新请求结构
    #[derive(Deserialize)]
    struct UpdateRequest {
        /// 客户端回传的版本号（用于乐观锁）
        #[serde(default)]
        version_info: String,
        /// 要更新的资源
        resources: Vec<serde_json::Value>,
        /// 是否强制重启（仅对 Listener 有效）
        #[serde(default)]
        force_restart: bool,
    }
    
    let path = format!("/v1/discovery:{}", resource_type.to_string().to_lowercase() + "s");
    
    // Read request body
    let whole_body = if let Ok(collected) = req.collect().await {
        collected.to_bytes()
    } else {
        logger::log_api_request("POST", &path, 400);
        return Ok(bad_request("Failed to read request body"));
    };
    
    let update_req: UpdateRequest = match serde_json::from_slice(&whole_body) {
        Ok(r) => r,
        Err(e) => {
            logger::log_api_request("POST", &path, 400);
            return Ok(bad_request(&format!("Invalid JSON: {e}")));
        }
    };
    
    // 检查版本冲突（乐观锁）
    if !update_req.version_info.is_empty() {
        let (current_version, _) = state.xds_versions.get_version(resource_type);
        if update_req.version_info != current_version.to_string() {
            logger::log_api_request("POST", &path, 409);
            return Ok(conflict_response(&format!(
                "Version conflict: expected {current_version}, got {version}",
                current_version = current_version, version = update_req.version_info
            )));
        }
    }
    
    if update_req.resources.is_empty() {
        logger::log_api_request("POST", &path, 400);
        return Ok(bad_request("No resources provided"));
    }
    
    // 处理更新
    let result = match resource_type {
        ResourceType::Listener => {
            updaters::update_listener(&state, &update_req.resources[0], update_req.force_restart).await
        }
        ResourceType::Route => {
            updaters::update_route(&state, &update_req.resources[0]).await
        }
        ResourceType::Http => {
            updaters::update_http(&state, &update_req.resources[0]).await
        }
        ResourceType::Logging => {
            updaters::update_logging(&state, &update_req.resources[0]).await
        }
        ResourceType::Performance => {
            updaters::update_performance(&state, &update_req.resources[0]).await
        }
    };
    
    match result {
        Ok(message) => {
            // 更新版本号
            let (new_version, new_nonce) = state.xds_versions.increment(resource_type);
            
            logger::log_api_request("POST", &path, 200);
            
            let response = serde_json::json!({
                "status": "ACK",
                "version_info": new_version.to_string(),
                "nonce": new_nonce.to_string(),
                "message": message
            });
            
            json_response(StatusCode::OK, &response)
        }
        Err(e) => {
            logger::log_api_request("POST", &path, 400);
            
            let response = serde_json::json!({
                "status": "NACK",
                "error_detail": {
                    "code": 400,
                    "message": e
                }
            });
            
            json_response(StatusCode::BAD_REQUEST, &response)
        }
    }
}
