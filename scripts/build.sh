#!/bin/bash
set -e

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

step() { echo -e "\n${BLUE}==> $1${NC}"; }
ok()   { echo -e "${GREEN}    ✓ $1${NC}"; }
warn() { echo -e "${YELLOW}    ! $1${NC}"; }

echo -e "${BLUE}"
echo "  ┌─────────────────────────────────┐"
echo "  │  ProcGeo — Full Build Pipeline  │"
echo "  └─────────────────────────────────┘"
echo -e "${NC}"

# ── 1. Rust core ──────────────────────────────────────────
step "Building Rust workspace (release)"
cargo build --release
ok "procgeo-core, procgeo-sops, procgeo-io, procgeo"

# ── 2. Tests ──────────────────────────────────────────────
step "Running tests"
cargo test --workspace
ok "All tests passed"

# ── 3. Node.js native binding (napi-rs) ──────────────────
step "Building Node.js binding (napi-rs)"
cd "$ROOT/bindings/procgeo-node"
if command -v pnpm &> /dev/null; then
    pnpm install --silent 2>/dev/null || true
    pnpm run build
elif command -v bun &> /dev/null; then
    bun install --silent 2>/dev/null || true
    bunx @napi-rs/cli build --platform --release
else
    warn "Skipped — pnpm or bun required"
fi
ok "bindings/procgeo-node/"
cd "$ROOT"

# ── 4. WASM binding (wasm-pack) ──────────────────────────
step "Building WASM binding (wasm-pack)"
if command -v wasm-pack &> /dev/null; then
    wasm-pack build bindings/procgeo-wasm --target web --out-dir pkg --release
    ok "bindings/procgeo-wasm/pkg/"
else
    warn "wasm-pack not found — install: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    warn "Skipped WASM build"
fi

# ── 5. Python binding (PyO3 + maturin) ───────────────────
step "Building Python binding (PyO3)"
if command -v maturin &> /dev/null; then
    cd "$ROOT/bindings/procgeo-py"
    maturin develop --release
    ok "procgeo Python module installed"
    cd "$ROOT"
elif command -v uv &> /dev/null; then
    warn "maturin not found — install: uv tool install maturin"
    warn "Skipped Python build"
else
    warn "maturin not found — install: pip install maturin"
    warn "Skipped Python build"
fi

# ── Summary ───────────────────────────────────────────────
echo ""
echo -e "${GREEN}Build complete!${NC}"
echo ""
echo "  Rust:   cargo test --workspace"
echo "  Node:   node bindings/procgeo-node/examples/basic.js"
echo "  Python: python bindings/procgeo-py/examples/basic.py"
echo "  WASM:   python3 -m http.server 8080 -d bindings/procgeo-wasm"
echo "          → http://localhost:8080/examples/index.html"
echo ""
