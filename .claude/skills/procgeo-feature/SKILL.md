---
name: procgeo-feature
description: Use when implementing a new feature (SOP, core API, or I/O format) into the procgeo Rust crates. Guides the full implementation chain including Rust SOP code, unit tests, benchmarking, registry registration, TypeScript (napi-rs) binding, Python (PyO3) binding, and prelude re-export. Triggers on requests to add a new SOP, geometry operation, creation node, modifier, or any new capability that must be wired through all layers.
---

# Implementing a New Feature in ProcGeo

When adding a new feature to the procgeo library, you MUST implement it across ALL layers. Follow this checklist precisely.

## Pre-Implementation: Decide Where It Lives

| Feature type | Crate | Module location |
|---|---|---|
| New SOP (generator, modifier, multi-input) | `procgeo-sops` | `crates/procgeo-sops/src/<category>/` |
| New geometry primitive or core data structure | `procgeo-core` | `crates/procgeo-core/src/` |
| New I/O format reader/writer | `procgeo-io` | `crates/procgeo-io/src/` |
| New composite operation (COP) | `procgeo-cops` | `crates/procgeo-cops/src/` |

## Coordinate System and Creation Defaults

Treat these as repo-wide geometry conventions unless a source format or explicit user requirement overrides them:

- `Y` is up
- The default ground plane is `XZ`
- Most generator SOPs default to `center = Vec3::ZERO`
- Height-like or axial defaults usually run along `+Y`
- Polygon winding must be CCW when viewed from outside so normals point outward
- New SOP param defaults must match Houdini defaults first; when no Houdini-specific convention applies, follow the repo's existing creation defaults

Common built-in defaults to preserve when adding related features, examples, or bindings:

- `Box`: size `1,1,1`, centered at origin
- `Grid`: `10x10`, `rows=10`, `cols=10`, oriented on `XZ`
- `Circle`: radius `1`, `divisions=40`, oriented on `XZ`
- `Line`: origin at zero, direction `+Y`, length `1`, `points=2`
- `Sphere`: radius `0.5`, centered at origin
- `Tube`: height `1`, centered at origin, aligned to `Y`, no caps by default
- `Helix`: rises along `Y`
- `Spiral`: expands in `XZ`, optional rise in `Y`
- `Add`: starts empty; points/polygons/polylines are explicit user input

SOPs are organized by category. Use an existing category if the feature fits, or create a new one:

| Category | Feature flag | Module | Examples |
|---|---|---|---|
| `creation` | `creation` | `creation/` | BoxSop, GridSop, SphereSop |
| `transform` | `transform` | `transform/` | TransformSop |
| `reshape` | `reshape` | `reshape/` | SubdivideSop, SmoothSop, ClipSop, PolyExtrudeSop |
| `topology` | `topology` | `topology/` | FuseSop, SortSop, ReverseSop |
| `scatter` | `scatter` | `scatter/` | ScatterSop |
| `color` | `color` | `color/` | ColorSop |
| `normals` | `normals` | `normals/` | NormalSop |
| `merge` | `merge` | `merge/` | MergeSop |
| `copy` | `copy` | `copy/` | CopyToPointsSop |
| `attributes` | `attributes` | `attributes/` | AttribCreateSop, AttribNoiseSop |
| `groups_sops` | `groups_sops` | `groups/` | GroupCreateSop, GroupCombineSop |
| `delete` | `delete` | `delete/` | BlastSop, DeleteSop |
| `deform` | `deform` | `deform/` | BendSop, PointDeformSop |
| `boolean` | `boolean` | `boolean/` | BooleanSop |
| `voronoi` | `voronoi` | `voronoi/` | VoronoiFractureSop |
| `measure_sops` | `measure_sops` | `measure/` | MeasureSop |
| `utility_sops` | `utility_sops` | `utility/` | EnumerateSop, NullSop |

## Step 1: Implement the Rust SOP

Create the SOP file at `crates/procgeo-sops/src/<category>/<sop_name>.rs`.

### Params struct

```rust
use serde::{Deserialize, Serialize};
use procgeo_core::Geometry;
use crate::{Sop, SopError};

/// Parameters for the <Name>SOP.
/// All fields must have Houdini-matching defaults.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct <Name>Params {
    pub some_param: f32,
    // ... all params with pub visibility
}

impl Default for <Name>Params {
    fn default() -> Self {
        Self {
            some_param: 1.0,
            // Houdini-matching defaults. If Houdini leaves orientation implied,
            // align with ProcGeo's Y-up / XZ-ground conventions.
        }
    }
}
```

### SOP struct and trait impl

