/// Auto-registration dengan Melisa Management API
/// MNode mendaftar diri ke Melisa management API untuk menjadi bagian dari network
use serde_json::json;
use crate::config::NodeConfig;

pub async fn register_with_melisa(config: &NodeConfig) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let melisa_url = config.melisa_url();
    
    // Prepare registration data
    // NOTE: pid is optional - management API will generate virtual PID if not provided
    let register_data = json!({
        "name": config.name,       // ← Use node name, not route_path
        // "pid": Not sending OS PID - let management API generate virtual PID
        "url": config.node_url(),
        "domain": config.domain,
        "route_path": config.route_path,
    });

    // Send registration request
    let response = client
        .post(format!("{}/register", melisa_url))
        .json(&register_data)
        .send()
        .await?;

    if response.status().is_success() {
        let body: serde_json::Value = response.json().await?;
        if body["success"].as_bool().unwrap_or(false) {
            // Extract registered node info
            if let Some(node) = body.get("node") {
                if let Some(hash) = node.get("hash").and_then(|h| h.as_str()) {
                    println!("✓ Registered with hash: {}", hash);
                }
                if let Some(pid) = node.get("pid") {
                    println!("✓ Assigned virtual PID: {}", pid);
                }
            }
            return Ok(());
        } else {
            return Err(format!(
                "Registration failed: {}",
                body["message"].as_str().unwrap_or("Unknown error")
            )
            .into());
        }
    } else {
        return Err(format!("HTTP {}: {}", response.status(), response.text().await?).into());
    }
}
