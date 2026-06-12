# Melisa Architecture - Data Flow

## 🎯 Overview

Arsitektur Melisa mengikuti pola **adapter → api → melisad** yang memisahkan concerns:

```
External Request
      ↓
Handler (melisad/management/handler.rs)
      ↓
Adapter (mcore/adapter/json.rs) - Format/validation
      ↓
API Service (mcore/api/services.rs) - Business logic
      ↓
Melisad (mcore/melisad/services/node/) - Operations
      ↓
Response
```

---

## 📋 Detailed Flow: Node Registration

### **1. MNode Registration Request (mnode/src/registration.rs)**

MNode mengirim HTTP POST request **tanpa PID**:

```json
POST http://localhost:8888/register
{
  "name": "mnode",
  "url": "http://127.0.0.1:3000",
  "domain": "mnode.local",
  "route_path": "/mnode"
}
```

**Why no PID?**
- Real OS PIDs biasanya < 100,000
- Sistem expects virtual PIDs (100k-999k) untuk managed nodes
- API layer akan generate virtual PID dari node identifier

---

### **2. Management Handler (management/handler.rs)**

**Step 1: Parse HTTP Request**
```rust
// Parse body → RegisterNodeRequest
let req: RegisterNodeRequest = serde_json::from_slice(&body)?;

// RegisterNodeRequest {
//   name: "mnode",
//   pid: None,           ← Optional (tidak dikirim dari mnode)
//   url: "http://127.0.0.1:3000",
//   domain: "mnode.local",
//   route_path: "/mnode"
// }
```

**Step 2: Create Standardized ApiRequest**
```rust
let api_request = ApiRequest {
    version: "1.0".to_string(),
    action: Action::CreateNode,
    request_id: Uuid::new_v4().to_string(),  // Unique request tracking
    timestamp: Utc::now().timestamp() as u64,
    data: CreateNodeData {
        name: req.name,
        pid: req.pid,  // None - akan di-generate
        url: req.url,
        domain: req.domain,
        route_path: req.route_path,
    },
};
```

**Step 3: Call Adapter**
```rust
api_create_node(&api_request)
```

---

### **3. Adapter Layer (adapter/json.rs)**

**Purpose**: Konversi dari HTTP ke standardized API format

```rust
pub fn api_create_node(request: &ApiRequest<CreateNodeData>) 
    -> Result<NodeProcess, NodeError> {
    
    // Extract data dari ApiRequest wrapper
    create_node(
        &request.data.name,
        request.data.pid,              // None
        &request.data.url,
        &request.data.domain,
        &request.data.route_path,
    )
}
```

---

### **4. API Service Layer (api/services.rs)**

**Purpose**: Business logic dan validation

```rust
pub fn create_node(
    name: &str,
    pid: Option<u32>,                  // ← Optional from adapter
    url: &str,
    domain: &str,
    route_path: &str,
) -> Result<NodeProcess, NodeError> {
    
    // Step 1: Validate input
    if name.trim().is_empty() {
        return Err(NodeError::InvalidInput("name cannot be empty"));
    }
    
    // Step 2: Handle PID
    let final_pid = match pid {
        Some(p) if (PID_START..=PID_END).contains(&p) => p,  // Valid PID
        Some(_) => {
            // PID out of range → generate virtual
            generate_virtual_pid(&format!("{}-{}", name, url))
        }
        None => {
            // No PID → generate virtual
            generate_virtual_pid(&format!("{}-{}", name, url))
        }
    };
    
    // Step 3: Delegate to melisad layer
    NODE_MANAGER.create(name, final_pid, url, domain, route_path)
}
```

**Virtual PID Generation:**
```rust
pub fn generate_virtual_pid(node_identifier: &str) -> u32 {
    let mut hasher = DefaultHasher::new();
    node_identifier.hash(&mut hasher);
    let hash_value = hasher.finish();
    
    // Map to valid range (100,000 - 999,999)
    let range_size = (PID_END - PID_START + 1) as u64;
    let virtual_pid = PID_START as u64 + (hash_value % range_size);
    virtual_pid as u32
}
```

**Result untuk "mnode-http://127.0.0.1:3000":**
- Hash: Deterministic
- Virtual PID: 951364 (consistent untuk node yang sama)

---

### **5. Melisad Layer (melisad/services/node/operations.rs)**

