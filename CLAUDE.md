# ProcGeo

Procedural geometry library in Rust, inspired by Houdini SOPs.

## Build & Test

```bash
cargo build            # build all crates
cargo test --workspace # run all tests
cargo test -p procgeo-core  # test core only
cargo test -p procgeo-sops  # test SOPs only
```

## Architecture

- `procgeo-core` — Geometry model (points, vertices, primitives, attributes, groups)
- `procgeo-sops` — SOP implementations (feature-gated by category)
- `procgeo-io` — Format readers/writers (OBJ, glTF)
- `procgeo` — Umbrella crate with `prelude` module
- `bindings/procgeo-node` — TypeScript/Node.js bindings (napi-rs)
- `bindings/procgeo-py` — Python bindings (PyO3/maturin)

## Conventions

- All SOP parameters have `Default` matching Houdini defaults
- SOPs are stateless: `fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry>`
- Point positions stored SoA (separate x/y/z vecs) for SIMD
- Attributes use typed handles: `AttribHandle<T>` for compile-time safety
- Groups use bitsets (`bitvec`)
- Uses `glam` for vector/matrix math
- Use `GeometryExt` trait for `.apply()` chaining

## Bindings

**Bindings must always be kept up to date.** When adding or modifying SOPs, I/O formats, or core Geometry APIs:

1. Add corresponding wrapper functions/methods to both `bindings/procgeo-node/src/lib.rs` (napi-rs) and `bindings/procgeo-py/src/lib.rs` (PyO3)
2. Expose new SOPs as functions with params as JS objects / Python kwargs
3. Ensure new Geometry methods are mirrored on the binding's Geometry class
4. Test that both bindings compile: `cargo build -p procgeo-node && cargo build -p procgeo-py`

## Documentation

**The docs site (`web/public/docs/`) must be kept up to date.** When making API changes:

1. **Adding a new SOP**: Add a `.sop-card` entry to the appropriate SOPs reference page (`web/public/docs/sops-creation.html`, `sops-transform.html`, `sops-topology.html`, or `sops-utility.html`) with params table and example
2. **Adding a new I/O format**: Update `web/public/docs/io.html` with the format's capabilities table row and usage examples
3. **Changing core Geometry API**: Update `web/public/docs/geometry.html` (for points/vertices/primitives), `web/public/docs/attributes.html` (for attribute system), or `web/public/docs/groups.html` (for group system)
4. **Updating bindings**: Update the corresponding binding page (`web/public/docs/bindings-rust.html`, `bindings-wasm.html`, `bindings-node.html`, `bindings-python.html`) with new function signatures and examples
5. **Adding WASM functions**: Also update the type definitions in `web/src/main.js` (the `procgeoTypes` const) so the playground gets autocomplete

### Docs pages structure
- `web/public/docs/index.html` — Overview and quick start
- `web/public/docs/architecture.html` — Crate layout, design principles, feature flags
- `web/public/docs/geometry.html` — Core geometry model (points, vertices, primitives)
- `web/public/docs/attributes.html` — Attribute system (classes, types, handles)
- `web/public/docs/groups.html` — Group system (element groups, boolean ops)
- `web/public/docs/sops-creation.html` — Creation SOPs (Box, Grid, Sphere, etc.)
- `web/public/docs/sops-transform.html` — Transform & Reshape SOPs
- `web/public/docs/sops-topology.html` — Topology SOPs (Fuse, Sort, Blast, etc.)
- `web/public/docs/sops-utility.html` — Utility, Measure, Color, Attribute, Group SOPs
- `web/public/docs/bindings-rust.html` — Rust API reference
- `web/public/docs/bindings-wasm.html` — WebAssembly API reference
- `web/public/docs/bindings-node.html` — Node.js API reference
- `web/public/docs/bindings-python.html` — Python API reference
- `web/public/docs/io.html` — I/O formats (OBJ, glTF)
- `web/public/docs/examples.html` — Cookbook with Rust/JS/Python examples
- `web/public/docs/style.css` — Shared styles (all pages link to this)
