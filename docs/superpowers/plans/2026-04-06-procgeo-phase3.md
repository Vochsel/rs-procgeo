# ProcGeo Phase 3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add glTF export, TypeScript bindings (napi-rs), Python bindings (PyO3), and more SOPs (Resample, Smooth, Reverse, Color, Null, Dissolve, Clip).

**Architecture:** glTF via `gltf-json` crate for writing. napi-rs wraps the umbrella crate into an npm package. PyO3+maturin wraps into a pip package. Both bindings expose Geometry class + all SOPs as functions.

**Tech Stack:** gltf-json for glTF, napi-rs/napi-derive for TS, pyo3/maturin for Python

---

## File Structure (new files only)

```
crates/
  procgeo-io/
    src/
      gltf.rs                        # glTF/GLB writer
  procgeo-sops/
    src/
      topology/
        resample.rs                   # Resample SOP
        dissolve.rs                   # Dissolve SOP
        reverse.rs                    # Reverse SOP
      reshape/
        smooth.rs                     # Smooth SOP
        clip.rs                       # Clip SOP
      color/
        mod.rs
        color.rs                      # Color SOP
      utility/
        null.rs                       # Null SOP
bindings/
  procgeo-node/
    Cargo.toml
    package.json
    src/
      lib.rs                          # napi-rs bindings
    index.d.ts                        # (auto-generated)
  procgeo-py/
    Cargo.toml
    pyproject.toml
    src/
      lib.rs                          # PyO3 bindings
```

---

### Task 1: More SOPs — Resample, Smooth, Reverse, Dissolve, Clip, Color, Null

**Files:** New files in procgeo-sops/src/

- [ ] **Resample SOP** (topology/resample.rs): Resample curves/polylines to uniform segment length.
  ResampleParams: length (f32, default 0.1), max_segments (u32, default 1000).
  For each open polyline: walk along edges, place new points at uniform intervals. Closed polygons pass through unchanged.
  Tests: resample_line (5-point line resampled to 10 segments), resample_preserves_closed.

- [ ] **Smooth SOP** (reshape/smooth.rs): Laplacian smoothing on mesh points.
  SmoothParams: iterations (u32, default 1), strength (f32, default 0.5).
  For each iteration: for each point, compute average of neighbor positions (neighbors = points sharing an edge via shared prims). New pos = lerp(old, avg, strength).
  Tests: smooth_reduces_bbox (smoothing a subdivided box shrinks it), smooth_zero_strength (no change).

- [ ] **Reverse SOP** (topology/reverse.rs): Reverse vertex winding order on polygons (flip normals).
  ReverseParams: (empty).
  For each polygon prim, reverse the vertex order.
  Tests: reverse_flips_normals (Normal SOP before and after → normals inverted).

- [ ] **Dissolve SOP** (topology/dissolve.rs): Remove edges between coplanar faces, merging them into larger polygons.
  DissolveParams: angle_threshold (f32, default 0.01 radians).
  Simplified version: for flat surfaces (like Grid), merge adjacent coplanar quads.
  Actually, simplest useful version: dissolve by removing flat edges. Just implement removing primitives by group for now.
  Tests: dissolve_flat_grid (grid with all coplanar faces → reduced face count).

- [ ] **Clip SOP** (reshape/clip.rs): Cut geometry by a plane, keeping one side.
  ClipParams: origin (Vec3, default ZERO), normal (Vec3, default Y — keeps above), keep_above (bool, default true).
  For each face: classify points as above/below plane. If all above → keep. If all below → discard. If mixed → clip by computing intersection points on edges crossing the plane, build new face.
  Tests: clip_box_half (box clipped at y=0 → 5 faces), clip_preserves_if_all_above.

- [ ] **Color SOP** (color/color.rs): Set the Cd (color) attribute.
  ColorParams: color ([f32;3], default [1,1,1]), class (AttribClass, default Point).
  Clone input, create/overwrite "Cd" Vector3 attribute with Color qualifier, set all elements to the color.
  Tests: color_sets_cd (verify Cd attribute exists with correct values), color_on_prims.

