// xDS Discovery handlers module

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Request, Response, StatusCode};
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;

use super::response::{bad_request, conflict_response, json_response};
use super::types::{
    DiscoveryResponse, ListenerResource, Resource, ResourceSnapshot, RouteResource, ServerEndpoint,
    SnapshotResponse, VersionedValue,
};
use super::updaters;
use crate::config::{AppState, ResourceType};
use crate::logger;

/// Get all resources snapshot
pub async fn handle_snapshot(state: Arc<AppState>) -> Result<Response<Full<Bytes>>, Infallible> {
    let dynamic_config = state.dynamic_config.read().await;

    let (listener_ver, listener_nonce) = state.xds_versions.get_version(ResourceType::Listener);
    let (route_ver, route_nonce) = state.xds_versions.get_version(ResourceType::Route);
    let (http_ver, http_nonce) = state.xds_versions.get_version(ResourceType::Http);
    let (logging_ver, logging_nonce) = state.xds_versions.get_version(ResourceType::Logging);
    let (perf_ver, perf_nonce) = state.xds_versions.get_version(ResourceType::Performance);

    let snapshot = SnapshotResponse {
        version_info: format!(
            "{}",
            std::cmp::max(
                std::cmp::max(listener_ver, route_ver),
                std::cmp::max(std::cmp::max(http_ver, logging_ver), perf_ver)
            )
        ),
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

/// GET method to fetch resources (simple query)
pub async fn handle_discovery_get(
    state: Arc<AppState>,
    resource_type: ResourceType,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let dynamic_config = state.dynamic_config.read().await;
    let (version, nonce) = state.xds_versions.get_version(resource_type);

    let type_url = format!("type.yarhs.io/{resource_type}");
    let path = format!(
        "/v1/discovery:{}",
        resource_type.to_string().to_lowercase() + "s"
    );

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
                value: serde_json::to_value(&*dynamic_config.http).unwrap_or_else(|e| {
                    logger::log_error(&format!("Failed to serialize HTTP config: {e}"));
                    serde_json::json!({"error": "serialization_failed"})
                }),
            }]
        }
        ResourceType::Logging => {
            vec![Resource {
                type_url: type_url.clone(),
                name: "default".to_string(),
                value: serde_json::to_value(&dynamic_config.logging).unwrap_or_else(|e| {
                    logger::log_error(&format!("Failed to serialize logging config: {e}"));
                    serde_json::json!({"error": "serialization_failed"})
                }),
            }]
        }
        ResourceType::Performance => {
            vec![Resource {
                type_url: type_url.clone(),
                name: "default".to_string(),
                value: serde_json::to_value(&dynamic_config.performance).unwrap_or_else(|e| {
                    logger::log_error(&format!("Failed to serialize performance config: {e}"));
                    serde_json::json!({"error": "serialization_failed"})
                }),
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

/// POST method to update resources (xDS standard)
pub async fn handle_discovery_post(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
    resource_type: ResourceType,
) -> Result<Response<Full<Bytes>>, Infallible> {
    use http_body_util::BodyExt;

    /// xDS update request structure
    #[derive(Deserialize)]
    struct UpdateRequest {
        /// Version returned by client (for optimistic locking)
        #[serde(default)]
        version_info: String,
        /// Resources to update
        resources: Vec<serde_json::Value>,
        /// Whether to force restart (only valid for Listener)
        #[serde(default)]
        force_restart: bool,
    }

    let path = format!(
        "/v1/discovery:{}",
        resource_type.to_string().to_lowercase() + "s"
    );

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

    // Check version conflict (optimistic locking)
    if !update_req.version_info.is_empty() {
        let (current_version, _) = state.xds_versions.get_version(resource_type);
        if update_req.version_info != current_version.to_string() {
            logger::log_api_request("POST", &path, 409);
            return Ok(conflict_response(&format!(
                "Version conflict: expected {current_version}, got {version}",
                current_version = current_version,
                version = update_req.version_info
            )));
        }
    }

    if update_req.resources.is_empty() {
        logger::log_api_request("POST", &path, 400);
        return Ok(bad_request("No resources provided"));
    }

    // Process update
    let result = match resource_type {
        ResourceType::Listener => {
            updaters::update_listener(&state, &update_req.resources[0], update_req.force_restart)
                .await
        }
        ResourceType::Route => updaters::update_route(&state, &update_req.resources[0]).await,
        ResourceType::Http => updaters::update_http(&state, &update_req.resources[0]).await,
        ResourceType::Logging => updaters::update_logging(&state, &update_req.resources[0]).await,
        ResourceType::Performance => {
            updaters::update_performance(&state, &update_req.resources[0]).await
        }
    };

    match result {
        Ok(message) => {
            // Update version number
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
