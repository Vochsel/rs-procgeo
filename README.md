# ProcGeo

Procedural geometry library for Rust, inspired by Houdini's SOP (Surface Operator) architecture.

ProcGeo brings Houdini's proven geometry model — points, vertices, primitives, attributes, and groups — into Rust as a composable, high-performance library with bindings for Node.js, Python, and WebAssembly.

## Motivation

Houdini's procedural geometry pipeline is one of the most expressive and battle-tested systems in VFX and games. But it lives inside a proprietary DCC application. ProcGeo extracts the core ideas — stateless operators, typed attributes, element-level granularity — and rebuilds them as a standalone library with a few design goals:

- **Portability.** Use procedural geometry anywhere: game engines, web apps, CLI tools, cloud pipelines — not just inside Houdini.
- **Composability.** SOPs are pure functions. Chain them, branch them, run them in parallel. No hidden state, no side effects.
- **Performance.** SoA memory layout, SIMD-accelerated math, and zero-cost abstractions let you process millions of points without leaving Rust's safety guarantees.
- **Polyglot.** First-class bindings for TypeScript/Node.js (napi-rs), Python (PyO3), and WebAssembly mean the same geometry engine works across your entire stack.

## Quick Start

```toml
# Cargo.toml
[dependencies]
procgeo = "0.1"
```

```rust
use procgeo::prelude::*;
use glam::Vec3;

fn main() -> Result<(), SopError> {
    let geo = generate(&BoxSop, &BoxParams {
        size: Vec3::splat(2.0),
        center: Vec3::ZERO,
    })?
    .apply(&SubdivideSop, &SubdivideParams { depth: 2, ..Default::default() })?
    .apply(&NormalSop, &NormalParams::default())?;

    println!("{} points, {} prims", geo.num_points(), geo.num_prims());
    Ok(())
}
```

## Architecture

```
procgeo (umbrella crate, re-exports + prelude)
 ├── procgeo-core     Geometry model, attributes, groups, math
 ├── procgeo-sops     SOP implementations, feature-gated by category
 ├── procgeo-io       Format readers/writers (OBJ, glTF)
 └── bindings/
      ├── procgeo-node   TypeScript/Node.js (napi-rs)
      ├── procgeo-py     Python (PyO3/maturin)
      ├── procgeo-wasm   WebAssembly (wasm-bindgen)
      └── procgeo-three  Three.js bridge (toMesh, toBufferGeometry, etc.)
```

### Core Geometry Model

The `Geometry` struct mirrors Houdini's geometry container:

| Element | Storage | Handle | Description |
|---------|---------|--------|-------------|
| **Points** | SoA (`x[]`, `y[]`, `z[]`) | `PointHandle(u32)` | Spatial positions |
| **Vertices** | `(PointHandle, PrimHandle)` pairs | `VertexHandle(u32)` | Primitive-point topology |
| **Primitives** | Enum-dispatched (`Polygon`, ...) | `PrimHandle(u32)` | Faces, curves, etc. |
| **Attributes** | Typed per-class storage | `AttribHandle<T>` | Data on any element class |
| **Groups** | `BitVec` (point/prim/vertex) | Named | Boolean element selection |

Attributes support four classes — **Point**, **Vertex**, **Primitive**, **Detail** — and store typed data including integers, floats, vectors, matrices, and strings. Type qualifiers (`point`, `vector`, `normal`, `color`) carry semantic meaning through the pipeline.

### Intrinsic Attributes

Well-known attributes are treated as first-class citizens, matching Houdini conventions:

| Attribute | Type | Class | Description |
|-----------|------|-------|-------------|
| `P` | Vector3 | Point | Position — unified with `PointStorage`, writable via attribute API |
| `N` | Vector3 | Point | Normal — created by Normal SOP, exported in OBJ/glTF |
| `Cd` | Vector3 | Point | Color — created by Color SOP, exported in glTF as `COLOR_0` |

Writing to `P` via the attribute API (e.g., Attribute Noise with `attrib_name="P"`) actually moves points — the SoA position storage and attribute map stay in sync.

### The SOP Trait

Every operator implements the same stateless interface:

```rust
pub trait Sop {
    type Params: Default;
    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError>;
    fn input_count(&self) -> (usize, usize); // (min, max)
    fn name(&self) -> &'static str;
}
```

SOPs never mutate their inputs. They take immutable references and return a new `Geometry`. This makes them safe to cache, parallelize, and compose:

```rust
let result = geo
    .apply(&TransformSop, &TransformParams { translate: Vec3::Y, ..Default::default() })?
    .apply(&NormalSop, &NormalParams::default())?;
```

### SOP Registry

All SOPs register in a `SopRegistry` that enables dynamic dispatch by name. Bindings use this for automatic sync — adding a new SOP requires only one line in the registry, no binding code changes needed.

```rust
let registry = procgeo_sops::default_registry();
let geo = registry.execute("box", &[], "{}")?;
let moved = registry.execute("transform", &[&geo], r#"{"translate":[10,0,0]}"#)?;
println!("{:?}", registry.list()); // ["attrib_blur", "attrib_copy", "box", ...]
```

