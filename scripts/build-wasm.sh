#!/bin/bash
set -e
cd "$(dirname "$0")/.."

# Use rustup toolchain (Homebrew rustc doesn't have wasm32 target)
if [[ -d "$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin" ]]; then
  export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
elif [[ -d "$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin" ]]; then
  export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
fi

wasm-pack build bindings/procgeo-wasm --target web --out-dir pkg --release
cp bindings/procgeo-wasm/pkg/procgeo_wasm.js web/wasm/
cp bindings/procgeo-wasm/pkg/procgeo_wasm_bg.wasm web/wasm/
cp bindings/procgeo-wasm/pkg/procgeo_wasm.d.ts web/wasm/
node scripts/validate-web-editor-types.mjs
echo "WASM built → web/wasm/"
