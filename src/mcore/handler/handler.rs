// mcore/handler/handler.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

use chrono::Utc;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::mcore::adapter::json::{Action, ApiRequest, CreateNodeData, api_create_node};
use crate::mcore::api::services::{allow_node, delete_node, list_allowed_nodes};
use crate::mcore::config::load_config::SECRET_MANAGEMENT_TOKEN;
use crate::mcore::errors::enode::NodeError;
use crate::mcore::melisad::services::node::NODE_MANAGER;
use crate::mcore::mlog::LOGGER;

#[derive(serde::Deserialize, Debug)]
pub struct RegisterNodeRequest {
    pub name: String,
    pub url: String,
    pub domain: String,
    pub route_path: String,
    #[serde(default = "default_request_ip")]
    pub ip: String,
    #[serde(default = "default_request_version")]
    pub version: String,
    #[serde(default)]
    pub invite_code: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
pub struct AllowNodeRequest {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    pub domain: String,
    pub route_path: String,
}

#[derive(serde::Serialize)]
pub struct RegisterNodeResponse {
    pub success: bool,
    pub message: String,
    pub pid: Option<u32>,
}

pub async fn handle_management_request(
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let (parts, body) = req.into_parts();

    let mut authenticated = false;
    if let Some(auth_header) = parts.headers.get(hyper::header::AUTHORIZATION)
        && let Ok(auth_str) = auth_header.to_str()
        && auth_str == SECRET_MANAGEMENT_TOKEN
    {
        authenticated = true;
    }

    if !authenticated {
        return Ok(json_response(
            StatusCode::UNAUTHORIZED,
            json!({
                "success": false,
                "message": "Unauthorized: Invalid or missing management token"
            }),
        ));
    }

    let method = parts.method.clone();
    let path = parts.uri.path().to_string();
    let body_bytes = body.collect().await?.to_bytes();

    match (method.as_str(), path.as_str()) {
        ("POST", "/register") => handle_register_node(body_bytes).await,
        ("POST", "/unregister") => handle_unregister_node(body_bytes).await,
        ("GET", "/nodes") => handle_list_nodes().await,
        ("POST", "/nodes/allow") => handle_allow_node(body_bytes).await,
        ("GET", "/nodes/allow") => handle_list_allowed_nodes().await,
        _ => Ok(json_response(
            StatusCode::NOT_FOUND,
            json!({
                "success": false,
                "message": "Endpoint not found"
            }),
        )),
    }
}

async fn handle_register_node(body: Bytes) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let req: RegisterNodeRequest = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(err) => {
            return Ok(json_response(
                StatusCode::BAD_REQUEST,
                json!({
                    "success": false,
                    "message": format!("Invalid JSON: {}", err)
                }),
            ));
        }
    };

    let api_request = ApiRequest {
        version: "1.0".to_string(),
        action: Action::CreateNode,
        request_id: Uuid::new_v4().to_string(),
        timestamp: Utc::now().timestamp() as u64,
        data: CreateNodeData {
            name: req.name,
            url: req.url,
            domain: req.domain,
            route_path: req.route_path,
            ip: req.ip,
            version: req.version,
            invite_code: req.invite_code,
        },
    };

    match api_create_node(&api_request).await {
        Ok(pid) => {
            let identity = NODE_MANAGER.get_identity(pid);
            let _ = LOGGER.log_info(&format!(
                "Node registered via API: PID {} at {}",
                pid,
                identity
                    .as_ref()
                    .map(|node| node.url.as_str())
                    .unwrap_or("-")
            ));

            Ok(json_response(
                StatusCode::CREATED,
                json!({
                    "success": true,
                    "message": format!(
                        "Node '{}' registered successfully",
                        api_request.data.name
                    ),
                    "node": {
                        "pid": pid,
                        "name": identity.as_ref().map(|node| &node.name),
                        "url": identity.as_ref().map(|node| &node.url),
                        "domain": identity.as_ref().map(|node| &node.domain),
                        "route_path": identity.as_ref().map(|node| &node.route_path),
                        "ip": identity.as_ref().map(|node| &node.registered_from_ip),
                        "version": identity.as_ref().map(|node| &node.version),
                        "invite_code": identity.as_ref().map(|node| &node.invite_code),
                        "invited_by": identity.as_ref().and_then(|node| node.invited_by),
                    }
                }),
            ))
        }
        Err(err) => {
            let _ = LOGGER.log_error(&format!("Registration failed: {:?}", err));
            let status = match &err {
                NodeError::AlreadyExists => StatusCode::CONFLICT,
                NodeError::InvalidInput(_) => StatusCode::BAD_REQUEST,
                NodeError::NotFound => StatusCode::NOT_FOUND,
                NodeError::RegistryFull => StatusCode::SERVICE_UNAVAILABLE,
                NodeError::NotAllowed(_) | NodeError::InvalidInvite => StatusCode::FORBIDDEN,
                NodeError::IoError(_)
                | NodeError::JsonError(_)
                | NodeError::FailedValidation(_) => StatusCode::INTERNAL_SERVER_ERROR,
            };

            Ok(json_response(
                status,
                json!({
                    "success": false,
                    "message": format!("Failed to register node: {}", err)
                }),
            ))
        }
    }
}