```rust
pub struct <Name>Sop;

impl Sop for <Name>Sop {
    type Params = <Name>Params;

    fn name(&self) -> &'static str {
        "<snake_case_name>"  // Used by SopRegistry for dynamic dispatch
    }

    fn input_count(&self) -> (usize, usize) {
        // (min_inputs, max_inputs)
        // Generator: (0, 0)
        // Modifier:  (1, 1)
        // Multi-input: (2, N) e.g., (2, 2) for CopyToPoints, (1, usize::MAX) for Merge
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        // For generators (0 inputs):
        //   let mut geo = Geometry::new(); or Geometry::with_capacity(n, m);
        // For modifiers (1 input):
        //   let mut out = inputs[0].clone();
        // For multi-input:
        //   Access inputs[0], inputs[1], etc.

        // Implementation here...

        Ok(out)
    }
}
```

### Module registration (`mod.rs`)

In `crates/procgeo-sops/src/<category>/mod.rs`:
```rust
pub mod <sop_name>;
pub use <sop_name>::{<Name>Sop, <Name>Params};
```

If creating a NEW category, also add the module in `crates/procgeo-sops/src/lib.rs`:
```rust
#[cfg(feature = "<feature_flag>")]
pub mod <category>;
```

### Unit tests

Include tests in the same SOP file, inside a `#[cfg(test)] mod tests` block:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::creation::grid::{GridSop, GridParams};
    use crate::{GeometryExt, generate};

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    fn make_grid() -> Geometry {
        generate(&GridSop, &GridParams { rows: 10, cols: 10, ..Default::default() }).unwrap()
    }

    #[test]
    fn <name>_basic() {
        let geo = make_box();
        let result = geo.apply(&<Name>Sop, &<Name>Params::default()).unwrap();
        // Assert geometry invariants: point count, prim count, attributes, etc.
        assert!(result.num_points() > 0);
    }

    #[test]
    fn <name>_specific_behavior() {
        // Test the specific feature behavior with non-default params
    }

    #[test]
    fn <name>_edge_cases() {
        // Test empty geometry, single point, degenerate cases
    }
}
```

**Test guidelines:**
- Always test with `Default::default()` params
- Test at least one non-default param combination
- Test edge cases (empty geometry, single point/prim)
- Use `approx` crate for floating-point comparisons where needed
- For generators: verify point count, prim count, bounding box
- For generators with orientation semantics: assert the actual axis/plane, not just counts
- For polygons/surfaces: assert signed normal direction or winding-sensitive behavior, not `abs()`
- For modifiers: verify the operation transforms geometry correctly
- For attributes: verify attribute existence, type, and values

## Step 2: Register in SopRegistry

In `crates/procgeo-sops/src/registry.rs`, inside `default_registry()`:

```rust
// <Category Name>
#[cfg(feature = "<feature_flag>")]
{
    r.add(crate::<category>::<Name>Sop);
}
```

If adding to an existing category block, just add the `r.add(...)` line inside the existing `#[cfg]` block.

## Step 3: Feature Flags (if new category)

Only needed when creating a NEW feature category. Skip if adding to an existing category.

**`crates/procgeo-sops/Cargo.toml`** - add to `[features]`:
```toml
<feature_flag> = []
```
And add to the `default` list.

**`crates/procgeo/Cargo.toml`** - add to `[features]`:
```toml
<feature_flag> = ["procgeo-sops/<feature_flag>"]
```
And add to the `default` list.

## Step 4: Re-export in Prelude

In `crates/procgeo/src/lib.rs`, inside the `prelude` module:

```rust
#[cfg(feature = "<feature_flag>")]
pub use procgeo_sops::<category>::*;
```

If adding to an existing category, this is already done. Only needed for new categories.

## Step 5: TypeScript/Node.js Binding (napi-rs)

In `bindings/procgeo-node/src/lib.rs`, add a `#[napi]` function.

### Pattern for generator SOPs (0 inputs):

```rust
#[napi]
pub fn create_<name>(params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::<category>::<Name>Params {
        some_param: get_f32(&p, "some_param", 1.0),
        // Map each param using helpers: get_f32, get_u32, get_vec3, get_bool, get_str
        ..Default::default()
    };
    let inner = procgeo_sops::<category>::<Name>Sop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}
```

### Pattern for modifier SOPs (1 input):

```rust
#[napi]
pub fn <snake_name>(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::<category>::<Name>Params {
        some_param: get_f32(&p, "some_param", 1.0),
        // ...
    };
    let inner = procgeo_sops::<category>::<Name>Sop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}
```

### Pattern for multi-input SOPs (2+ inputs):

```rust
#[napi]
pub fn <snake_name>(a: &Geometry, b: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::<category>::<Name>Params { /* ... */ };
    let inner = procgeo_sops::<category>::<Name>Sop
        .execute(&[&a.inner, &b.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}
```

**Node binding conventions:**
- Params use **snake_case** keys (matching Rust serde)
- All params are optional via `Option<serde_json::Value>`
- Use helper functions: `get_f32()`, `get_u32()`, `get_vec3()`, `get_bool()`, `get_str()`
- Error conversion via `sop_err()`
- Generator functions are prefixed with `create_`
- Modifier functions use the SOP name directly (e.g., `transform`, `subdivide`, `color`)

