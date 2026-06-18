use chrono::Utc;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
/// Management API Handler - handle register/unregister node requests
/// Alur data:
/// HTTP request → handler (parsing) → adapter (format) → api/services (logic) → melisad (NODE_MANAGER)
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::mcore::adapter::json::{Action, ApiRequest, CreateNodeData, api_create_node};
use crate::mcore::api::services::delete_node;
use crate::mcore::config::load_config::SECRET_MANAGEMENT_TOKEN;
use crate::mcore::errors::enode::NodeError;
use crate::mcore::melisad::services::node::NODE_MANAGER;
use crate::mcore::mlog::LOGGER;

#[derive(serde::Deserialize, Debug)]
pub struct RegisterNodeRequest {
    pub name: String,
    #[serde(default)]
    pub pid: Option<u32>, // Optional - will be generated in API layer
    pub url: String,
    pub domain: String,
    pub route_path: String,
}

#[derive(serde::Serialize)]
pub struct RegisterNodeResponse {
    pub success: bool,
    pub message: String,
    pub node_hash: Option<String>,
}

pub async fn handle_management_request(
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let (parts, body) = req.into_parts();

    // --- TAMBAHKAN PROTEKSI AUTENTIKASI ---
    let mut authenticated = false;
    if let Some(auth_header) = parts.headers.get(hyper::header::AUTHORIZATION)
        && let Ok(auth_str) = auth_header.to_str()
        && auth_str == SECRET_MANAGEMENT_TOKEN
    {
        authenticated = true;
    }

    if !authenticated {
        let error_body = serde_json::json!({
            "success": false,
            "message": "Unauthorized: Invalid or missing management token"
        });
        return Ok(Response::builder()
            .status(hyper::StatusCode::UNAUTHORIZED) // 401 Unauthorized
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(error_body.to_string())))
            .unwrap());
    }
    // --------------------------------------

    let method = parts.method.clone();
    let path = parts.uri.path().to_string();
    let body_bytes = body.collect().await?.to_bytes();

    let response = match (method.as_str(), path.as_str()) {
        ("POST", "/register") => handle_register_node(body_bytes).await,
        ("POST", "/unregister") => handle_unregister_node(body_bytes).await,
        ("GET", "/nodes") => handle_list_nodes().await,
        _ => {
            let error_body = json!({
                "success": false,
                "message": "Endpoint not found"
            });
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(error_body.to_string())))
                .unwrap())
        }
    };

    response
}

fn build_response(
    status: StatusCode,
    body: serde_json::Value,
) -> Result<Response<Full<Bytes>>, hyper::http::Error> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
}

async fn handle_register_node(body: Bytes) -> Result<Response<Full<Bytes>>, hyper::Error> {
    // Step 1: Parse HTTP request → RegisterNodeRequest
    let req: RegisterNodeRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            let error_body = json!({
                "success": false,
                "message": format!("Invalid JSON: {}", e)
            });
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(error_body.to_string())))
                .unwrap());
        }
    };

    // Step 2: Create ApiRequest wrapper (adapter layer)
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
        },
    };

    // Step 3: Call adapter → api/services → melisad (NODE_MANAGER.create)
    match api_create_node(&api_request) {
        Ok(node) => {
            let _ = LOGGER.log_info(&format!(
                "Node registered via API: {} at {}",
                node.name, node.url
            ));
            let response_body = json!({
                "success": true,
                "message": format!("Node '{}' registered successfully", node.name),
                "node": {
                    "hash": node.hash,
                    "name": node.name,
                    "url": node.url,
                    "domain": node.domain,
                    "route_path": node.route_path,
                }
            });
            Ok(Response::builder()
                .status(StatusCode::CREATED)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(response_body.to_string())))
                .unwrap())
        }
        Err(e) => {
            let _ = LOGGER.log_error(&format!("Registration failed: {:?}", e));

            // Pilih HTTP status code berdasarkan tipe error
            let status = match &e {
                NodeError::AlreadyExists => StatusCode::CONFLICT, // 409
                NodeError::InvalidInput(_) => StatusCode::BAD_REQUEST, // 400
                NodeError::NotFound => StatusCode::NOT_FOUND,     // 404
                NodeError::IoError(_)
                | NodeError::JsonError(_)
                | NodeError::FailedValidation(_) => StatusCode::INTERNAL_SERVER_ERROR, // 500
            };

            let error_body = json!({
                "success": false,
                "message": format!("Failed to register node: {}", e)   // gunakan Display, bukan Debug
            });
            Ok(Response::builder()
                .status(status)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(error_body.to_string())))
                .unwrap())
        }
    }
}

async fn handle_unregister_node(body: Bytes) -> Result<Response<Full<Bytes>>, hyper::Error> {
    // Parse request body - expect {"hash": "xxx"}
    let req: Value = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            let error_body = json!({
                "success": false,
                "message": format!("Invalid JSON: {}", e)
            });
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(error_body.to_string())))
                .unwrap());
        }
    };

    let hash = match req.get("hash").and_then(|h| h.as_str()) {
        Some(h) => h,
        None => {
            let error_body = json!({
                "success": false,
                "message": "Missing 'hash' field"
            });
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(error_body.to_string())))
                .unwrap());
        }
    };

    // Try to delete node
    match delete_node(hash) {
        Ok(_) => {
            let _ = LOGGER.log_info(&format!("Node unregistered: {}", hash));
            let response_body = json!({
                "success": true,
                "message": format!("Node '{}' unregistered successfully", hash)
            });
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(response_body.to_string())))
                .unwrap())
        }
        Err(e) => {
            let error_body = json!({
                "success": false,
                "message": format!("Failed to unregister node: {:?}", e)
            });
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(error_body.to_string())))
                .unwrap())
        }
    }
}

async fn handle_list_nodes() -> Result<Response<Full<Bytes>>, hyper::Error> {
    match NODE_MANAGER.list() {
        Some(node_hashes) => {
            // Get full node info for each hash
            let mut nodes = vec![];
            for hash in node_hashes {
                if let Some(node) = NODE_MANAGER.get(&hash) {
                    nodes.push(json!({
                        "hash": node.hash,
                        "name": node.name,
                        "url": node.url,
                        "domain": node.domain,
                        "route_path": node.route_path,
                        "status": format!("{:?}", node.status)
                    }));
                }
            }

            let response_body = json!({
                "success": true,
                "count": nodes.len(),
                "nodes": nodes
            });
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(response_body.to_string())))
                .unwrap())
        }
        None => {
            let response_body = json!({
                "success": true,
                "count": 0,
                "nodes": []
            });
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(response_body.to_string())))
                .unwrap())
        }
    }
}
