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
      └── procgeo-wasm   WebAssembly (wasm-bindgen)
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
// Chaining with .apply()
let result = geo
    .apply(&TransformSop, &TransformParams { translate: Vec3::Y, ..Default::default() })?
    .apply(&NormalSop, &NormalParams::default())?;

// Generator SOPs (no inputs)
let grid = generate(&GridSop, &GridParams::default())?;
```

### Available SOPs

| Category | SOPs |
|----------|------|
| **Creation** | Box, Grid, Line, Circle, Sphere, Tube, Torus |
| **Transform** | Transform |
| **Normals** | Normal |
| **Merge** | Merge |
| **Attributes** | Attribute Create, Delete, Promote, Rename |
| **Groups** | Group Create, Group Combine |
| **Delete** | Blast, Delete |
| **Copy** | Copy to Points |
| **Reshape** | Subdivide, PolyExtrude, Clip |
| **Scatter** | Scatter |
| **Topology** | Sort, Fuse, Connectivity, Resample, Reverse |
| **Measure** | Measure |
| **Utility** | Enumerate, Null |
| **Color** | Color |

Each category is a Cargo feature flag. Disable what you don't need:

```toml
[dependencies]
procgeo = { version = "0.1", default-features = false, features = ["creation", "transform", "obj"] }
```

### I/O

```rust
use procgeo::io::{write_file, read_file};

// Extension-based dispatch
write_file(&geo, Path::new("output.obj"))?;
write_file(&geo, Path::new("output.glb"))?;
let geo = read_file(Path::new("model.obj"))?;
```

| Format | Read | Write | Feature |
|--------|------|-------|---------|
| Wavefront OBJ | Yes | Yes | `obj` |
| glTF 2.0 / GLB | -- | Yes | `gltf` |

## Design

### SoA Point Layout

Points are stored as three contiguous `Vec<f32>` arrays — one each for x, y, z — rather than as an array of `Vec3` structs. This Structure-of-Arrays layout is the same approach used by high-performance physics engines and particle systems:

```rust
pub struct PointStorage {
    x: Vec<f32>,   // [x0, x1, x2, ...]
    y: Vec<f32>,   // [y0, y1, y2, ...]
    z: Vec<f32>,   // [z0, z1, z2, ...]
}
```

This layout enables SIMD processing over each component independently and produces better cache utilization when iterating over a single axis (e.g., filtering by Y height).

### Typed Handles

Raw `usize` indices are error-prone — it's easy to accidentally use a point index where a primitive index is expected. ProcGeo uses distinct newtypes for each element class:

```rust
let point: PointHandle = geo.add_point(Vec3::ZERO);
let prim: PrimHandle = geo.add_face(&[p0, p1, p2])?;
// These are different types — can't be confused at compile time
```

### Bitwise Groups

Groups are `BitVec`-backed, making boolean operations (union, intersect, subtract, complement) operate on whole machine words rather than element-by-element. This is critical for SOPs like Blast and Delete that need to resolve complex group expressions before acting.

### SmallVec Vertex Lists

Most primitives (triangles, quads) have 3-4 vertices. `SmallVec<[VertexHandle; 4]>` stores these inline without heap allocation, falling back to the heap only for high-valence polygons.

### Feature-Gated Compilation

Each SOP category is a Cargo feature. The umbrella crate enables all by default, but downstream users can compile only what they need — useful for WASM bundles or embedded targets where binary size matters.

## Performance

ProcGeo is designed for high-throughput geometry processing:

- **SoA memory layout** — contiguous component arrays for cache-friendly SIMD iteration
- **glam math** — SSE2/SSE4.1/NEON-accelerated vector and matrix operations
- **SmallVec** — inline vertex storage avoids heap allocation for typical primitives
- **BitVec groups** — word-at-a-time boolean operations on element selections
- **Pre-allocated capacity** — `Geometry::with_capacity(points, prims)` avoids reallocation in generators
- **Stateless SOPs** — pure functions enable parallel execution and result caching

Benchmarks (Criterion.rs) cover point add/read/write, SoA iteration, primitive construction, attribute CRUD, topology rebuilds, and group operations:

```bash
cargo bench -p procgeo-core
```

## Bindings

### Node.js (napi-rs)

```javascript
const { createBox, createGrid, copyToPoints } = require('@procgeo/core');

const grid = createGrid({ rows: 10, columns: 10 });
const box = createBox({ size_x: 0.1, size_y: 0.1, size_z: 0.1 });
const instances = copyToPoints(box, grid);
console.log(`${instances.numPoints} points`);
```

### Python (PyO3)

```python
import procgeo

grid = procgeo.create_grid(rows=10, columns=10)
box = procgeo.create_box(size_x=0.1, size_y=0.1, size_z=0.1)
instances = procgeo.copy_to_points(box, grid)
print(f"{instances.num_points} points")
```

Both bindings are thin wrappers — all geometry logic stays in Rust. Build with:

```bash
cargo build -p procgeo-node   # Node.js
cargo build -p procgeo-py     # Python (or use maturin)
```

## Build & Test

```bash
cargo build                    # build all crates
cargo test --workspace         # run all tests
cargo test -p procgeo-core     # test core only
cargo test -p procgeo-sops     # test SOPs only
cargo bench -p procgeo-core    # run benchmarks
```

## License

MIT