**Purpose**: Actual node management operations

```rust
impl NodeManager {
    pub fn create(
        &self,
        name: &str,
        pid: u32,                      // ← Virtual PID (951364)
        url: &str,
        domain: &str,
        route_path: &str,
    ) -> Result<NodeProcess, NodeError> {
        
        // Step 1: Validate PID is in valid range
        if !(PID_START..=PID_END).contains(&pid) {
            return Err(NodeError::InvalidInput("pid out of allowed range"));
        }
        
        // Step 2: Generate node hash dari name
        let hash = generate_hash(name);
        // hash = "3ca6f4feb922a9087e3bf77c50dd6ece638407d7474f4b3203fc4ca515f6d643"
        
        // Step 3: Check duplicate
        let mut processes_lock = self.processes.write().unwrap();
        let mut new_map = (*processes_lock.clone()).clone();
        
        if new_map.contains_key(&hash) {
            return Err(NodeError::AlreadyExists);
        }
        
        // Step 4: Create NodeProcess
        let node = NodeProcess {
            hash: hash.clone(),
            name: name.to_string(),
            pid,                       // Virtual PID
            url,
            domain,
            route_path,
            status: NodeStatus::Active,
        };
        
        // Step 5: Store in memory (Arc<HashMap>)
        new_map.insert(hash, node.clone());
        *processes_lock = std::sync::Arc::new(new_map);
        
        // Step 6: Persist to disk jika perlu
        let current_accumulated = self.accumulated_bytes
            .fetch_add(estimated_bytes, Ordering::SeqCst) + estimated_bytes;
        
        if current_accumulated >= CONFIG.nodes.flush_threshold_bytes {
            self.flush()?;  // Write to nodes.json
        }
        
        Ok(node)
    }
}
```

---

### **6. Response Flow (Reverse)**

Response mengalir kembali melalui layer yang sama:

```
melisad (NodeProcess)
  ↓
api/services (returns NodeProcess)
  ↓
adapter (wraps dalam ApiRequest response)
  ↓
handler (converts to JSON HTTP response)
  ↓
HTTP 201 Created
{
  "success": true,
  "message": "Node 'mnode' registered successfully",
  "node": {
    "hash": "3ca6f4feb922a9087e3bf77c50dd6ece638407d7474f4b3203fc4ca515f6d643",
    "name": "mnode",
    "url": "http://127.0.0.1:3000",
    "domain": "mnode.local",
    "route_path": "/mnode",
    "pid": 951364
  }
}
```

---

## 🏗️ Layer Architecture

### **Handler Layer (melisad/management/handler.rs)**
- **Responsibility**: HTTP parsing, request/response formatting
- **Input**: HTTP body (bytes)
- **Output**: HTTP response (JSON)
- **Operations**: Parse, validate JSON structure, call adapter

### **Adapter Layer (mcore/adapter/json.rs)**
- **Responsibility**: Format translation, standardization
- **Input**: RegisterNodeRequest (HTTP format)
- **Output**: Standardized ApiRequest with metadata
- **Operations**: Add request_id, timestamp, version tracking
- **Key function**: `api_create_node()` → calls `create_node()` in API service

### **API Service Layer (mcore/api/services.rs)**
- **Responsibility**: Business logic, validation, transformation
- **Input**: Parsed data (name, pid, url, etc.)
- **Output**: NodeProcess atau Error
- **Operations**: Validate input, generate virtual PIDs, call melisad
- **Key function**: `create_node()` with PID handling logic

### **Melisad Layer (mcore/melisad/)**
- **Responsibility**: Actual data persistence, state management
- **Input**: Validated data from API service
- **Output**: NodeProcess or NodeError
- **Operations**: Memory management (Arc, RwLock), persistence (nodes.json), node discovery

---

## 💡 Key Benefits

| Aspect | Benefit |
|--------|---------|
| **Separation of Concerns** | Each layer punya tanggung jawab jelas |
| **Testability** | Bisa test adapter tanpa HTTP, api tanpa adapter, dll |
| **Reusability** | API service bisa dipanggil dari berbagai handler (HTTP, gRPC, CLI, etc) |
| **Maintenance** | Perubahan HTTP format hanya affect handler, bukan API logic |
| **Request Tracking** | ApiRequest dengan request_id untuk audit/logging |

---

## 🔄 Comparison: Before vs After