### Available SOPs

| Category | SOPs |
|----------|------|
| **Creation** | Box, Grid, Line, Circle, Sphere, Tube, Torus |
| **Transform** | Transform |
| **Normals** | Normal |
| **Merge** | Merge |
| **Attributes** | Create, Delete, Promote, Rename, Transfer, Copy, Randomize, Sort, Blur, Fill, Noise |
| **Groups** | Group Create, Group Combine |
| **Delete** | Blast, Delete |
| **Copy** | Copy to Points |
| **Reshape** | Subdivide (Linear + Catmull-Clark), PolyExtrude, Smooth, Clip |
| **Scatter** | Scatter |
| **Topology** | Sort, Fuse, Connectivity, Resample, Reverse |
| **Voronoi** | Voronoi Fracture |
| **Measure** | Measure (Area, Perimeter) |
| **Utility** | Enumerate, Null |
| **Color** | Color |

The Attribute Noise SOP includes built-in Perlin, Simplex, and Worley (F1/F2-F1) noise with fBm and ridged multifractal fractal layering — all implemented from scratch, no external crate.

Each category is a Cargo feature flag. Disable what you don't need:

```toml
[dependencies]
procgeo = { version = "0.1", default-features = false, features = ["creation", "transform", "obj"] }
```

### I/O

```rust
use procgeo::io::{write_file, read_file};

write_file(&geo, Path::new("output.obj"))?;
write_file(&geo, Path::new("output.glb"))?;
let geo = read_file(Path::new("model.obj"))?;
```

| Format | Read | Write | Feature |
|--------|------|-------|---------|
| Wavefront OBJ | Yes | Yes | `obj` |
| glTF 2.0 / GLB | — | Yes | `gltf` |

## Bindings

### WebAssembly (browser + Node.js)

Published as [`@vochsel/procgeo-js`](https://www.npmjs.com/package/@vochsel/procgeo-js) on npm.

```bash
npm install @vochsel/procgeo-js
```

```js
import init, { createBox, subdivide, computeNormals, attribNoise } from '@vochsel/procgeo-js';

await init();

const box = createBox({ size: [2, 2, 2] });
const subdiv = subdivide(box, { depth: 2, mode: 'catmullClark' });
const noisy = attribNoise(subdiv, {
  attribName: 'P', dimensions: 3,
  noiseType: 'simplex', fractal: 'standard',
  octaves: 4, amplitude: 0.3,
});
const geo = computeNormals(noisy);

// Export
const obj = geo.toObj();   // OBJ string
const glb = geo.toGlb();   // GLB Uint8Array

// WebGL / Three.js buffers
const positions = geo.getPositions();       // Float32Array
const indices = geo.getTriangleIndices();   // Uint32Array
const normals = geo.getNormals();           // Float32Array | undefined
```

#### Three.js Bridge

```js
import { toMesh, toWireframe, toPointCloud } from '@vochsel/procgeo-js/three';

scene.add(toMesh(geo));                    // shaded mesh with auto-material
scene.add(toWireframe(geo));               // wireframe overlay
scene.add(toPointCloud(scatterResult));    // point cloud
```

#### Generic Dispatch

New SOPs are automatically available without binding updates:

```js
const names = pg.listSops();               // all registered SOP names
const geo = pg.executeSopCreate('box', { size: [1, 1, 1] });
const moved = pg.executeSop('transform', geo, { translate: [5, 0, 0] });
```

### Node.js (napi-rs)

```javascript
const pg = require('@procgeo/core');

const grid = pg.createGrid({ rows: 10, cols: 10 });
const box = pg.createBox({ size: [0.1, 0.1, 0.1] });
const instances = pg.copyToPoints(box, grid);
console.log(`${instances.numPoints} points`);
```

### Python (PyO3)

```python
import procgeo

grid = procgeo.create_grid(rows=10, cols=10)
box = procgeo.create_box(size_x=0.1, size_y=0.1, size_z=0.1)
instances = procgeo.copy_to_points(box, grid)
print(f"{instances.num_points} points")
```

## Web Playground

An interactive playground at `web/` with a Monaco code editor (with autocomplete) and Three.js viewport. Write procedural geometry code and see it render in real-time.

```bash
pnpm setup:web    # install deps + copy WASM
pnpm dev:web      # start Vite dev server
```

Includes 14 example presets: noise terrain, Catmull-Clark sphere, scatter instances, extruded city, voronoi fracture, and more.

## Build

One command builds everything — Rust, tests, and all bindings:

```bash
pnpm build         # or: ./scripts/build.sh
```

This runs:
1. `cargo build --release` + `cargo test --workspace`
2. Node.js binding (napi-rs)
3. WASM build + auto-copies to `web/wasm/`
4. Python binding (maturin)

Individual targets:

```bash
pnpm build:rust      # Rust only (build + test)
pnpm build:node      # Node.js binding
pnpm build:wasm      # WASM + copy to web/
pnpm build:python    # Python binding
pnpm test            # cargo test --workspace
pnpm check           # cargo check all crates + bindings
pnpm bench           # criterion benchmarks
pnpm dev:web         # start playground dev server
```

## License

MIT
