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

# Link WASM build
mkdir -p public/wasm
WASM_PKG="../bindings/procgeo-wasm/pkg"
if [ -d "$WASM_PKG" ]; then
    cp "$WASM_PKG/procgeo_wasm.js" public/wasm/
    cp "$WASM_PKG/procgeo_wasm_bg.wasm" public/wasm/
    cp "$WASM_PKG/procgeo_wasm.d.ts" public/wasm/
    echo "WASM files copied to public/wasm/"
else
    echo "WASM package not found. Run: cd ../bindings/procgeo-wasm && ./build.sh"
    exit 1
fi

echo ""
echo "Ready! Run: pnpm dev"