### **Before (Incorrect)**
```
HTTP request
  ↓
handler
  ↓
NODE_MANAGER.create()  ← Direct call, no abstraction
  ↓
Response
```

**Problems:**
- Tight coupling antara HTTP dan business logic
- Susah test API tanpa HTTP server
- PID validation terlalu strict (reject real OS PIDs)
- No standardized request format

### **After (Correct)**
```
HTTP request
  ↓
handler (parse HTTP)
  ↓
adapter (standardize format) → ApiRequest with metadata
  ↓
api/services (business logic) → handles Optional PID, generates virtual PID
  ↓
melisad (NODE_MANAGER) → operates on valid data
  ↓
Response
```

**Improvements:**
✅ Clean separation of concerns
✅ Flexible PID handling (optional, auto-generate)
✅ Request tracking via request_id
✅ Standardized API format
✅ Easy to test each layer independently
✅ Can extend handler for other protocols (gRPC, CLI, etc) reusing same API layer

---

## 🧪 Real Example: Node Registration Flow

### Terminal Output Menunjukkan Alur:

```
[MNode]
Connecting to Melisa Management API...

  ↓ HTTP POST /register
  ↓ No PID in request
  
[Management Handler]
✓ Parsed request body
✓ Created ApiRequest with request_id

  ↓ Calls adapter
  
[Adapter Layer]
✓ api_create_node() called

  ↓ Calls API service
  
[API Service]
✓ PID is None
✓ Generating virtual PID from "mnode-http://127.0.0.1:3000"
✓ Virtual PID: 951364
✓ Calling NODE_MANAGER.create()

  ↓ Calls melisad
  
[Melisad Layer]
✓ Validating: 951364 in range 100k-999k ✓
✓ Generating hash from "mnode"
✓ Hash: 3ca6f4feb922a9087e3bf77c50dd6ece...
✓ Storing in memory + persisting to nodes.json

  ↓ Returns NodeProcess
  
[Response]
HTTP 201 Created with node details
```

---

## 📊 PID Handling Deep Dive

### **PID Range Configuration**
```rust
// mcore/melisad/services/mconf.rs
pub const PID_START: u32 = 100_000;   // Min virtual PID
pub const PID_END: u32 = 999_999;     // Max virtual PID
```

### **Why Virtual PIDs?**

Real OS PIDs:
- Range: 1 to ~32,768 (varies by OS)
- MNode OS PID: ~74,000 (actual process ID)
- Problem: Falls within range but represents OS process, not managed node

Virtual PIDs for Melisa:
- Range: 100,000 to 999,999 (reserved for managed nodes)
- Generated deterministically from node identifier
- Consistent across restarts for same node
- Won't conflict with real OS PIDs

### **Generation Algorithm**

```rust
generate_virtual_pid("mnode-http://127.0.0.1:3000")
  ↓
Hash string dengan DefaultHasher
  ↓
Ambil hash value (u64)
  ↓
Map ke range 100k-999k: (hash % 899,999) + 100,000
  ↓
Result: 951,364 (deterministic)
```

---

## 🛠️ Extension Points

### **Add gRPC Handler**
```rust
// mcore/melisad/grpc/handler.rs
pub fn handle_grpc_register(req: GrpcRegisterRequest) {
    // Convert gRPC request to ApiRequest
    let api_request = ApiRequest { ... };
    
    // Reuse same API layer
    api_create_node(&api_request)
}
```

### **Add CLI**
```rust
// cli/main.rs
fn cmd_register_node(name: &str, url: &str, domain: &str, route_path: &str) {
    // Call API service directly (no HTTP needed)
    create_node(name, None, url, domain, route_path)
}
```

### **Add Persistence Layer**
```rust
// mcore/api/services.rs
pub fn create_node(...) {
    // ... existing logic ...
    
    // Could add database persistence here
    database.insert(node)?;
    
    NODE_MANAGER.create(...)
}
```

---

## 📝 Summary

Melisa architecture sekarang mengikuti clean architecture principles:

1. **Handler**: HTTP protocol adapter
2. **Adapter**: Data format translator
3. **API Service**: Business logic (validation, transformation, generation)
4. **Melisad**: Actual data operations

Alur data: `adapter → api → melisad` memastikan:
- ✅ Scalable
- ✅ Testable
- ✅ Maintainable
- ✅ Extensible
