# ProcGeo

Procedural geometry library in Rust, inspired by Houdini SOPs.

## Role and structure
When adding features, always add in rust in the most performant, GPU optimized way, that will work across platform. You also need to build out the python and wasm bindings.
The goal of this is to build complex, procedural systems out of code that mimic real life geometry, scenes, and structure.

Always create unit tests to prevent regressions.

Benchmark results in benchmarks/ across similar libraries in various languages.

The wasm/web build allows in browser and node compilation of procedural systems to threejs for realtime rendering.
The python build allows interaction with custom ai model training, and interaction with pixars pxr-usd library (until native rust integration exists)

## Architecture

The repo is a monorepo with a Cargo workspace (`crates/*`, `bindings/*`,
`benchmarks/rust`) and a pnpm + Turborepo JS workspace (`apps/*`, `packages/*`).

**Rust crates** (`crates/`)
- `procgeo-core` — Geometry model (points, vertices, primitives, attributes, groups)
- `procgeo-sops` — SOP implementations (feature-gated by category)
- `procgeo-io` — Format readers/writers (OBJ, glTF)
- `procgeo` — Umbrella crate with `prelude` module

**Bindings** (`bindings/`)
- `bindings/procgeo-wasm` — TypeScript/JavaScript bindings for browser and Node.js (wasm-bindgen)
- `bindings/procgeo-py` — Python bindings (PyO3/maturin)

**Apps & packages**
- `apps/web` — `@procgeo/playground`, the Vite + Three.js + Monaco web playground (runs procgeo via WASM)
- `apps/desktop` — `@procgeo/desktop`, the native Tauri app. Its `src-tauri` crate depends on procgeo as a **native Rust dependency** (no WASM/IPC of geometry logic — only render buffers cross to the webview). It is **detached** from the root Cargo workspace via its own empty `[workspace]`, so Tauri's GUI deps stay out of `cargo build --workspace`.
- `packages/three` — `@procgeo/three`, the Three.js bridge (BufferGeometry/Mesh/wireframe helpers)

## Conventions

- All SOP parameters have `Default` matching Houdini defaults
- SOPs are stateless: `fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry>`
- Point positions stored SoA (separate x/y/z vecs) for SIMD
- Attributes use typed handles: `AttribHandle<T>` for compile-time safety
- Groups use bitsets (`bitvec`)
- Uses `glam` for vector/matrix math
- Use `GeometryExt` trait for `.apply()` chaining

### Winding order convention

All polygon faces must use **CCW winding when viewed from outside** (right-hand rule), producing **outward-pointing normals** via Newell's method. This matches Houdini's convention.

- Creation SOPs (box, sphere, tube, grid, etc.) must emit faces with outward normals
- SOPs that compute face normals (polyextrude, normal, group_create) use Newell's method and depend on correct winding
- When adding new faces in any SOP, verify the normal direction with `(v1-v0).cross(v2-v0)` or Newell's method
- Tests that check extrusion/normal direction must assert the actual signed value, not `abs()`

## Bindings

**Bindings must always be kept up to date.** When adding or modifying SOPs, I/O formats, or core Geometry APIs:

1. Add corresponding wrapper functions/methods to both `bindings/procgeo-wasm/src/lib.rs` (wasm-bindgen) and `bindings/procgeo-py/src/lib.rs` (PyO3)
2. Expose new SOPs as functions with params as JS objects / Python kwargs
3. Ensure new Geometry methods are mirrored on the binding's Geometry class
4. Update `apps/web/src/procgeo-editor-types.d.ts` so Monaco autocomplete stays aligned with the WASM API, then run `pnpm build:editor-types`
5. Test that both bindings compile: `cargo build -p procgeo-wasm && cargo build -p procgeo-py`

New SOPs need no change to the desktop app: `apps/desktop` dispatches through `procgeo_sops::default_registry()`, so any registered SOP is callable by name from `cook`.
