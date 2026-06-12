# MNode - Worker Node Documentation

## 🎯 MNode vs Management API

### Management API (Port 8888)
- **Bukan** untuk user-facing traffic
- **Khusus** untuk node registration/management
- Endpoints: `/register`, `/unregister`, `/nodes`
- Diakses **hanya oleh Melisa daemon** dan node workers

### MNode (Port 3000+)
- **Worker node** yang menjalankan aplikasi Anda
- **Melayani** user requests melalui proxy
- Serve **static files** (HTML, CSS, JS)
- Provide **API endpoints** untuk backend logic
- Auto-register ke Management API saat startup

## 📁 Directory Structure

```
mnode/
├── mnode.conf              ← ⭐ Configuration file (PENTING)
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── main.rs            ← Entry point
│   ├── config.rs          ← Config parser (reads mnode.conf)
│   ├── handler.rs         ← HTTP request handler (serves files)
│   ├── registration.rs    ← Auto-register logic
│   └── ...
└── public/
    └── html/              ← ⭐ Your static files go here!
        ├── index.html     ← Homepage
        ├── style.css      ← Stylesheets
        ├── app.js         ← JavaScript
        └── ...            ← Other files
```

## ⚙️ Configuration (mnode.conf)

```toml
# Server Configuration
host = "127.0.0.1"
port = 3000

# [registration] - Auto-registration ke Melisa Management API
[registration]
melisa_host = "127.0.0.1"
melisa_port = 8888
node_name = "mnode-service"
node_domain = "mnode.local"
node_route_path = "/mnode"

# [static_files] - Static file serving configuration
[static_files]
directory = "./public/html"    ← Directory lokasi HTML files
enabled = true                 ← Enable/disable file serving

# [api] - Backend API Configuration
[api]
enabled = true
base_path = "/api"
```

### Configuration Precedence
1. **File first**: Baca dari `mnode.conf` jika ada
2. **Environment**: Fallback ke environment variables jika file tidak ada
3. **Hardcoded defaults**: Final fallback ke default values

### Environment Variables (jika mnode.conf tidak ada)
```bash
MNODE_PORT              # Port (default: 3000)
MELISA_HOST             # Melisa host (default: 127.0.0.1)
MELISA_PORT             # Melisa port (default: 8888)
MNODE_DOMAIN            # Domain (default: mnode.local)
MNODE_ROUTE_PATH        # Route path (default: /mnode)
STATIC_FILES_DIR        # Static files dir (default: ./public/html)
```

## 📝 How to Use Custom HTML

### Step 1: Create/Edit HTML Files
Place your files di `public/html/`:

```
public/html/
├── index.html           ← Served at / (homepage)
├── dashboard.html       ← Served at /dashboard.html
├── style.css            ← Served at /style.css
├── app.js               ← Served at /app.js
└── images/
    └── logo.png         ← Served at /images/logo.png
```

### Step 2: Update mnode.conf (optional)
```toml
[static_files]
directory = "./public/html"    ← Change if needed
enabled = true
```

### Step 3: Run MNode
```bash
cd mnode
cargo run --bin mnode
```

### Step 4: Access Your Files
- Homepage: `http://localhost:3000/`
- Files: `http://localhost:3000/{filename}`
- CSS: `http://localhost:3000/style.css`
- Images: `http://localhost:3000/images/logo.png`

## 🔄 Request Flow

```
User Browser (Port 8080 - Melisa Proxy)
    ↓
Proxy routes based on domain/path
    ↓
MNode (Port 3000)
    ↓
Handler checks:
  1. Is it a static file? (public/html/)
     → Serve file dengan MIME type
  2. Is it an API endpoint? (/api/*)
     → Return JSON response
  3. Is it root (/)?
     → Serve index.html atau default page
  4. Else → 404
```

## 🎨 Example: Creating Custom Dashboard

