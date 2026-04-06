#!/bin/bash
set -e
echo "Building procgeo TypeScript bindings..."
cd "$(dirname "$0")"
npm install
npx napi build --platform --release
echo "Build complete! The .node binary and index.d.ts are ready."
echo "Usage: const procgeo = require('./procgeo.node')"
