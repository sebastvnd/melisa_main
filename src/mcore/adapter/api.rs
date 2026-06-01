use serde::{Deserialize, Serialize};

use crate::mcore::{adapter::api::Action::CreateNode, api::services::create_node};

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiRequest<T> {
    pub version: String,
    pub action: Action,
    pub request_id: String,
    pub timestamp: u64,
    pub data: T,
}

pub struct ApiResponse<T> {
    pub request_id: String,
    pub success: bool,
    pub code: u16,
    pub message: String,
    pub data: Option<T>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateNodeData {
    name: String,
    pid: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Action {
    CreateNode,
    DeleteNode,
}

pub fn ex_create_node(name: &str,  pid: u32) {
    let response = create_node(name, pid);
    println!("{}", response);
}

// data palsu untuk tujuan testing api CreateNode format data seperti ini
// berlaku untuk aplikasi dari luar ini jadi titik masuk dari data flow
// program ini - best erick mantap
pub fn fake_data_request() -> ApiRequest<CreateNodeData> {
    ApiRequest  {
        version: "1.0".to_string(), 
        action: Action::CreateNode,
        request_id: "id001".to_string(),
        timestamp: 17828661,
        data: CreateNodeData {
            name: "melisa beta".to_string(),
            pid: 808,
        },
    }
}

// TODO masih eror coba pikirin lagi data flownya
// coba untuk bersihkan datanya dulu - ok sukses sudah
pub fn execute(request: &ApiRequest<CreateNodeData>) {
    match request.action {
        Action::CreateNode => {
            ex_create_node(&request.data.name, request.data.pid);
        }
        Action::DeleteNode => {
            println!("DeleteNode action executed");
        }
    }
}