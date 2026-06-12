#!/bin/bash
# Quick Start Script untuk Melisa dengan MNode

PROJECT_DIR="/Users/saferoom/Documents/RUST/melisa_beta"
MELISA_BIN="$PROJECT_DIR/target/debug/melisa_beta"
MNODE_BIN="$PROJECT_DIR/mnode/target/debug/mnode"

echo "╔════════════════════════════════════════════════════╗"
echo "║         MELISA + MNODE QUICK START                ║"
echo "╚════════════════════════════════════════════════════╝"
echo ""

# Check if binaries exist
if [ ! -f "$MELISA_BIN" ]; then
    echo "Building melisa_beta..."
    cd "$PROJECT_DIR"
    cargo build
fi

if [ ! -f "$MNODE_BIN" ]; then
    echo "Building mnode..."
    cd "$PROJECT_DIR/mnode"
    cargo build
fi

echo ""
echo "✓ Binaries ready"
echo ""
echo "To start the services, run in separate terminals:"
echo ""
echo "Terminal 1 - Start Melisa Daemon:"
echo "  cd $PROJECT_DIR"
echo "  cargo run --bin melisa_beta"
echo ""
echo "Terminal 2 - Start MNode (auto-registers):"
echo "  cd $PROJECT_DIR/mnode"
echo "  cargo run --bin mnode"
echo ""
echo "Terminal 3 - Test Management API:"
echo "  curl http://localhost:8888/nodes"
echo ""
echo "Terminal 4 - Test MNode:"
echo "  curl http://localhost:3000/"
echo "  curl http://localhost:3000/api/info"
echo "  curl http://localhost:3000/api/health"
echo ""
echo "For more information, see MNODE_SETUP.md"