- [ ] **Null SOP** (utility/null.rs): Pass-through (no-op). Used as a placeholder in node graphs.
  NullParams: (empty).
  Clone input and return.
  Tests: null_passthrough (verify identical output).

- [ ] Add feature flags: `color` in Cargo.toml, add `pub mod color` in lib.rs
- [ ] Update all mod.rs files to export new SOPs
- [ ] Run tests, commit: "feat(sops): add Resample, Smooth, Reverse, Dissolve, Clip, Color, Null SOPs"

---

### Task 2: glTF Export

**Files:**
- Create: `crates/procgeo-io/src/gltf.rs`
- Modify: `crates/procgeo-io/Cargo.toml`
- Modify: `crates/procgeo-io/src/lib.rs`

- [ ] Add `gltf-json = "0.4"` to workspace and procgeo-io deps
- [ ] Add `gltf` feature flag to procgeo-io

- [ ] Implement GltfWriter:
  - Build a single mesh with all polygon faces
  - Positions as FLOAT VEC3 accessor
  - Normals (if "N" attribute exists) as FLOAT VEC3 accessor
  - Colors (if "Cd" attribute exists) as FLOAT VEC3 accessor
  - Indices as UNSIGNED_INT SCALAR accessor
  - Triangulate quads/ngons (fan from first vertex) for glTF compatibility
  - Write as GLB (binary glTF) for single-file output
  - Pack all buffer data into a single binary buffer

- [ ] Implement write_glb(geo, writer) function
- [ ] Add "glb"/"gltf" to the write_file dispatcher

- [ ] Tests:
  - gltf_write_box (write box to buffer, verify non-empty)
  - gltf_write_with_normals (box + Normal SOP → write with normals)
  - gltf_write_with_colors (box + Color SOP → write with colors)

- [ ] Commit: "feat(io): add glTF/GLB writer"

---

### Task 3: TypeScript Bindings (napi-rs)

**Files:**
- Create: `bindings/procgeo-node/Cargo.toml`
- Create: `bindings/procgeo-node/package.json`
- Create: `bindings/procgeo-node/src/lib.rs`
- Modify: root `Cargo.toml` (add to workspace members)

- [ ] Set up napi-rs crate:
  ```toml
  [package]
  name = "procgeo-node"
  version = "0.1.0"
  edition = "2024"

  [lib]
  crate-type = ["cdylib"]

  [dependencies]
  napi = { version = "2", features = ["napi6", "serde-json"] }
  napi-derive = "2"
  procgeo = { path = "../../crates/procgeo" }
  procgeo-core = { path = "../../crates/procgeo-core" }
  procgeo-sops = { path = "../../crates/procgeo-sops" }
  procgeo-io = { path = "../../crates/procgeo-io" }
  glam = { workspace = true }
  serde = { workspace = true }
  serde_json = "1.0"

  [build-dependencies]
  napi-build = "2"
  ```

- [ ] Create build.rs: `fn main() { napi_build::setup(); }`

- [ ] Create package.json with napi config

- [ ] Implement bindings in src/lib.rs:
  - `JsGeometry` class wrapping `Geometry` with napi attributes:
    - `num_points()`, `num_prims()`, `num_vertices()`
    - `point_pos(index: u32) -> Vec<f64>` (returns [x,y,z])
    - `bounding_box() -> Object` (returns {min: [x,y,z], max: [x,y,z]})
  - SOP functions that take/return JsGeometry:
    - `create_box(params?: BoxParamsJs) -> JsGeometry`
    - `create_grid(params?: GridParamsJs) -> JsGeometry`
    - `create_sphere(params?: SphereParamsJs) -> JsGeometry`
    - `create_line(params?: LineParamsJs) -> JsGeometry`
    - `create_circle(params?: CircleParamsJs) -> JsGeometry`
    - `create_tube(params?: TubeParamsJs) -> JsGeometry`
    - `create_torus(params?: TorusParamsJs) -> JsGeometry`
    - `transform(geo: &JsGeometry, params?: TransformParamsJs) -> JsGeometry`
    - `compute_normals(geo: &JsGeometry) -> JsGeometry`
    - `merge(geos: Vec<&JsGeometry>) -> JsGeometry`
    - `subdivide(geo: &JsGeometry, params?) -> JsGeometry`
    - `scatter(geo: &JsGeometry, params?) -> JsGeometry`
    - `copy_to_points(source: &JsGeometry, target: &JsGeometry) -> JsGeometry`
    - `poly_extrude(geo: &JsGeometry, params?) -> JsGeometry`
    - `write_obj(geo: &JsGeometry, path: String)`
    - `write_glb(geo: &JsGeometry, path: String)`
  - Params structs as serde-compatible JS objects

