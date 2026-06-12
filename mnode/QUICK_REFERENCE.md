# MNode Quick Reference

## 📍 File Locations

| Item | Location |
|------|----------|
| **Configuration** | `mnode/mnode.conf` |
| **Homepage** | `mnode/public/html/index.html` |
| **CSS Files** | `mnode/public/html/*.css` |
| **JavaScript** | `mnode/public/html/*.js` |
| **Images/Assets** | `mnode/public/html/images/` |
| **Source Code** | `mnode/src/` |

## 🔧 mnode.conf Template

```toml
host = "127.0.0.1"
port = 3000

[registration]
melisa_host = "127.0.0.1"
melisa_port = 8888
node_name = "mnode-service"
node_domain = "mnode.local"
node_route_path = "/mnode"

[static_files]
directory = "./public/html"
enabled = true

[api]
enabled = true
base_path = "/api"
```

## 📋 How MNode Works

```
1. Load mnode.conf (atau fallback ke env vars)
2. Start HTTP server di configured port
3. Auto-register dengan Melisa Management API
4. Listen for requests:
   - Static files → Serve dari public/html/
   - /api/info → Return JSON
   - /api/health → Return health status
   - / → Serve index.html atau default page
```

## ✅ Adding Custom HTML

### Option 1: Replace Default
```bash
# Edit existing
nano mnode/public/html/index.html

# Add supporting files
mnode/public/html/
├── index.html
├── style.css
└── app.js
```

### Option 2: Add New Pages
```bash
# Create new page
echo "<h1>Dashboard</h1>" > mnode/public/html/dashboard.html

# Access
http://localhost:3000/dashboard.html
```

### Option 3: Change Directory
```toml
# mnode.conf
[static_files]
directory = "./my-assets"  # New directory
```

## 🚀 Quick Start

```bash
# 1. Terminal 1 - Start Melisa
cd melisa_beta && cargo run --bin melisa_beta

# 2. Terminal 2 - Start MNode (auto-registers)
cd mnode && cargo run --bin mnode

# 3. Terminal 3 - Test
curl http://localhost:3000/                    # Home
curl http://localhost:3000/api/info            # Info
curl http://localhost:3000/api/health          # Health
curl http://localhost:8888/nodes               # List nodes
```

## 📱 API Response Examples

### GET /api/info
```json
{
  "status": "active",
  "url": "http://127.0.0.1:3000",
  "domain": "mnode.local",
  "route_path": "/mnode",
  "pid": 12345,
  "static_files_enabled": true,
  "static_files_dir": "./public/html",
  "timestamp": "2024-06-12T10:30:45Z"
}
```

### GET /api/health
```json
{
  "status": "healthy",
  "timestamp": "2024-06-12T10:30:45Z"
}
```

## 🔗 URL Examples

| Path | Serves From | Type |
|------|-------------|------|
| `/` | `public/html/index.html` | HTML |
| `/dashboard.html` | `public/html/dashboard.html` | HTML |
| `/style.css` | `public/html/style.css` | CSS |
| `/app.js` | `public/html/app.js` | JS |
| `/images/logo.png` | `public/html/images/logo.png` | Image |
| `/api/info` | Generated | JSON |
| `/api/health` | Generated | JSON |

## 🔄 File Serving Flow

```
Browser Request: GET /dashboard.html
                 ↓
MNode Handler checks:
  1. Is static file? → YES
  2. File exists? → Check public/html/dashboard.html
  3. Security check → No traversal, is file
  4. Serve file → HTTP 200 + HTML content
```

## ⚙️ Environment Variables (if no mnode.conf)

```bash
export MNODE_PORT=3001
export MELISA_HOST=127.0.0.1
export MELISA_PORT=8888
export MNODE_DOMAIN=api.local
export MNODE_ROUTE_PATH=/api
export STATIC_FILES_DIR=./public/html

cargo run --bin mnode
```

## 🛡️ Security

- ✅ Directory traversal blocked (`..` paths)
- ✅ Only files served (not directories)
- ✅ MIME type validation
- ✅ Path normalization

## 📊 MNode vs Other Components

| Component | Port | Purpose | Type |
|-----------|------|---------|------|
| **Melisa Proxy** | 8080 | Main HTTP proxy | Daemon |
| **Management API** | 8888 | Node management | API |
| **MNode** | 3000+ | Worker service | Worker |

## 🆘 Common Issues

| Problem | Solution |
|---------|----------|
| Files not found | Check path: `public/html/{file}` |
| 404 on root | Add `index.html` to `public/html/` |
| Can't register | Check Melisa running on 8888 |
| Wrong MIME type | Check file extension |
| Port in use | Change `port` in mnode.conf |

## 📚 More Info

- Full guide: `mnode/MNODE_GUIDE.md`
- Setup docs: `/MNODE_SETUP.md`
- Melisa docs: `/../MNODE_SETUP.md`