### 1. Create index.html
```html
<!DOCTYPE html>
<html>
<head>
    <title>My Dashboard</title>
    <link rel="stylesheet" href="style.css">
</head>
<body>
    <h1>My Custom Dashboard</h1>
    <div id="info"></div>
    <script src="app.js"></script>
</body>
</html>
```

### 2. Create style.css
```css
body {
    font-family: Arial, sans-serif;
    max-width: 1000px;
    margin: 0 auto;
    padding: 20px;
}

h1 {
    color: #667eea;
}
```

### 3. Create app.js
```javascript
// Fetch node info from API
fetch('/api/info')
    .then(r => r.json())
    .then(data => {
        document.getElementById('info').innerHTML = 
            `<p>Node: ${data.url}</p>`;
    });
```

### 4. Run and Test
```bash
cargo run --bin mnode
curl http://localhost:3000/
```

## 🔌 API Endpoints

### GET /
Returns: index.html atau default HTML page

### GET /api/info
Returns:
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
Returns:
```json
{
  "status": "healthy",
  "timestamp": "2024-06-12T10:30:45Z"
}
```

### GET /{path}
Serve file dari public/html/{path}
- Content-Type auto-detected (html, css, js, png, jpg, etc.)
- Cache-Control: public, max-age=3600 (1 hour)

## 🔒 Security Features

1. **Directory Traversal Protection**
   - Paths containing `..` atau `//` are blocked
   - Can only access files in `public/html` directory

2. **File Type Restriction**
   - Only regular files served (not directories)
   - MIME type validation

3. **Path Normalization**
   - Leading slashes removed
   - Invalid characters filtered

## 🚀 Complete Setup Example

### Terminal 1 - Melisa Daemon
```bash
cd /Users/saferoom/Documents/RUST/melisa_beta
cargo run --bin melisa_beta
```
Output:
```
Management API listening on http://127.0.0.1:8888
Melisa proxy listening on http://127.0.0.1:8080
```

### Terminal 2 - MNode Worker
```bash
cd /Users/saferoom/Documents/RUST/melisa_beta/mnode
cargo run --bin mnode
```
Output:
```
✓ Config loaded from mnode.conf
✓ Successfully registered with Melisa
MNode Server Ready - http://127.0.0.1:3000
```

### Terminal 3 - Test
```bash
# Homepage
curl http://localhost:3000/

# API
curl http://localhost:3000/api/info
curl http://localhost:3000/api/health

# Verify registered
curl http://localhost:8888/nodes
```

## 🎯 Common Tasks

### Add a New HTML Page
1. Create `public/html/dashboard.html`
2. Access via `http://localhost:3000/dashboard.html`

### Add CSS/JS Files
1. Create `public/html/style.css`
2. Create `public/html/app.js`
3. Reference in HTML: `<link rel="stylesheet" href="style.css">`

### Add Images
1. Create `public/html/images/logo.png`
2. Reference in HTML: `<img src="images/logo.png">`

### Change Port
Edit `mnode.conf`:
```toml
port = 3001  # Change from 3000 to 3001
```

### Change Static Files Directory
Edit `mnode.conf`:
```toml
[static_files]
directory = "./assets"  # Change from ./public/html
```

## ⚠️ Important Notes

1. **mnode.conf location**: Must be in mnode root directory
2. **File paths**: Relative to mnode working directory
3. **Auto-reload**: No auto-reload. Restart mnode to apply config changes
4. **MIME types**: Auto-detected by file extension
5. **Performance**: Files are read from disk per request (no caching in code)

## 🐛 Troubleshooting

### "Failed to register with Melisa"
- Check: Melisa daemon sudah running di port 8888?
- Check: MELISA_HOST dan MELISA_PORT benar?

### Files not serving (404)
- Check: File ada di `public/html/`?
- Check: Path relatif benar?
- Check: `static_files_enabled = true` di mnode.conf?

### Wrong MIME type
- MIME type auto-detected dari extension
- Custom MIME types bisa ditambah di handler.rs

### Performance issues
- Consider: Add caching layer (nginx, Redis)
- Consider: Move large files ke CDN
