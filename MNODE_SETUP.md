# Melisa Node Architecture Implementation

## Overview
Implementasi sistem node management untuk Melisa, dengan fitur:
- ✅ Management API di port 8888 (bukan 80, menggunakan privilege port)
- ✅ Auto-registration untuk mnode saat startup  
- ✅ HTML file serving dan backend API di mnode
- ✅ JSON-based node management dari mcore/adapter

## Port Configuration

### Main Proxy Server
- **Port:** 8080 (bisa diubah di `melisa.conf`)
- **Host:** 127.0.0.1
- **Fungsi:** HTTP Reverse Proxy dengan load balancing

### Management API Server
- **Port:** 8888 (konfigurasi di `[management]` section di `melisa.conf`)
- **Host:** 127.0.0.1
- **Fungsi:** Node registration/unregistration/listing
- **Note:** Port ini DIKUNCI hanya untuk management operations

### MNode Worker
- **Port:** 3000 (default, bisa diubah dengan env var)
- **Fungsi:** Backend service dengan HTML + API endpoints

## Architecture

```
┌─────────────────────────────────────────┐
│         MELISA DAEMON (Port 8080)       │
│  - HTTP Reverse Proxy                   │
│  - Load Balancer                        │
│  - Node Router                          │
└──────┬──────────────────────────────────┘
       │
       ├──────────────────────────────────────┐
       │ ┌──────────────────────────────────┐ │
       │ │ Management API (Port 8888)       │ │
       │ │ - POST /register                 │ │
       │ │ - POST /unregister               │ │
       │ │ - GET /nodes                     │ │
       │ └──────────────────────────────────┘ │
       │                                      │
       └──────────────────────────────────────┘
       │
       ├──────────────────────────────┐
       │                              │
   ┌───▼─────┐                   ┌───▼─────┐
   │  MNode1 │                   │  MNode2 │
   │ :3000   │                   │ :3001   │
   └─────────┘                   └─────────┘
```

## Setup & Running

### 1. Build the project
```bash
cd /Users/saferoom/Documents/RUST/melisa_beta

# Build main melisa daemon
cargo build

# Build mnode
cd mnode && cargo build
```

### 2. Start Melisa Daemon
```bash
cargo run --bin melisa_beta
```

Output:
```
--- [MELISAD DAEMON STARTUP] ---
Config: melisa.conf
Listen: http://127.0.0.1:8080
Node registry: nodes.json (0 node)
Memulai sinkronisasi dan verifikasi node...
Node registry berhasil divalidasi dan disimpan.
Melisa proxy listening on http://127.0.0.1:8080
Management API listening on http://127.0.0.1:8888
  POST /register   - Register a new node
  POST /unregister - Unregister a node
  GET  /nodes      - List all nodes
```

### 3. Start MNode Worker (in another terminal)
```bash
cd mnode
cargo run --bin mnode

# Atau dengan custom configuration:
MNODE_NAME=my-service MNODE_PORT=3000 MNODE_DOMAIN=api.local MNODE_ROUTE_PATH=/api cargo run
```

Output:
```
--- [MNODE STARTUP] ---
Node name: mnode-hostname
Node port: 3000
Melisa management: http://127.0.0.1:8888
✓ Successfully registered with Melisa
Node listening on http://127.0.0.1:3000
  GET  /          - HTML Homepage
  GET  /api/info  - Node info
  GET  /api/health - Health check
```

## API Endpoints

### Management API (Port 8888)

#### 1. Register Node
```bash
curl -X POST http://localhost:8888/register \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-service",
    "pid": 1234,
    "url": "http://127.0.0.1:3000",
    "domain": "api.local",
    "route_path": "/api"
  }'

# Response:
{
  "success": true,
  "message": "Node 'my-service' registered successfully",
  "node": {
    "hash": "abc123...",
    "name": "my-service",
    "url": "http://127.0.0.1:3000",
    "domain": "api.local",
    "route_path": "/api"
  }
}
```

#### 2. List Nodes
```bash
curl http://localhost:8888/nodes

# Response:
{
  "success": true,
  "count": 1,
  "nodes": [
    {
      "hash": "abc123...",
      "name": "my-service",
      "url": "http://127.0.0.1:3000",
      "domain": "api.local",
      "route_path": "/api",
      "status": "Active"
    }
  ]
}
```

#### 3. Unregister Node
```bash
curl -X POST http://localhost:8888/unregister \
  -H "Content-Type: application/json" \
  -d '{"hash": "abc123..."}'

# Response:
{
  "success": true,
  "message": "Node 'abc123...' unregistered successfully"
}
```

### MNode API (Port 3000)

#### 1. Homepage
```bash
curl http://localhost:3000/
# Returns: HTML page with node information
```

