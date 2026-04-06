#!/bin/bash
set -e
cd "$(dirname "$0")"

echo "Setting up ProcGeo Playground..."

# Install dependencies
if command -v pnpm &> /dev/null; then
    pnpm install
elif command -v bun &> /dev/null; then
    bun install
else
    npm install
fi

# Copy WASM build into web/wasm/ (source dir, not public — Vite imports it as a module)
mkdir -p wasm
WASM_PKG="../bindings/procgeo-wasm/pkg"
if [ -d "$WASM_PKG" ]; then
    cp "$WASM_PKG/procgeo_wasm.js" wasm/
    cp "$WASM_PKG/procgeo_wasm_bg.wasm" wasm/
    cp "$WASM_PKG/procgeo_wasm.d.ts" wasm/
    cp "$WASM_PKG/procgeo_wasm_bg.wasm.d.ts" wasm/ 2>/dev/null || true
    echo "WASM files copied to wasm/"
else
    echo "WASM package not found. Run: cd ../bindings/procgeo-wasm && ./build.sh"
    exit 1
fi

echo ""
echo "Ready! Run: pnpm dev"
