#!/usr/bin/env bash
# install.sh — build & install (or update) the `ffs` binary.
#
# First run:   installs `ffs` onto your system.
# Later runs:  rebuilds from the current source and overwrites the old
#              binary — i.e. running this script again *is* how you update.
#
# Usage:
#   ./install.sh              # install to ~/.local/bin (no sudo needed)
#   ./install.sh --system     # install to /usr/local/bin (uses sudo)
#   ./install.sh --no-pull    # skip `git pull` even if this is a git repo
#
set -euo pipefail

BIN_NAME="ffs"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SYSTEM_INSTALL=false
DO_PULL=true

for arg in "$@"; do
  case "$arg" in
    --system) SYSTEM_INSTALL=true ;;
    --no-pull) DO_PULL=false ;;
    -h|--help)
      echo "Usage: $0 [--system] [--no-pull]"
      exit 0
      ;;
    *)
      echo "Unknown option: $arg" >&2
      exit 1
      ;;
  esac
done

cd "$SCRIPT_DIR"

# --- 0. Make sure cargo/rustc exist; offer to install rustup if missing ---
if ! command -v cargo >/dev/null 2>&1; then
  echo "==> Rust toolchain not found."
  read -r -p "Install Rust now via rustup? [Y/n] " reply
  reply=${reply:-Y}
  if [[ "$reply" =~ ^[Yy]$ ]]; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck disable=SC1090
    source "$HOME/.cargo/env"
  else
    echo "Aborting: cargo is required to build ${BIN_NAME}." >&2
    exit 1
  fi
fi

# --- 1. Pull latest source if this is a git checkout (update path) ---
if [[ "$DO_PULL" == true ]] && [[ -d .git ]]; then
  echo "==> Pulling latest source..."
  git pull --ff-only || echo "    (git pull failed or not needed, continuing with local source)"
fi

# --- 2. Build ---
echo "==> Building release binary..."
cargo build --release

BUILT_BIN="target/release/${BIN_NAME}"
if [[ ! -f "$BUILT_BIN" ]]; then
  echo "error: expected binary not found at $BUILT_BIN" >&2
  echo "       check that [[bin]] name in Cargo.toml matches \"${BIN_NAME}\"" >&2
  exit 1
fi

# --- 3. Install ---
if [[ "$SYSTEM_INSTALL" == true ]]; then
  INSTALL_DIR="/usr/local/bin"
  echo "==> Installing system-wide to $INSTALL_DIR (may ask for sudo password)..."
  sudo install -m 755 "$BUILT_BIN" "$INSTALL_DIR/$BIN_NAME"
else
  INSTALL_DIR="$HOME/.local/bin"
  mkdir -p "$INSTALL_DIR"
  echo "==> Installing for current user to $INSTALL_DIR..."
  install -m 755 "$BUILT_BIN" "$INSTALL_DIR/$BIN_NAME"
fi

echo "==> Installed: $INSTALL_DIR/$BIN_NAME"

# --- 4. PATH check ---
if ! command -v "$BIN_NAME" >/dev/null 2>&1; then
  echo
  echo "NOTE: '$INSTALL_DIR' is not on your PATH yet."
  SHELL_RC="$HOME/.bashrc"
  case "$(basename "${SHELL:-bash}")" in
    zsh) SHELL_RC="$HOME/.zshrc" ;;
    fish) SHELL_RC="$HOME/.config/fish/config.fish" ;;
  esac
  echo "  Add this line to $SHELL_RC, then restart your terminal:"
  echo "    export PATH=\"$INSTALL_DIR:\$PATH\""
fi

echo
echo "==> Done. Version installed:"
"$INSTALL_DIR/$BIN_NAME" --version || true
