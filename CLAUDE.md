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
- `procgeo-io` — Format readers/writers (OBJ, future: glTF, USD)
- `procgeo` — Umbrella crate with `prelude` module

## Conventions

- All SOP parameters have `Default` matching Houdini defaults
- SOPs are stateless: `fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry>`
- Point positions stored SoA (separate x/y/z vecs) for SIMD
- Attributes use typed handles: `AttribHandle<T>` for compile-time safety
- Groups use bitsets (`bitvec`)
- Uses `glam` for vector/matrix math
- Use `GeometryExt` trait for `.apply()` chaining
