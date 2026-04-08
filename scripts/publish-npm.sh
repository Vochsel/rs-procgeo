#!/bin/bash
set -e

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

step() { echo -e "\n${BLUE}==> $1${NC}"; }
ok()   { echo -e "${GREEN}    + $1${NC}"; }
err()  { echo -e "${RED}    ! $1${NC}"; }

VERSION="${1:-0.1.0}"
PUBLISH_FLAG="--dry-run"
MODE="dry-run"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -p) PUBLISH_FLAG=""; MODE="LIVE PUBLISH"; shift ;;
    *) shift ;;
  esac
done

echo -e "${BLUE}Publishing @procgeo/lib and @procgeo/three @ ${VERSION}${NC}"
echo -e "Mode: ${MODE}\n"

# ─── Build WASM ──────────────────────────────────────────────────────────────

step "Building WASM (release)"
if [[ -d "$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin" ]]; then
  export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
elif [[ -d "$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin" ]]; then
  export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
fi
wasm-pack build bindings/procgeo-wasm --target web --out-dir pkg --release
ok "WASM built"

# ─── @procgeo/lib ────────────────────────────────────────────────────────────

step "Staging @procgeo/lib@${VERSION}"
PKG_LIB="bindings/procgeo-wasm/pkg"

cat > "$PKG_LIB/package.json" << EOF
{
  "name": "@procgeo/lib",
  "version": "${VERSION}",
  "description": "Procedural geometry library inspired by Houdini SOPs -- WASM bindings with full TypeScript types",
  "type": "module",
  "main": "procgeo_wasm.js",
  "types": "procgeo_wasm.d.ts",
  "files": [
    "procgeo_wasm_bg.wasm",
    "procgeo_wasm.js",
    "procgeo_wasm.d.ts",
    "procgeo_wasm_bg.wasm.d.ts",
    "types.d.ts"
  ],
  "exports": {
    ".": {
      "import": "./procgeo_wasm.js",
      "types": "./procgeo_wasm.d.ts"
    },
    "./types": {
      "types": "./types.d.ts"
    }
  },
  "keywords": ["procedural", "geometry", "houdini", "sop", "wasm", "3d", "mesh"],
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/vochsel/rs-procgeo"
  },
  "sideEffects": false
}
EOF
ok "package.json"

# Copy editor types (hand-written, used for Monaco autocomplete)
cp "$ROOT/web/src/procgeo-editor-types.d.ts" "$PKG_LIB/types.d.ts"
ok "types.d.ts (Monaco autocomplete)"

cat > "$PKG_LIB/README.md" << 'READMEEOF'
# @procgeo/lib

Procedural geometry library inspired by Houdini SOPs. Runs in the browser and Node.js via WebAssembly.

## Install

```bash
npm install @procgeo/lib
```

## Usage