#### 2. Node Info
```bash
curl http://localhost:3000/api/info

# Response:
{
  "status": "active",
  "name": "mnode-hostname",
  "url": "http://127.0.0.1:3000",
  "domain": "mnode.local",
  "route_path": "/mnode",
  "pid": 12345,
  "timestamp": "2024-06-12T10:30:45Z"
}
```

#### 3. Health Check
```bash
curl http://localhost:3000/api/health

# Response:
{
  "status": "healthy",
  "timestamp": "2024-06-12T10:30:45Z"
}
```

## Configuration

### melisa.conf

```toml
# Main proxy
host = "127.0.0.1"
port = 8080

# Management API (JANGAN GANTI PORT INI)
[management]
port = 8888
enabled = true

# Logging
[logging]
log_dir = "./logs"
access_log_enabled = true
error_log_enabled = true
level = "info"

# Node management
[nodes]
storage_file = "nodes.json"
flush_threshold_bytes = 51200
health_check_interval_secs = 30

# Proxy settings
[proxy]
load_balancer_strategy = "round_robin"
request_timeout_secs = 30
max_idle_per_host = 32
max_retries = 3
retry_backoff_ms = 100
metrics_report_interval_secs = 60
```

### MNode Environment Variables

```bash
MNODE_NAME              # Node identifier (default: mnode-{hostname})
MNODE_PORT              # Listen port (default: 3000)
MELISA_HOST             # Melisa host (default: 127.0.0.1)
MELISA_PORT             # Melisa management port (default: 8888)
MNODE_DOMAIN            # Domain untuk routing (default: mnode.local)
MNODE_ROUTE_PATH        # Route path prefix (default: /mnode)
```

Example:
```bash
MNODE_NAME=api-service \
MNODE_PORT=3001 \
MNODE_DOMAIN=api.example.com \
MNODE_ROUTE_PATH=/api \
cargo run --bin mnode
```

## Testing Workflow

### 1. Terminal 1 - Start Melisa
```bash
cd /Users/saferoom/Documents/RUST/melisa_beta
cargo run --bin melisa_beta
```

### 2. Terminal 2 - Check Management API is running
```bash
curl http://localhost:8888/nodes
# Should return: {"success": true, "count": 0, "nodes": []}
```

### 3. Terminal 3 - Start MNode (auto-registers)
```bash
cd /Users/saferoom/Documents/RUST/melisa_beta/mnode
cargo run --bin mnode
```

### 4. Terminal 2 - Verify node registered
```bash
curl http://localhost:8888/nodes
# Should show 1 registered node
```

### 5. Terminal 4 - Test MNode endpoints
```bash
# Homepage
curl http://localhost:3000/

# Node info
curl http://localhost:3000/api/info

# Health check
curl http://localhost:3000/api/health
```

## Key Implementation Details

### 1. Port 80 Issue Resolution
- **Problem:** Port 80 memerlukan root privileges (permission denied)
- **Solution:** Menggunakan port 8080 untuk main proxy (non-privileged)
- **Alternative:** Jika perlu port 80, gunakan reverse proxy di nginx dengan setups yang tepat

### 2. Management Port 8888
- Dedicated port untuk node management operations
- Tidak digunakan untuk proxying traffic normal
- Terlindungi configuration file

### 3. MNode Auto-Registration
- Saat startup, mnode mengirim POST request ke `/register`
- Menggunakan PID sebagai unique identifier
- Automatic URL dan domain configuration

### 4. JSON Node Management
- Menggunakan sistem dari `mcore/adapter/json.rs`
- Copy-on-Write semantics untuk thread safety
- Persistent storage ke `nodes.json`

## Troubleshooting

### Problem: "Permission denied (os error 13)"
**Solution:** This was the port 80 issue. Now using port 8080 which doesn't require root.

### Problem: "Failed to register with Melisa"
**Check:**
1. Pastikan Melisa daemon sudah running: `curl http://localhost:8888/nodes`
2. Pastikan MELISA_HOST dan MELISA_PORT benar
3. Check logs untuk error detail

### Problem: Node tidak muncul di /nodes list
**Check:**
1. Lihat output dari mnode saat startup
2. Pastikan registration request berhasil (status 201)
3. Check melisa logs untuk error handling

### Problem: Routing tidak bekerja
**Check:**
1. Pastikan domain dan route_path configuration benar
2. Pastikan mnode server aktif dan listening
3. Check proxy logs untuk routing decisions

## Architecture Notes

### Thread Safety
- Node manager menggunakan Arc<RwLock> untuk thread-safe access
- Copy-on-Write semantics untuk zero-copy updates
- Atomic tracking untuk accumulated bytes

### Performance
- Connection pooling untuk HTTP client
- Round-robin load balancing
- Efficient JSON serialization dengan serde

### Scalability
- Multiple mnode instances dapat diregistrasi
- Load balancer secara otomatis mendistribusikan traffic
- Health checks periodic untuk node availability