async fn handle_unregister_node(body: Bytes) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let req: Value = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(err) => {
            return Ok(json_response(
                StatusCode::BAD_REQUEST,
                json!({
                    "success": false,
                    "message": format!("Invalid JSON: {}", err)
                }),
            ));
        }
    };

    let pid = match req
        .get("pid")
        .and_then(|pid| pid.as_u64())
        .map(|pid| pid as u32)
    {
        Some(pid) => pid,
        None => {
            return Ok(json_response(
                StatusCode::BAD_REQUEST,
                json!({
                    "success": false,
                    "message": "Missing or invalid 'pid' field"
                }),
            ));
        }
    };

    match delete_node(pid) {
        Ok(_) => {
            let _ = LOGGER.log_info(&format!("Node unregistered: PID {}", pid));
            Ok(json_response(
                StatusCode::OK,
                json!({
                    "success": true,
                    "message": format!("Node PID {} unregistered successfully", pid)
                }),
            ))
        }
        Err(err) => Ok(json_response(
            StatusCode::NOT_FOUND,
            json!({
                "success": false,
                "message": format!("Failed to unregister node: {}", err)
            }),
        )),
    }
}

async fn handle_list_nodes() -> Result<Response<Full<Bytes>>, hyper::Error> {
    let mut nodes = Vec::new();

    for pid in NODE_MANAGER.list() {
        if let Some(node) = NODE_MANAGER.get(pid) {
            nodes.push(json!({
                "pid": node.pid,
                "name": node.identity.name,
                "url": node.identity.url,
                "domain": node.identity.domain,
                "route_path": node.identity.route_path,
                "status": format!("{:?}", node.health.status),
                "invite_code": node.identity.invite_code,
                "invited_by": node.identity.invited_by
            }));
        }
    }

    Ok(json_response(
        StatusCode::OK,
        json!({
            "success": true,
            "count": nodes.len(),
            "nodes": nodes
        }),
    ))
}

async fn handle_allow_node(body: Bytes) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let req: AllowNodeRequest = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(err) => {
            return Ok(json_response(
                StatusCode::BAD_REQUEST,
                json!({
                    "success": false,
                    "message": format!("Invalid JSON: {}", err)
                }),
            ));
        }
    };

    match allow_node(&req.name, req.url.as_deref(), &req.domain, &req.route_path) {
        Ok(entry) => Ok(json_response(
            StatusCode::CREATED,
            json!({
                "success": true,
                "message": format!("Node '{}' added to allowlist", entry.name),
                "allowed_node": entry
            }),
        )),
        Err(err) => {
            let status = match &err {
                NodeError::AlreadyExists => StatusCode::CONFLICT,
                NodeError::InvalidInput(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Ok(json_response(
                status,
                json!({
                    "success": false,
                    "message": format!("{}", err)
                }),
            ))
        }
    }
}

async fn handle_list_allowed_nodes() -> Result<Response<Full<Bytes>>, hyper::Error> {
    let nodes = list_allowed_nodes();
    Ok(json_response(
        StatusCode::OK,
        json!({
            "success": true,
            "count": nodes.len(),
            "allowed_nodes": nodes
        }),
    ))
}

fn json_response(status: StatusCode, body: serde_json::Value) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

fn default_request_ip() -> String {
    "unknown".to_string()
}

fn default_request_version() -> String {
    "unknown".to_string()
}
