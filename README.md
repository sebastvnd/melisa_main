# Melisa - Open Server Architecture

<div align="center">

![Melisa](https://img.shields.io/badge/Melisa-0.1.0--beta-blue) ![License](https://img.shields.io/badge/License-MIT-green) ![Language](https://img.shields.io/badge/Language-Rust-orange) ![Status](https://img.shields.io/badge/Status-Experimental-yellow)

An open-source server architecture inspired by Pingora/Nginx, entirely written in Rust. A high-performance, modern proxy gateway system designed for distributed architectures.

[Features](#features) • [Getting Started](#getting-started) • [Architecture](#architecture) • [Documentation](#documentation)

</div>

---

## Overview

**Melisa** is an experimental yet powerful open-source reverse proxy and load balancer written in Rust. It combines the architectural principles of industry-leading proxies like Pingora and Nginx with modern Rust concurrency patterns to create a fast, safe, and efficient server architecture.

This project demonstrates cutting-edge approaches to:
- Asynchronous I/O with Tokio runtime
- Thread-safe concurrent access patterns
- High-performance network proxying
- Distributed node management
- Enterprise-grade logging and monitoring

### Key Highlights

- **100% Rust Implementation**: Leverages Rust's performance and memory safety
- **Tokio-based Async Runtime**: Handles thousands of concurrent connections efficiently
- **Production-Ready Features**: Load balancing, health checks, metrics, and logging
- **Distributed Architecture**: Support for multiple worker nodes with dynamic registration
- **Configuration as Code**: TOML-based configuration for flexibility
- **Comprehensive Logging**: Nginx-style access logs with rotation and multiple log levels

---

## Table of Contents

1. [Features](#features)
2. [Architecture Overview](#architecture-overview)
3. [System Requirements](#system-requirements)
4. [Installation](#installation)
5. [Getting Started](#getting-started)
6. [Configuration](#configuration)
7. [Running Melisa](#running-melisa)
8. [Worker Nodes (MNode)](#worker-nodes-mnode)
9. [API Documentation](#api-documentation)
10. [Load Balancing Strategies](#load-balancing-strategies)
11. [Health Checking](#health-checking)
12. [Logging System](#logging-system)
13. [Metrics and Monitoring](#metrics-and-monitoring)
14. [Development](#development)
15. [Troubleshooting](#troubleshooting)
16. [Contributing](#contributing)
17. [License](#license)

---

## Features

### Core Features

✅ **High-Performance Proxy**
- Asynchronous, non-blocking I/O using Tokio
- Capable of handling thousands of concurrent connections
- Minimal latency and resource overhead

✅ **Advanced Load Balancing**
- Round-robin strategy
- Least connections strategy
- Random distribution strategy
- Customizable per deployment

✅ **Distributed Node Management**
- Dynamic node registration and deregistration
- Persistent node registry (JSON-based storage)
- Real-time node availability tracking
- Automatic node health validation

✅ **Health Checking System**
- Startup probes for new nodes
- Liveness probes for continuous monitoring
- Find node probes for discovery
- Configurable check intervals

✅ **Comprehensive Logging**
- Nginx-style access logging format
- Structured error, info, warn, and debug levels
- Automatic log rotation with configurable size limits
- File-based logging with buffering and flushing

✅ **Management API**
- Dedicated management interface (separate port)
- Node registration endpoint
- Node deletion endpoint
- Configuration retrieval
- Metrics reporting

✅ **Request Routing**
- Path-based routing rules
- Host-based routing (extensible)
- Protocol preservation
- Custom header handling

✅ **Worker Node Integration**
- MNode framework for creating worker nodes
- Auto-registration with Melisa core
- Static file serving capability
- Custom backend service support

---

## Architecture Overview

### System Design

Melisa follows a master-worker distributed architecture:

```
┌─────────────────────────────────────────────────────────┐
│                   Client Requests                       │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────────┐
│                   MELISA PROXY CORE                      │
│  ┌────────────────────────────────────────────────────┐  │
│  │  Proxy Server (Main Port)                          │  │
│  │  • Request routing and forwarding                  │  │
│  │  • Load balancing                                  │  │
│  │  • Connection pooling                              │  │
│  │  • Request/Response handling                       │  │
│  └────────────────────────────────────────────────────┘  │
│                                                           │
│  ┌────────────────────────────────────────────────────┐  │
│  │  Management API (Dedicated Port 8888)              │  │
│  │  • Node registration/deregistration                │  │
│  │  • Configuration management                        │  │
│  │  • Metrics collection                              │  │
│  └────────────────────────────────────────────────────┘  │
│                                                           │
│  ┌────────────────────────────────────────────────────┐  │
│  │  Node Manager & Registry                           │  │
│  │  • In-memory node registry                         │  │
│  │  • Health monitoring                               │  │
│  │  • Persistence layer                               │  │
│  └────────────────────────────────────────────────────┘  │
└───────┬──────────────────────────────────────────────────┘
        │
        ├─────────────────┬──────────────────┬─────────────────┐
        │                 │                  │                 │
        ▼                 ▼                  ▼                 ▼
    ┌────────┐       ┌────────┐        ┌────────┐       ┌────────┐
    │ MNode 1│       │ MNode 2│        │ MNode 3│       │ MNode N│
    │ Port   │       │ Port   │        │ Port   │       │ Port   │
    │ 3000   │       │ 3001   │        │ 3002   │       │ 3nnn   │
    │        │       │        │        │        │       │        │
    │Backend │       │Backend │        │Backend │       │Backend │
    │Service │       │Service │        │Service │       │Service │
    └────────┘       └────────┘        └────────┘       └────────┘
```

### Component Structure

#### Melisa Core
The central proxy gateway that:
- Accepts incoming client connections
- Routes requests to appropriate worker nodes based on routing rules
- Implements load balancing algorithms
- Manages the registry of available nodes
- Performs health checks on nodes
- Logs all activities for monitoring and debugging

#### Worker Nodes (MNode)
Individual service instances that:
- Run user-defined backend services
- Auto-register with Melisa core on startup
- Respond to health check probes
- Serve static files (optional)
- Accept forwarded requests from the proxy

#### Management API
A separate interface for:
- Administrative operations
- Node registration/deregistration
- Metrics retrieval
- Configuration updates

---

## System Requirements

### Build Requirements

- **Rust**: 1.70+ (preferably 1.80+)
- **Cargo**: Comes with Rust
- **Operating System**: Linux, macOS, or Windows (with WSL)
- **CPU**: x86_64 architecture recommended
- **RAM**: 512MB minimum (2GB recommended for development)
- **Disk**: 1GB for build artifacts

### Runtime Requirements

- **Operating System**: Linux, macOS, or Windows (with proper socket support)
- **RAM**: 256MB minimum (500MB recommended for production)
- **Network**: TCP/IP stack support
- **File Permissions**: Ability to bind to ports (may require elevated privileges for ports < 1024)

### Dependencies

Melisa uses the following key Rust crates:

```
tokio (1.52.3)          - Async runtime
hyper (1.4.1)           - HTTP protocol implementation
reqwest (0.13.4)        - HTTP client
serde/serde_json        - Serialization
sha2 (0.11.0)          - Cryptographic hashing
chrono (0.4)            - Time and date handling
uuid (1.0)              - Unique identifier generation
toml (1.1.2)            - TOML configuration parsing
```

---

## Installation

### Prerequisites

Ensure Rust is installed on your system:

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### Building from Source

1. **Clone the repository**

```bash
git clone https://github.com/sebastvnd/melisa.git
cd melisa
```

2. **Build the project**

```bash
# Development build (faster compile time)
cargo build

# Release build (optimized binary)
cargo build --release
```

The compiled binary will be available at:
- Debug: `target/debug/melisa_beta`
- Release: `target/release/melisa_beta`

3. **Verify installation**

```bash
./target/release/melisa_beta --version
```

### Docker Setup (Optional)

If you prefer containerized deployment:

```dockerfile
FROM rust:latest

WORKDIR /app
COPY . .

RUN cargo build --release

EXPOSE 8080 8888

CMD ["./target/release/melisa_beta"]
```

Build and run the Docker image:

```bash
docker build -t melisa:latest .
docker run -p 8080:8080 -p 8888:8888 melisa:latest
```

---

## Getting Started

### Quick Start Guide

#### Step 1: Configure Melisa Core

Copy the example configuration:

```bash
cp melisa.conf.example melisa.conf
```

Edit `melisa.conf` with your settings:

```toml
# Listen on localhost:8080 for proxy traffic
host = "127.0.0.1"
port = 8080

# Management API on port 8888
[management]
port = 8888
enabled = true

# Logging configuration
[logging]
log_dir = "./logs"
access_log_enabled = true
error_log_enabled = true
debug_log_enabled = false
level = "info"

# Node management
[nodes]
storage_file = "nodes.json"
health_check_interval_secs = 30

# Proxy behavior
[proxy]
load_balancer_strategy = "round_robin"
request_timeout_secs = 30
max_retries = 3
```

#### Step 2: Create a Worker Node

Set up an MNode worker:

```bash
cd mnode
cp mnode.conf.example mnode.conf
```

Configure `mnode.conf`:

```toml
# MNode configuration
server_port = 3000
route_path = "/api/service1"

[registration]
melisa_host = "127.0.0.1"
melisa_port = 8888

[static_files]
directory = "./public/html"
enabled = true
```

#### Step 3: Start the Services

**Terminal 1 - Melisa Core:**

```bash
./target/release/melisa_beta
```

Expected output:
```
╔════════════════════════════════════════════╗
║                MELISA CORE                 ║
║════════════════════════════════════════════╝
║  melisa version 0.1.0-beta
║  open server architecture
║  Copyright (c) 2026 sebastvn.d
╚════════════════════════════════════════════╝

  Config: melisa.conf
  Listen: http://127.0.0.1:8080
  Node registry: nodes.json (0 node)

Start node manager

...
```

**Terminal 2 - MNode Worker:**

```bash
cd mnode
./target/release/mnode_worker
```

The node will automatically register with Melisa.

#### Step 4: Test the Setup

```bash
# Test Melisa health
curl http://localhost:8080/api/health

# Check registered nodes (via management API)
curl http://localhost:8888/api/nodes

# Access a proxied service
curl http://localhost:8080/api/service1/info
```

---

## Configuration

### Melisa Core Configuration (melisa.conf)

The `melisa.conf` file controls the behavior of the proxy core.

#### Proxy Server Settings

```toml
# The address Melisa listens on for client traffic
host = "127.0.0.1"
port = 8080
```

- `host`: IP address to bind to (use "0.0.0.0" for all interfaces)
- `port`: TCP port number (requires elevated privileges for ports < 1024)

#### Management API Configuration

```toml
[management]
port = 8888
enabled = true
```

- `port`: Dedicated port for management operations
- `enabled`: Set to `false` to disable the management API

#### Logging Configuration

```toml
[logging]
log_dir = "./logs"
access_log_enabled = true
error_log_enabled = true
proxy_log_enabled = true
debug_log_enabled = false

access_log_format = "$remote_addr - - [$time_local] \"$request\" $status $bytes_sent \"$http_referer\" \"$http_user_agent\" $request_time"

max_file_size_mb = 100
max_backups = 10
flush_interval_ms = 10000

level = "info"  # debug, info, warn, error
```

**Log Variables:**
- `$remote_addr`: Client IP address
- `$time_local`: Local timestamp
- `$request`: HTTP request line
- `$status`: HTTP status code
- `$bytes_sent`: Response size in bytes
- `$http_referer`: HTTP Referer header
- `$http_user_agent`: User-Agent header
- `$request_time`: Total request processing time

**Log Levels:**
- `debug`: Detailed diagnostic information
- `info`: General informational messages
- `warn`: Warning messages
- `error`: Error messages only

#### Node Management Configuration

```toml
[nodes]
storage_file = "nodes.json"
flush_threshold_bytes = 51200
health_check_interval_secs = 30
```

- `storage_file`: Where node registry is persisted
- `flush_threshold_bytes`: Size threshold for auto-saving registry (50KB default)
- `health_check_interval_secs`: Frequency of periodic health checks

#### Proxy Behavior Configuration

```toml
[proxy]
load_balancer_strategy = "round_robin"  # round_robin, least_connections, random
request_timeout_secs = 30
max_idle_per_host = 32
max_retries = 3
retry_backoff_ms = 100
metrics_report_interval_secs = 60
```

- `load_balancer_strategy`: How to select which node receives a request
- `request_timeout_secs`: Maximum time to wait for backend response
- `max_idle_per_host`: Connection pool size per backend
- `max_retries`: Number of times to retry failed requests
- `retry_backoff_ms`: Delay between retry attempts
- `metrics_report_interval_secs`: How often to report metrics

### Worker Node Configuration (mnode.conf)

The `mnode.conf` file configures individual worker nodes.

```toml
# Server settings
server_port = 3000
route_path = "/api/service1"

[registration]
melisa_host = "127.0.0.1"
melisa_port = 8888
auto_register = true
register_interval_secs = 5

[static_files]
directory = "./public/html"
enabled = true

[service]
name = "api-service"
version = "1.0.0"
```

---

## Running Melisa

### Development Mode

For development and testing:

```bash
# Build in debug mode
cargo build

# Run with debug logging
RUST_LOG=debug ./target/debug/melisa_beta
```

### Production Mode

For production deployments:

```bash
# Build optimized binary
cargo build --release

# Run in background
nohup ./target/release/melisa_beta > melisa.log 2>&1 &

# Or use systemd (see below)
```

### Systemd Service Setup

Create `/etc/systemd/system/melisa.service`:

```ini
[Unit]
Description=Melisa Proxy Gateway
After=network.target

[Service]
Type=simple
User=melisa
WorkingDirectory=/opt/melisa
ExecStart=/opt/melisa/melisa_beta
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable melisa
sudo systemctl start melisa
sudo systemctl status melisa
sudo journalctl -u melisa -f
```

### Running Multiple Instances

For high availability, run multiple Melisa instances:

```bash
# Instance 1
./target/release/melisa_beta --config melisa-1.conf

# Instance 2
./target/release/melisa_beta --config melisa-2.conf

# Use an external load balancer to distribute traffic
```

---

## Worker Nodes (MNode)

### Overview

MNode is a framework for building worker nodes that integrate with Melisa:

- **Automatic Registration**: Nodes self-register with Melisa on startup
- **Health Reporting**: Respond to health check probes
- **Static File Serving**: Optional static file serving capability
- **Custom Backends**: Easy integration with custom backend services

### MNode Directory Structure

```
mnode/
├── mnode.conf              # Configuration file
├── Cargo.toml              # Rust package manifest
├── public/
│   └── html/               # Static files (HTML, CSS, JS)
│       ├── index.html
│       ├── style.css
│       └── app.js
└── src/
    ├── main.rs             # Entry point
    ├── config.rs           # Configuration loading
    ├── handler.rs          # Request handlers
    └── registration.rs     # Melisa registration logic
```

### MNode Architecture

```
┌─────────────────────────────────────────┐
│          MNode Application               │
├─────────────────────────────────────────┤
│                                         │
│  ┌─────────────────────────────────┐   │
│  │   HTTP Server (e.g., port 3000) │   │
│  └──────────────┬──────────────────┘   │
│                 │                       │
│      ┌──────────┴───────────┐          │
│      │                      │          │
│      ▼                      ▼          │
│  ┌────────┐           ┌──────────┐   │
│  │ Static │           │ API      │   │
│  │ Files  │           │ Handlers │   │
│  └────────┘           └──────────┘   │
│                                       │
│  ┌─────────────────────────────────┐ │
│  │  Auto-Registration & Health      │ │
│  │  - Register with Melisa on start │ │
│  │  - Respond to health probes      │ │
│  │  - Send periodic heartbeats      │ │
│  └─────────────────────────────────┘ │
└─────────────────────────────────────────┘
        │
        │ Registers with
        │
        ▼
┌─────────────────────────────────────────┐
│    Melisa Management API (Port 8888)    │
└─────────────────────────────────────────┘
```

### Building a Custom MNode

1. **Modify `src/handler.rs`** to add your custom endpoints:

```rust
pub async fn handle_request(
    req: Request<Incoming>,
) -> Result<Response<String>, Box<dyn Error>> {
    let path = req.uri().path();
    
    if path == "/api/custom" {
        return Ok(Response::new("Custom endpoint response".to_string()));
    }
    
    // ... other routes
}
```

2. **Update `mnode.conf`** for your service:

```toml
server_port = 3000
route_path = "/api/myservice"

[service]
name = "my-backend-service"
version = "1.0.0"
```

3. **Build and run**:

```bash
cd mnode
cargo build --release
./target/release/mnode
```

### Static File Serving

MNode can serve static files from `public/html/`:

```toml
[static_files]
directory = "./public/html"
enabled = true
```

Place your HTML, CSS, JS files in `public/html/` and they'll be automatically served.

---

## API Documentation

### Proxy API (Main Port)

#### Health Check
```
GET /api/health
```

Response:
```json
{
  "status": "healthy",
  "timestamp": "2026-06-19T10:30:45Z"
}
```

#### Node Information
```
GET /api/info
```

Response:
```json
{
  "name": "melisa-proxy",
  "version": "0.1.0-beta",
  "uptime_seconds": 3600,
  "active_connections": 42
}
```

### Management API (Port 8888)

#### Register Node
```
POST /api/nodes/register
Content-Type: application/json

{
  "name": "mnode-1",
  "host": "127.0.0.1",
  "port": 3000,
  "route_path": "/api/service1",
  "weight": 1
}
```

Response:
```json
{
  "success": true,
  "node_id": "uuid-1234",
  "message": "Node registered successfully"
}
```

#### List Nodes
```
GET /api/nodes
```

Response:
```json
{
  "nodes": [
    {
      "id": "uuid-1234",
      "name": "mnode-1",
      "host": "127.0.0.1",
      "port": 3000,
      "route_path": "/api/service1",
      "status": "healthy",
      "last_check": "2026-06-19T10:30:45Z"
    }
  ],
  "total": 1
}
```

#### Remove Node
```
DELETE /api/nodes/{node_id}
```

Response:
```json
{
  "success": true,
  "message": "Node removed successfully"
}
```

#### Metrics
```
GET /api/metrics
```

Response:
```json
{
  "timestamp": "2026-06-19T10:30:45Z",
  "total_requests": 10000,
  "total_bytes_sent": 524288000,
  "total_bytes_received": 104857600,
  "avg_response_time_ms": 45.3,
  "nodes": {
    "uuid-1234": {
      "requests": 5000,
      "errors": 12,
      "avg_response_time_ms": 42.1
    }
  }
}
```

---

## Load Balancing Strategies

### 1. Round Robin

Distributes requests equally across all healthy nodes in a circular manner.

**Configuration:**
```toml
[proxy]
load_balancer_strategy = "round_robin"
```

**Use Case:** When all nodes have similar capacity and performance characteristics.

**Example Flow:**
```
Request 1 → Node A
Request 2 → Node B
Request 3 → Node C
Request 4 → Node A (cycle repeats)
```

### 2. Least Connections

Routes each request to the node with the fewest active connections.

**Configuration:**
```toml
[proxy]
load_balancer_strategy = "least_connections"
```

**Use Case:** When request duration varies significantly; provides natural load balancing.

**Example Flow:**
```
Node A: 10 connections
Node B: 5 connections
Node C: 8 connections
New Request → Node B (fewest connections)
```

### 3. Random

Randomly selects a node from the available healthy nodes.

**Configuration:**
```toml
[proxy]
load_balancer_strategy = "random"
```

**Use Case:** Simple, minimal overhead; good for debugging.

**Example Flow:**
```
Request 1 → Node C (random)
Request 2 → Node A (random)
Request 3 → Node B (random)
Request 4 → Node C (random)
```

---

## Health Checking

### Overview

Melisa implements comprehensive health checking to ensure only healthy nodes receive traffic:

### Startup Probe

Validates that a node is ready to serve traffic when it first registers:

```
Timeline:
T=0: Node registers with Melisa
T=1: Melisa performs startup probe
T=2-5: Retries if probe fails
T=6+: Node marked as healthy or removed
```

### Liveness Probe

Periodically checks if registered nodes remain healthy:

```
Interval: Configured via health_check_interval_secs (default: 30s)
Method: HTTP GET to /api/health
Timeout: request_timeout_secs (default: 30s)
```

### Health Check Endpoints

**Startup Probe:**
```
GET {node_host}:{node_port}/{route_path}/health/startup
```

**Liveness Probe:**
```
GET {node_host}:{node_port}/{route_path}/health/live
```

**Readiness Probe:**
```
GET {node_host}:{node_port}/{route_path}/health/ready
```

### Configuration

```toml
[nodes]
health_check_interval_secs = 30

[proxy]
request_timeout_secs = 30
max_retries = 3
retry_backoff_ms = 100
```

---

## Logging System

### Log Types

Melisa supports multiple log types for comprehensive visibility:

#### Access Log
Records all HTTP requests/responses:

```
127.0.0.1 - - [2026-06-19 10:30:45] "GET /api/users HTTP/1.1" 200 1024 "-" "Mozilla/5.0" 0.045
```

Fields:
- Client IP
- Request timestamp
- Request line (method, path, protocol)
- Response status code
- Response size
- Referrer
- User-Agent
- Request duration

#### Error Log
Captures errors and critical issues:

```
[2026-06-19 10:30:46] [ERROR] Failed to connect to node: 127.0.0.1:3000 - Connection refused
[2026-06-19 10:30:47] [WARN] Health check failed for node uuid-1234
```

#### Debug Log
Detailed diagnostic information (when enabled):

```
[2026-06-19 10:30:45] [DEBUG] Routing request to node uuid-1234
[2026-06-19 10:30:45] [DEBUG] Response from backend: 200 OK (45ms)
```

#### Proxy Log
Proxy-specific events and metrics:

```
[2026-06-19 10:30:45] [INFO] Node registered: mnode-1 (127.0.0.1:3000)
[2026-06-19 10:30:45] [INFO] Periodic health check completed (3 nodes checked)
```

### Log Rotation

Logs are automatically rotated based on size:

```toml
[logging]
max_file_size_mb = 100      # Rotate when file reaches 100MB
max_backups = 10            # Keep 10 backup files
flush_interval_ms = 10000   # Flush to disk every 10 seconds
```

**Rotation Example:**
```
access.log        (current, < 100MB)
access.log.1      (previous rotation)
access.log.2      (older rotation)
...
access.log.10     (oldest kept)
```

---

## Metrics and Monitoring

### Available Metrics

Melisa tracks comprehensive metrics for monitoring and optimization:

#### Global Metrics
- Total requests processed
- Total bytes sent/received
- Average response time
- Error rate
- Active connections
- Uptime

#### Per-Node Metrics
- Requests routed to node
- Success/error counts
- Average response time
- Connection count
- Last health check time
- Node status

### Retrieving Metrics

Via Management API:

```bash
curl http://localhost:8888/api/metrics
```

Response:
```json
{
  "timestamp": "2026-06-19T10:30:45Z",
  "uptime_seconds": 3600,
  "total_requests": 50000,
  "total_bytes_sent": 2621440000,
  "total_bytes_received": 524288000,
  "avg_response_time_ms": 48.2,
  "error_rate_percent": 0.8,
  "active_connections": 127,
  "nodes": {
    "uuid-1234": {
      "requests": 25000,
      "bytes_sent": 1310720000,
      "bytes_received": 262144000,
      "errors": 150,
      "avg_response_time_ms": 45.1,
      "status": "healthy",
      "last_check": "2026-06-19T10:30:30Z"
    },
    "uuid-5678": {
      "requests": 25000,
      "bytes_sent": 1310720000,
      "bytes_received": 262144000,
      "errors": 250,
      "avg_response_time_ms": 51.3,
      "status": "healthy",
      "last_check": "2026-06-19T10:30:30Z"
    }
  }
}
```

### Monitoring Best Practices

1. **Set up log aggregation** (ELK stack, Loki, etc.) for centralized log analysis
2. **Implement metrics collection** (Prometheus, Grafana) for visualization
3. **Configure alerts** for critical issues:
   - High error rate (> 5%)
   - Node health failures
   - Slow response times (> 5s)
   - Node registry inconsistencies

---

## Development

### Project Structure

```
melisa/
├── src/
│   ├── mcore/
│   │   ├── adapter/              # Protocol adapters
│   │   │   ├── json.rs
│   │   │   └── mod.rs
│   │   ├── api/                  # API definitions
│   │   │   ├── mod.rs
│   │   │   └── services.rs
│   │   ├── config/               # Configuration loading
│   │   │   ├── load_config.rs
│   │   │   └── mod.rs
│   │   ├── errors/               # Error types
│   │   │   ├── econfig.rs
│   │   │   ├── enode.rs
│   │   │   └── mod.rs
│   │   ├── handler/              # Request handlers
│   │   │   ├── handler.rs
│   │   │   └── mod.rs
│   │   ├── melisad/              # Core daemon logic
│   │   │   ├── management/       # Management API
│   │   │   │   ├── mod.rs
│   │   │   │   └── server.rs
│   │   │   ├── probes/           # Health probes
│   │   │   │   ├── find_node.rs
│   │   │   │   ├── liveness_node.rs
│   │   │   │   ├── startup_node.rs
│   │   │   │   └── mod.rs
│   │   │   ├── proxy/            # Proxy logic
│   │   │   │   ├── forwarder.rs
│   │   │   │   ├── handler.rs
│   │   │   │   ├── loadbalancer.rs
│   │   │   │   ├── metrics.rs
│   │   │   │   ├── mod.rs
│   │   │   │   └── server.rs
│   │   │   ├── services/         # Service handlers
│   │   │   │   ├── node/
│   │   │   │   │   ├── manager.rs
│   │   │   │   │   ├── models.rs
│   │   │   │   │   ├── operations.rs
│   │   │   │   │   ├── persistence.rs
│   │   │   │   │   └── mod.rs
│   │   │   │   └── mod.rs
│   │   │   ├── utils/            # Utilities
│   │   │   │   ├── hashing.rs
│   │   │   │   └── mod.rs
│   │   │   └── mod.rs
│   │   ├── mlog/                 # Logging system
│   │   │   ├── log_config.rs
│   │   │   ├── logger.rs
│   │   │   ├── rotation.rs
│   │   │   └── mod.rs
│   │   └── mod.rs
│   └── main.rs                   # Entry point
│
├── mnode/
│   ├── src/
│   │   ├── main.rs
│   │   ├── config.rs
│   │   ├── handler.rs
│   │   └── registration.rs
│   ├── public/html/              # Static files
│   │   ├── index.html
│   │   ├── style.css
│   │   └── app.js
│   ├── mnode.conf.example
│   └── Cargo.toml
│
├── Cargo.toml
├── melisa.conf.example
├── LICENSE
└── README.md
```

### Building

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Watch mode (for development)
cargo watch -x build

# Run tests
cargo test

# Build documentation
cargo doc --open
```

### Code Contributions

When contributing, follow these guidelines:

1. **Code Style**: Use `cargo fmt`
```bash
cargo fmt
```

2. **Linting**: Run `cargo clippy`
```bash
cargo clippy -- -D warnings
```

3. **Testing**: Write tests for new features
```bash
cargo test -- --nocapture
```

4. **Documentation**: Document public APIs
```rust
/// Brief description
///
/// Longer explanation
///
/// # Examples
///
/// ```
/// // example code
/// ```
pub fn function_name() {
    // implementation
}
```

### Testing Strategy

```bash
# Run all tests
cargo test

# Run specific test module
cargo test mcore::

# Run with verbose output
cargo test -- --nocapture

# Run single-threaded (for debugging)
cargo test -- --test-threads=1
```

### Debugging

Enable debug logging:

```bash
RUST_LOG=debug cargo run
```

Use a debugger:

```bash
# With lldb (macOS/Linux)
rust-lldb ./target/debug/melisa_beta

# With gdb (Linux)
gdb ./target/debug/melisa_beta
```

---

## Troubleshooting

### Common Issues and Solutions

#### 1. Port Already in Use

**Problem:** `bind: address already in use`

**Solution:**
```bash
# Find process using the port
lsof -i :8080

# Kill the process
kill -9 <PID>

# Or use a different port in melisa.conf
port = 8081
```

#### 2. Node Registration Fails

**Problem:** Node cannot register with Melisa

**Check:**
```bash
# Verify Melisa is running
curl http://localhost:8080/api/health

# Check management API
curl http://localhost:8888/api/nodes

# Check firewall/network connectivity
nc -zv 127.0.0.1 8888
```

**Solution:**
- Ensure `melisa_host` and `melisa_port` in mnode.conf are correct
- Check firewall rules allow traffic on port 8888
- Verify Melisa management API is enabled in melisa.conf

#### 3. High Memory Usage

**Problem:** Melisa consuming excessive memory

**Check:**
```bash
# Monitor memory usage
watch -n 1 'ps aux | grep melisa_beta'

# Check for memory leaks
valgrind ./target/release/melisa_beta
```

**Solution:**
- Reduce `max_idle_per_host` in proxy configuration
- Check for unbounded connection pools
- Monitor number of registered nodes

#### 4. Slow Response Times

**Problem:** Requests taking too long

**Debug:**
```bash
# Check response times in logs
grep "request_time" ./logs/access.log | tail -20

# Enable debug logging
RUST_LOG=debug ./target/release/melisa_beta

# Check load balancing
curl http://localhost:8888/api/metrics
```

**Solution:**
- Check health of backend nodes
- Verify network connectivity
- Increase `request_timeout_secs` if needed
- Review load balancer distribution

#### 5. Nodes Marked as Unhealthy

**Problem:** All nodes failing health checks

**Debug:**
```bash
# Check node status
curl http://localhost:8888/api/nodes

# Test node directly
curl http://127.0.0.1:3000/api/health

# Check logs
tail -f ./logs/error.log
```

**Solution:**
- Ensure nodes are actually running
- Verify node health endpoints are implemented
- Check network connectivity between Melisa and nodes
- Increase health check timeout if needed

#### 6. File Permission Issues

**Problem:** Cannot write logs or config files

**Solution:**
```bash
# Ensure proper permissions
chmod 755 ./logs
chmod 644 melisa.conf

# Run with proper user
sudo chown -R melisa:melisa /opt/melisa
```

### Getting Help

1. Check the logs: `tail -f ./logs/error.log`
2. Enable debug logging: `RUST_LOG=debug`
3. Review this documentation
4. Check GitHub issues
5. Contact the project maintainer

---

## Performance Tuning

### System Tuning

```bash
# Increase file descriptor limit
ulimit -n 65536

# Increase socket backlog
sysctl -w net.core.somaxconn=4096

# Tune TCP parameters
sysctl -w net.ipv4.tcp_fin_timeout=30
sysctl -w net.ipv4.tcp_tw_reuse=1
```

### Melisa Configuration Optimization

```toml
[proxy]
# Increase connection pool
max_idle_per_host = 128

# Optimize timeouts
request_timeout_secs = 60

# Reduce retry delays
retry_backoff_ms = 50

# Increase metric reporting
metrics_report_interval_secs = 30

[nodes]
# Less frequent health checks in stable environments
health_check_interval_secs = 60

[logging]
# Reduce logging overhead
debug_log_enabled = false
flush_interval_ms = 5000
```

---

## Contributing

### How to Contribute

We welcome contributions! Here are ways you can help:

1. **Report Bugs**: Open an issue with details and steps to reproduce
2. **Suggest Features**: Propose enhancements with use cases
3. **Submit Code**: Fork, create a branch, and submit a pull request
4. **Improve Documentation**: Fix typos or add clarification
5. **Test on Different Systems**: Report compatibility issues

### Development Workflow

```bash
# Fork the repository
git clone https://github.com/your-username/melisa.git

# Create feature branch
git checkout -b feature/amazing-feature

# Make changes and test
cargo test

# Commit with clear messages
git commit -m "Add amazing feature"

# Push to your fork
git push origin feature/amazing-feature

# Create Pull Request on GitHub
```

### Code Review

All contributions go through code review:
- Follow Rust conventions (`cargo fmt`, `cargo clippy`)
- Write tests for new features
- Update documentation
- Ensure CI/CD checks pass

---

## License

Melisa is licensed under the **MIT License**. See the [LICENSE](LICENSE) file for details.

```
MIT License

Copyright (c) 2026 sebastvn.d

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, distribute, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.
```

---

## Roadmap

### Planned Features (Future Versions)

- [ ] **HTTP/2 and HTTP/3 Support**: Modern HTTP protocol versions
- [ ] **WebSocket Proxying**: Full WebSocket support
- [ ] **gRPC Support**: For microservices architectures
- [ ] **Dynamic Routing Rules**: Update routes without restart
- [ ] **TLS/SSL Termination**: HTTPS support
- [ ] **Rate Limiting**: Per-node and per-route rate limits
- [ ] **Circuit Breaker**: Automatic failure handling
- [ ] **Distributed Tracing**: Integration with tracing systems
- [ ] **Caching Layer**: Response caching capabilities
- [ ] **Admin Dashboard**: Web UI for management

### Known Limitations

- Currently supports HTTP/1.1 only
- Single-machine deployment (no multi-machine clustering yet)
- Basic authentication (no advanced auth systems)
- Limited routing capabilities (path-based only)

---

## FAQ

### Q: Is Melisa production-ready?

A: Melisa is currently at version 0.1.0-beta. While it demonstrates solid architectural principles and Rust best practices, it's recommended to thoroughly test in your environment before using in production.

### Q: How does Melisa compare to Nginx?

A: Melisa is inspired by Nginx but is a much simpler, experimental project. Nginx is production-proven with decades of refinement. Melisa demonstrates modern Rust approaches to similar problems.

### Q: Can I use Melisa for SSL/TLS?

A: Current version doesn't support SSL/TLS directly. Consider putting Melisa behind a reverse proxy like Nginx for SSL termination.

### Q: What's the maximum number of nodes?

A: Theoretically unlimited, but practically limited by:
- Available memory (each node entry takes ~500 bytes)
- Health check overhead (increases linearly with nodes)
- Message queue sizes

### Q: How do I monitor Melisa?

A: Three approaches:
1. Check logs in `./logs/` directory
2. Query metrics API: `curl http://localhost:8888/api/metrics`
3. Integrate with monitoring systems (Prometheus, Grafana, etc.)

### Q: Can I use environment variables for configuration?

A: Currently, configuration is TOML-based. Environment variable support can be added as a feature.

---

## Support and Contact

- **Issues**: GitHub Issues for bug reports and features
- **Discussions**: GitHub Discussions for questions and ideas
- **Author**: sebastvn.d
- **Year**: 2026

---

## Acknowledgments

### Inspiration

Melisa draws inspiration from:
- **Pingora**: Cloudflare's modern proxy written in Rust
- **Nginx**: The industry-standard reverse proxy
- **Tokio**: Rust's powerful async runtime

### Technologies Used

- **Rust**: Systems programming language
- **Tokio**: Async runtime
- **Hyper**: HTTP protocol library
- **Serde**: Serialization framework
- **TOML**: Configuration format

---

## Version History

### 0.1.0-beta (Current)
- Initial release
- Basic proxy functionality
- Node management
- Health checking
- Load balancing (3 strategies)
- Logging system
- Management API
- MNode worker framework

### Future Versions
- 0.2.0: HTTP/2 support, improved routing
- 0.3.0: TLS/SSL support, caching
- 0.4.0: gRPC support, advanced auth
- 1.0.0: Production-ready release

---

<div align="center">

**Made with ❤️ by sebastvn.d**

[GitHub](https://github.com/sebastvnd/melisa) • [Issues](https://github.com/sebastvnd/melisa/issues) • [Discussions](https://github.com/sebastvnd/melisa/discussions)

---

*Last Updated: June 2026*

</div>