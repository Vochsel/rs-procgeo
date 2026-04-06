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
