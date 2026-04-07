#!/bin/bash
set -e

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

step() { echo -e "\n${BLUE}==> $1${NC}"; }
ok()   { echo -e "${GREEN}    ✓ $1${NC}"; }

VERSION="${1:-0.1.0}"

step "Building WASM (release)"
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
wasm-pack build --target web --out-dir pkg --release --scope vochsel
ok "WASM built"

step "Configuring package as @vochsel/procgeo-js@${VERSION}"
cat > pkg/package.json << EOF
{
  "name": "@vochsel/procgeo-js",
  "version": "${VERSION}",
  "description": "Procedural geometry library inspired by Houdini SOPs — works in browser and Node.js",
  "type": "module",
  "main": "procgeo_wasm.js",
  "types": "procgeo_wasm.d.ts",
  "files": [
    "procgeo_wasm_bg.wasm",
    "procgeo_wasm.js",
    "procgeo_wasm.d.ts",
    "procgeo_wasm_bg.wasm.d.ts",
    "three.js",
    "three.d.ts"
  ],
  "exports": {
    ".": {
      "import": "./procgeo_wasm.js",
      "types": "./procgeo_wasm.d.ts"
    },
    "./three": {
      "import": "./three.js",
      "types": "./three.d.ts"
    }
  },
  "keywords": ["procedural", "geometry", "houdini", "sop", "wasm", "3d", "mesh", "threejs"],
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/vochsel/rs-procgeo"
  },
  "peerDependencies": {
    "three": ">=0.150.0"
  },
  "peerDependenciesMeta": {
    "three": { "optional": true }
  },
  "sideEffects": false
}
EOF
ok "package.json configured"

step "Copying Three.js bridge into package"
cp "$ROOT/../../utils/procgeo-three/index.js" pkg/three.js
cp "$ROOT/../../utils/procgeo-three/index.d.ts" pkg/three.d.ts
ok "Three.js bridge included"

step "Adding README"
cat > pkg/README.md << 'READMEEOF'
# @vochsel/procgeo-js

Procedural geometry library inspired by Houdini SOPs. Runs in the browser and Node.js via WebAssembly.

## Install

```bash
npm install @vochsel/procgeo-js
# or
pnpm add @vochsel/procgeo-js
```

## Usage

```js
import init, { createBox, subdivide, computeNormals, smooth } from '@vochsel/procgeo-js';

await init();

const box = createBox({ size: [2, 2, 2] });
const subdiv = subdivide(box, { depth: 2, mode: 'catmullClark' });
const smoothed = smooth(subdiv, { iterations: 3, strength: 0.5 });
const geo = computeNormals(smoothed);

console.log(`${geo.numPoints} points, ${geo.numPrims} prims`);

// Export
const obj = geo.toObj();   // OBJ string
const glb = geo.toGlb();   // GLB Uint8Array
```

## Three.js Integration

```js
import init, { createTorus, subdivide, computeNormals } from '@vochsel/procgeo-js';
import { toMesh, toWireframe, toPointCloud } from '@vochsel/procgeo-js/three';

await init();

const geo = computeNormals(subdivide(createTorus(), { depth: 2, mode: 'catmullClark' }));
scene.add(toMesh(geo));
```

## Available SOPs

**Creation:** `createBox`, `createGrid`, `createSphere`, `createLine`, `createCircle`, `createTube`, `createTorus`

**Manipulation:** `transform`, `computeNormals`, `subdivide` (linear + Catmull-Clark), `smooth`, `polyExtrude`, `clip`, `reverse`, `scatter`, `copyToPoints`, `fuse`, `color`, `voronoiFracture`

**Geometry Methods:** `getPositions()`, `getTriangleIndices()`, `getNormals()`, `getColors()`, `toObj()`, `toGlb()`, `boundingBox()`, `numPoints`, `numPrims`

**Three.js Bridge:** `toBufferGeometry`, `toMesh`, `toWireframe`, `toPointCloud`, `toEdges`, `createScene`
READMEEOF
ok "README added"

echo ""
echo -e "${GREEN}Package ready at pkg/${NC}"
echo ""
echo "  To publish:"
echo "    cd pkg && npm publish --access public"
echo ""
echo "  To dry-run:"
echo "    cd pkg && npm publish --access public --dry-run"
echo ""