## Step 6: Python Binding (PyO3)

In `bindings/procgeo-py/src/lib.rs`:

### Add the function:

```rust
#[pyfunction]
#[pyo3(signature = (geo, some_param=1.0, other_param=0))]
fn <snake_name>(geo: &Geometry, some_param: f32, other_param: i32) -> PyResult<Geometry> {
    let params = procgeo_sops::<category>::<Name>Params {
        some_param,
        other_param,
        // ...
    };
    let inner = procgeo_sops::<category>::<Name>Sop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}
```

For generators (no `geo` input):
```rust
#[pyfunction]
#[pyo3(signature = (some_param=1.0))]
fn create_<name>(some_param: f32) -> PyResult<Geometry> {
    let params = procgeo_sops::<category>::<Name>Params { some_param, ..Default::default() };
    let inner = procgeo_sops::<category>::<Name>Sop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}
```

### Register in the module:

In the `fn procgeo(m: &Bound<'_, PyModule>) -> PyResult<()>` function at the bottom:
```rust
m.add_function(wrap_pyfunction!(<snake_name>, m)?)?;
```

**Python binding conventions:**
- Each param is a separate keyword argument with default value in `#[pyo3(signature = (...))]`
- Types use native Python-compatible types: `f32`, `i32`, `bool`, `&str`, `Vec<f32>`, tuples
- Vec3 values are passed as separate `_x`, `_y`, `_z` arguments (e.g., `translate_x`, `translate_y`, `translate_z`)
- Error conversion via `sop_err()` which returns `PyRuntimeError`
- Generator functions are prefixed with `create_`

## Step 7: Benchmarks

### Rust benchmarks

Add to `benchmarks/rust/src/main.rs`. Create a new function `bench_<category>` or add to an existing one:

```rust
fn bench_<category>(results: &mut Vec<BenchResult>) {
    let scales: &[u32] = &[100, 10_000, 100_000];

    for &scale in scales {
        let rc = grid_rc(scale);

        // Create input geometry for the benchmark
        let grid = generate(&GridSop, &GridParams { rows: rc, cols: rc, ..Default::default() }).unwrap();

        let (mean, std, iters) = bench(|| {
            let _ = grid.clone().apply(
                &<Name>Sop,
                &<Name>Params {
                    // representative params
                    ..Default::default()
                },
            );
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "<category>",
            operation: "<operation_name>",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });
    }
}
```

Call it from `main()`:
```rust
eprintln!("Running <category> benchmarks...");
bench_<category>(&mut results);
```

### TypeScript benchmarks

Add to `benchmarks/typescript/bench.ts`:
```ts
// -- <Name> --
for (const scale of SCALES) {
  const rc = gridRC(scale);
  const grid = pg.createGrid({ rows: rc, cols: rc });

  const { mean, std, iters } = bench(() => {
    pg.<snakeName>(grid, { /* params */ });
  });
  emit({
    framework: "procgeo",
    language: "typescript",
    category: "<category>",
    operation: "<operation>",
    scale,
    mean_ms: mean,
    std_ms: std,
    iterations: iters,
  });
}
```

### Python benchmarks

Add to `benchmarks/python/bench_procgeo.py`:
```python
# -- <Name> --
mean, std, iters = bench(lambda: pg.<snake_name>(grid, some_param=value))
emit_result(FW, LANG, "<category>", "<operation>", scale, mean, std, iters)
```

## Step 8: Verify

Run these commands to verify the implementation compiles across all targets:

```bash
# Rust tests
cargo test -p procgeo-sops
cargo test -p procgeo

# Node binding compiles
cargo build -p procgeo-node

# Python binding compiles
cargo build -p procgeo-py

# Rust benchmarks compile
cargo build -p procgeo-bench-compare
```

## Checklist Summary

For every new feature, you MUST complete ALL of these:

- [ ] **Rust SOP**: `crates/procgeo-sops/src/<category>/<name>.rs` with Params + Sop impl
- [ ] **Module export**: `mod.rs` with `pub use`
- [ ] **Registry**: `r.add()` in `default_registry()` in `registry.rs`
- [ ] **Feature flag**: Only if new category (Cargo.toml in both `procgeo-sops` and `procgeo`)
- [ ] **Prelude**: Only if new category (`crates/procgeo/src/lib.rs`)
- [ ] **Unit tests**: In the SOP file, `#[cfg(test)] mod tests`
- [ ] **Node binding**: `#[napi]` function in `bindings/procgeo-node/src/lib.rs`
- [ ] **Python binding**: `#[pyfunction]` + module registration in `bindings/procgeo-py/src/lib.rs`
- [ ] **Rust benchmark**: In `benchmarks/rust/src/main.rs`
- [ ] **TS benchmark**: In `benchmarks/typescript/bench.ts`
- [ ] **Python benchmark**: In `benchmarks/python/bench_procgeo.py`
- [ ] **Compile check**: `cargo build -p procgeo-node && cargo build -p procgeo-py`