```js
import init, { createBox, subdivide, computeNormals, smooth } from '@procgeo/lib';

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

## Monaco / Editor Autocomplete

Import the bundled type definitions for rich editor support:

```ts
import types from '@procgeo/lib/types';
```

## Three.js Integration

See [@procgeo/three](https://www.npmjs.com/package/@procgeo/three) for Three.js helpers.

## Available SOPs

**Creation:** `createBox`, `createGrid`, `createSphere`, `createLine`, `createCircle`, `createTube`, `createTorus`, `createIcosphere`, `createTeapot`, `createMetaball`, `createHelix`, `createSpiral`

**Manipulation:** `transform`, `computeNormals`, `subdivide`, `smooth`, `polyExtrude`, `polyBevel`, `polyWire`, `polyReduce`, `polyFill`, `clip`, `reverse`, `scatter`, `copyToPoints`, `fuse`, `color`, `voronoiFracture`, `booleanOp`, `bend`, `revolve`, `resample`, `quadRemesh`, `quadWild`, `merge`

**Attributes:** `attribCreate`, `attribDelete`, `attribRename`, `attribPromote`, `attribTransfer`, `attribCopy`, `attribRandomize`, `attribSort`, `attribBlur`, `attribFill`, `attribNoise`, `enumerateAttrib`, `measure`

**Groups:** `groupCreate`, `groupCombine`, `blast`, `deleteSop`

**Image (COP):** `copNoise`, `copConstant`, `copRamp`, `copCheckerboard`, `copBlur`, `copResize`, `copFlip`, `copMirror`, `copRotate`, `copSwirl`, `copChannelSwap`, `copComposite`, `copCustomShader`, `copLoadImage`

**Export:** `geo.toObj()`, `geo.toGlb()`
READMEEOF
ok "README.md"

# ─── @procgeo/three ──────────────────────────────────────────────────────────

step "Staging @procgeo/three@${VERSION}"
PKG_THREE="utils/procgeo-three"

cat > "$PKG_THREE/package.json" << EOF
{
  "name": "@procgeo/three",
  "version": "${VERSION}",
  "description": "Three.js bridge for ProcGeo procedural geometry",
  "type": "module",
  "main": "index.js",
  "types": "index.d.ts",
  "files": [
    "index.js",
    "index.d.ts"
  ],
  "exports": {
    ".": {
      "import": "./index.js",
      "types": "./index.d.ts"
    }
  },
  "peerDependencies": {
    "three": ">=0.150.0",
    "@procgeo/lib": ">=${VERSION}"
  },
  "peerDependenciesMeta": {
    "@procgeo/lib": { "optional": true }
  },
  "keywords": ["procgeo", "threejs", "3d", "geometry", "mesh", "wireframe", "pointcloud"],
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/vochsel/rs-procgeo"
  },
  "sideEffects": false
}
EOF
ok "package.json"

cat > "$PKG_THREE/README.md" << 'READMEEOF'
# @procgeo/three

Three.js bridge for [ProcGeo](https://www.npmjs.com/package/@procgeo/lib) procedural geometry.

Converts ProcGeo Geometry objects into Three.js `BufferGeometry`, `Mesh`, wireframe `LineSegments`, and `Points`.

## Install

```bash
npm install @procgeo/three @procgeo/lib three
```

## Usage

```js
import init, { createTorus, subdivide, computeNormals } from '@procgeo/lib';
import { toMesh, toWireframe, toPointCloud, createScene } from '@procgeo/three';

await init();

const geo = computeNormals(subdivide(createTorus(), { depth: 2, mode: 'catmullClark' }));

const { scene, animate } = createScene(document.getElementById('canvas'));
scene.add(toMesh(geo));
scene.add(toWireframe(geo, { color: 0x88aaff }));
animate();
```

## API

| Function | Returns | Description |
|---|---|---|
| `toBufferGeometry(geo, opts?)` | `THREE.BufferGeometry` | Raw buffer geometry with positions, indices, normals, colors |
| `toMesh(geo, opts?)` | `THREE.Mesh` | Mesh with auto-detected vertex colors and configurable material |
| `toWireframe(geo, opts?)` | `THREE.LineSegments` | True polygon-edge wireframe (not triangulated diagonals) |
| `toPointCloud(geo, opts?)` | `THREE.Points` | Point cloud with optional vertex colors |
| `toEdges(geo, opts?)` | `THREE.LineSegments` | Edge outline using Three.js EdgesGeometry |
| `createScene(container, opts?)` | `SceneResult` | Quick scene setup with camera, lights, and animation loop |
READMEEOF
ok "README.md"

# ─── Publish ─────────────────────────────────────────────────────────────────

step "Publishing @procgeo/lib@${VERSION}"
cd "$ROOT/$PKG_LIB"
npm publish --access public $PUBLISH_FLAG
ok "@procgeo/lib published"

step "Publishing @procgeo/three@${VERSION}"
cd "$ROOT/$PKG_THREE"
npm publish --access public $PUBLISH_FLAG
ok "@procgeo/three published"

echo ""
echo -e "${GREEN}Done! Published @procgeo/lib@${VERSION} and @procgeo/three@${VERSION}${NC}"
