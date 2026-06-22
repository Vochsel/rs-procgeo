#!/bin/bash
set -e
cd "$(dirname "$0")/.."

G='\033[0;32m' B='\033[0;34m' Y='\033[1;33m' N='\033[0m'
step() { echo -e "\n${B}==> $1${N}"; }
ok()   { echo -e "${G}  ✓ $1${N}"; }
warn() { echo -e "${Y}  ! $1${N}"; }

# Prefer rustup toolchain for wasm-pack compatibility
[[ -d "$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin" ]] && \
  export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"

echo -e "${B}ProcGeo — Build All${N}"

# ── Rust ──
step "Rust workspace"
cargo build --release --workspace --exclude procgeo-py
cargo test --workspace --exclude procgeo-py
ok "Built + tested"

# ── WASM ──
step "WASM binding"
if command -v wasm-pack &>/dev/null; then
  wasm-pack build bindings/procgeo-wasm --target web --out-dir pkg --release
  cp bindings/procgeo-wasm/pkg/procgeo_wasm.js apps/web/wasm/
  cp bindings/procgeo-wasm/pkg/procgeo_wasm_bg.wasm apps/web/wasm/
  cp bindings/procgeo-wasm/pkg/procgeo_wasm.d.ts apps/web/wasm/
  node scripts/validate-web-editor-types.mjs
  ok "procgeo-wasm → apps/web/wasm/"
else
  warn "wasm-pack not found — install: cargo install wasm-pack"
fi

# ── Python (PyO3) ──
step "Python binding"
if command -v maturin &>/dev/null; then
  cd bindings/procgeo-py && maturin develop --release && cd ../..
  ok "procgeo-py"
else
  warn "maturin not found — install: uv tool install maturin"
fi

# ── Done ──
echo -e "\n${G}Done!${N} Rust + bindings built."
echo "  pnpm dev:web  → playground on localhost"
