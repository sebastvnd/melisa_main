/// HTTP Request Handler untuk mnode
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use http_body_util::Full;
use hyper::body::Bytes;
use serde_json::json;
use std::sync::Arc;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::NodeConfig;

pub async fn handle_request(
    req: Request<Incoming>,
    config: Arc<NodeConfig>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let (parts, _body) = req.into_parts();
    let path = parts.uri.path();

    // Try to serve static file first (jika enabled)
    if config.static_files_enabled {
        if let Ok(response) = serve_static_file(path, &config) {
            return Ok(response);
        }
    }

    // Handle API endpoints
    let response = match path {
        "/" => serve_default_page(&config),
        "/api/info" => serve_node_info(&config),
        "/api/health" => serve_health_check(),
        _ => serve_not_found(),
    };

    Ok(response)
}

/// Serve static files dari configured directory
fn serve_static_file(path: &str, config: &NodeConfig) -> Result<Response<Full<Bytes>>, String> {
    // Normalize path - remove leading slash
    let file_path = if path == "/" {
        "index.html".to_string()
    } else {
        path.trim_start_matches('/').to_string()
    };

    // Security: prevent directory traversal
    if file_path.contains("..") || file_path.contains("//") {
        return Err("Invalid path".to_string());
    }

    // Construct full path
    let full_path = PathBuf::from(&config.static_files_dir).join(&file_path);

    // Check if file exists
    if !full_path.exists() {
        return Err("File not found".to_string());
    }

    // Only serve files, not directories
    if !full_path.is_file() {
        return Err("Not a file".to_string());
    }

    // Read file content
    let content = fs::read(&full_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // Guess MIME type
    let mime_type = mime_guess::from_path(&full_path)
        .first_raw()
        .unwrap_or("application/octet-stream");

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", mime_type)
        .header("Cache-Control", "public, max-age=3600")
        .body(Full::new(Bytes::from(content)))
        .unwrap())
}

/// Serve default page (jika tidak ada index.html di static)
fn serve_default_page(config: &NodeConfig) -> Response<Full<Bytes>> {
    // First, try to serve index.html from static directory
    if let Ok(response) = serve_static_file("/index.html", config) {
        return response;
    }

    // Fallback: Generate default HTML page
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - Melisa Node</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }}
        
        .container {{
            background: white;
            border-radius: 10px;
            box-shadow: 0 20px 60px rgba(0,0,0,0.3);
            max-width: 600px;
            width: 100%;
            padding: 40px;
        }}
        
        h1 {{
            color: #333;
            margin-bottom: 10px;
            font-size: 2.5em;
        }}
        
        .subtitle {{
            color: #666;
            margin-bottom: 30px;
            font-size: 1.1em;
        }}
        
        .info-section {{
            background: #f5f5f5;
            padding: 20px;
            border-radius: 5px;
            margin: 20px 0;
            border-left: 4px solid #667eea;
        }}
        
        .info-item {{
            display: flex;
            justify-content: space-between;
            padding: 10px 0;
            border-bottom: 1px solid #ddd;
        }}
        
        .info-item:last-child {{
            border-bottom: none;
        }}
        
        .info-label {{
            color: #666;
            font-weight: 500;
        }}
        
        .info-value {{
            color: #333;
            font-family: 'Courier New', monospace;
            font-weight: bold;
        }}
        
        .status {{
            display: inline-block;
            background: #4CAF50;
            color: white;
            padding: 5px 15px;
            border-radius: 20px;
            font-size: 0.9em;
            margin-top: 20px;
        }}
        
        .api-section {{
            margin-top: 30px;
        }}
        
        .api-endpoint {{
            background: #f9f9f9;
            padding: 15px;
            border-radius: 5px;
            margin: 10px 0;
            border-left: 3px solid #764ba2;
            font-family: 'Courier New', monospace;
            font-size: 0.9em;
        }}
        
        .note {{
            background: #fff3cd;
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
            border-left: 4px solid #ffc107;
            color: #856404;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 {} Node</h1>
        <p class="subtitle">Melisa Network Node - Status Active</p>
        
        <div class="note">
            <strong>📁 Custom Files:</strong> Letakkan HTML/CSS/JS files di folder <code>public/html</code> di mnode directory.
            MNode akan auto-serve files tersebut dari / path.
        </div>
        
        <div class="info-section">
            <h2>Node Information</h2>
            <div class="info-item">
                <span class="info-label">Node URL:</span>
                <span class="info-value">{}</span>
            </div>
            <div class="info-item">
                <span class="info-label">Domain:</span>
                <span class="info-value">{}</span>
            </div>
            <div class="info-item">
                <span class="info-label">Route Path:</span>
                <span class="info-value">{}</span>
            </div>
            <div class="info-item">
                <span class="info-label">Static Files Dir:</span>
                <span class="info-value">{}</span>
            </div>
            <div class="status">✓ ACTIVE</div>
        </div>
        
        <div class="api-section">
            <h2>Available APIs</h2>
            <div class="api-endpoint">GET /api/info - Node information (JSON)</div>
            <div class="api-endpoint">GET /api/health - Health check status (JSON)</div>
        </div>
        
        <div class="api-section">
            <h2>Configuration</h2>
            <p>Edit <code>mnode.conf</code> di root directory untuk mengatur:</p>
            <ul style="margin-left: 20px; margin-top: 10px;">
                <li>Port dan host</li>
                <li>Direktori static files</li>
                <li>Melisa server connection</li>
                <li>Domain dan route path</li>
            </ul>
        </div>
    </div>
</body>
</html>"#,
        config.route_path,
        config.route_path,
        config.node_url(),
        config.domain,
        config.route_path,
        config.static_files_dir,
    );

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(html)))
        .unwrap()
}

fn serve_node_info(config: &NodeConfig) -> Response<Full<Bytes>> {
    let info = json!({
        "status": "active",
        "url": config.node_url(),
        "domain": config.domain,
        "route_path": config.route_path,
        "pid": std::process::id(),
        "static_files_enabled": config.static_files_enabled,
        "static_files_dir": config.static_files_dir,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(info.to_string())))
        .unwrap()
}

fn serve_health_check() -> Response<Full<Bytes>> {
    let health = json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(health.to_string())))
        .unwrap()
}

fn serve_not_found() -> Response<Full<Bytes>> {
    let error = json!({
        "error": "Not Found",
        "status": 404,
        "message": "Endpoint atau file tidak ditemukan"
    });

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(error.to_string())))
        .unwrap()
}