- [ ] Verify `cargo build -p procgeo-node` compiles
- [ ] Commit: "feat(bindings): add napi-rs TypeScript bindings"

---

### Task 4: Python Bindings (PyO3)

**Files:**
- Create: `bindings/procgeo-py/Cargo.toml`
- Create: `bindings/procgeo-py/pyproject.toml`
- Create: `bindings/procgeo-py/src/lib.rs`
- Modify: root `Cargo.toml` (add to workspace members)

- [ ] Set up PyO3 crate:
  ```toml
  [package]
  name = "procgeo-py"
  version = "0.1.0"
  edition = "2024"

  [lib]
  name = "procgeo"
  crate-type = ["cdylib"]

  [dependencies]
  pyo3 = { version = "0.23", features = ["extension-module"] }
  procgeo = { path = "../../crates/procgeo" }
  procgeo-core = { path = "../../crates/procgeo-core" }
  procgeo-sops = { path = "../../crates/procgeo-sops" }
  procgeo-io = { path = "../../crates/procgeo-io" }
  glam = { workspace = true }
  ```

- [ ] Create pyproject.toml for maturin build

- [ ] Implement bindings in src/lib.rs:
  - `Geometry` class (#[pyclass]) with methods:
    - `num_points()`, `num_prims()`, `num_vertices()`
    - `point_pos(index: usize) -> (f64, f64, f64)`
    - `bounding_box() -> ((f64,f64,f64), (f64,f64,f64))`
    - `__repr__` showing counts
  - Module functions:
    - `box_(**kwargs) -> Geometry` (size_x, size_y, size_z, center_x, center_y, center_z)
    - `grid(**kwargs) -> Geometry`
    - `sphere(**kwargs) -> Geometry`
    - `line(**kwargs) -> Geometry`
    - `circle(**kwargs) -> Geometry`
    - `tube(**kwargs) -> Geometry`
    - `torus(**kwargs) -> Geometry`
    - `transform(geo, **kwargs) -> Geometry`
    - `compute_normals(geo) -> Geometry`
    - `merge(geos: Vec<&Geometry>) -> Geometry` (takes a list)
    - `subdivide(geo, depth=1) -> Geometry`
    - `scatter(geo, count=100, seed=0) -> Geometry`
    - `copy_to_points(source, target) -> Geometry`
    - `poly_extrude(geo, **kwargs) -> Geometry`
    - `write_obj(geo, path)`
    - `write_glb(geo, path)`

- [ ] Verify `cargo build -p procgeo-py` compiles
- [ ] Commit: "feat(bindings): add PyO3 Python bindings"

---

### Task 5: Update umbrella crate and integration tests

- [ ] Forward new features through procgeo (color, gltf)
- [ ] Add to prelude: Color, Null, Resample, Smooth, Reverse, Clip SOPs
- [ ] Add integration tests:
  - test_full_workflow (box → subdivide → smooth → normal → color → write OBJ)
  - test_clip_and_measure (box → clip at y=0 → measure area)
  - test_reverse_normals (grid → normal → reverse → normal → verify flipped)
- [ ] Run `cargo test --workspace`
- [ ] Commit: "feat: Phase 3 umbrella exports and integration tests"
