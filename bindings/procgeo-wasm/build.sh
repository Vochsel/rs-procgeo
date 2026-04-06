#!/bin/bash
set -e
echo "Building procgeo WASM bindings..."
cd "$(dirname "$0")"

# Check for wasm-pack
if ! command -v wasm-pack &> /dev/null; then
    echo "Installing wasm-pack..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

wasm-pack build --target web --out-dir pkg --release
echo ""
echo "Build complete! Output in pkg/"
echo ""
echo "Usage in HTML:"
echo '  import init, { createBox, computeNormals } from "./pkg/procgeo_wasm.js";'
echo '  await init();'
echo '  const box = createBox({ size: [1, 1, 1] });'
