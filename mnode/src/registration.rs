/// Auto-registration dengan Melisa Management API
/// MNode mendaftar diri ke Melisa management API untuk menjadi bagian dari network
use crate::config::{NodeConfig, SECRET_MANAGEMENT_TOKEN};
use serde_json::json;

pub async fn register_with_melisa(config: &NodeConfig) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let melisa_url = config.melisa_url();

    let register_data = json!({
        "name": config.name,
        "url": config.node_url(),
        "domain": config.domain,
        "route_path": config.route_path,
        "ip": config.host,
        "version": env!("CARGO_PKG_VERSION"),
        "invite_code": config.invite_code,
    });

    // Send registration request dengan Header Authorization
    let response = client
        .post(format!("{}/register", melisa_url))
        // TODO PINDAHIN INI KE CONFIG
        .header("Authorization", format!("{}", SECRET_MANAGEMENT_TOKEN))
        .json(&register_data)
        .send()
        .await?;

    if response.status().is_success() {
        let body: serde_json::Value = response.json().await?;
        if body["success"].as_bool().unwrap_or(false) {
            // Extract registered node info
            if let Some(node) = body.get("node") {
                if let Some(pid) = node.get("pid") {
                    println!("✓ Assigned virtual PID: {}", pid);
                }
                if let Some(invite_code) = node.get("invite_code").and_then(|code| code.as_str()) {
                    println!("✓ Invite code: {}", invite_code);
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
