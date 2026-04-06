# ProcGeo Design Spec

A Rust library for procedural geometry creation, modeled as a 1:1 recreation of Houdini SOPs. TypeScript (napi-rs) and Python (PyO3) bindings expose the full SOP catalog. Performance-optimized with SIMD, SoA layouts, and cache-friendly data structures.

## Workspace Structure

```
rs-procgeo/
├── crates/
│   ├── procgeo-core/       # Geometry model, attributes, groups, spatial indexing
│   ├── procgeo-sops/       # All SOP implementations (feature-gated by category)
│   ├── procgeo-io/         # Export/import trait + format plugins
│   └── procgeo/            # Umbrella re-export crate
├── bindings/
│   ├── procgeo-node/       # napi-rs TypeScript bindings (@procgeo/core)
│   └── procgeo-py/         # PyO3 Python bindings (procgeo)
├── docs/
└── tests/                  # Integration tests, geometry validation
```

Cargo workspace at the root. Each crate has focused responsibility.

---

## 1. Core Geometry Model (`procgeo-core`)

### Geometry Container

`Geometry` is the central data structure. It owns all points, vertices, primitives, and attributes.

```rust
pub struct Geometry {
    points: PointStorage,
    vertices: VertexStorage,
    primitives: PrimStorage,
    attributes: AttributeMap,
    groups: GroupMap,
    detail: DetailAttributes,
    spatial_cache: SpatialCache,  // lazily-built BVH/KD-tree
}
```

### Points

- Stored in SoA layout: separate `Vec<f32>` for x, y, z components of position `P`
- Optional weight `Pw` for rational curves/NURBS
- Unique index per point. Deletion uses a free-list with periodic compaction
- `PointHandle(u32)` typed handle for safe referencing

### Vertices

- A vertex references a point (`PointHandle`) and belongs to a primitive (`PrimHandle`)
- Multiple vertices can share the same point (Houdini semantics)
- Vertex order within a primitive defines winding
- `VertexHandle(u32)` typed handle
- Stored contiguously, grouped by owning primitive for cache locality

### Primitives

Enum-dispatched for performance (no vtable overhead):

```rust
pub enum Primitive {
    Polygon(PolygonPrim),         // open/closed, arbitrary vertex count
    Mesh(MeshPrim),               // structured grid (rows × cols)
    NURBSCurve(NURBSCurvePrim),
    NURBSSurface(NURBSSurfacePrim),
    BezierCurve(BezierCurvePrim),
    BezierSurface(BezierSurfacePrim),
    Metaball(MetaballPrim),
    MetaSuperQuad(MetaSuperQuadPrim),
    Sphere(SpherePrim),           // parametric (transform-based)
    Tube(TubePrim),               // parametric
    Circle(CirclePrim),           // parametric
    Volume(VolumePrim),           // voxel grid
    VDB(VDBPrim),                 // OpenVDB sparse volume
    Packed(PackedPrim),           // reference to another Geometry
    Tetra(TetraPrim),             // tetrahedral element
    PolySoup(PolySoupPrim),       // shared-point polygon soup
}
```

Per-primitive vertex lists use `SmallVec<[VertexHandle; 4]>` — most prims have 3-4 verts, avoiding heap allocation in the common case.

### Attribute System

Four attribute classes matching Houdini:

| Class | Scope | One value per... |
|-------|-------|-----------------|
| `Point` | Point attributes | point |
| `Vertex` | Vertex attributes | vertex |
| `Primitive` | Primitive attributes | primitive |
| `Detail` | Global attributes | geometry (single value) |

Typed storage via enum with SoA layout:

```rust
pub enum AttributeStorage {
    Int(Vec<i32>),
    Int64(Vec<i64>),
    Float(Vec<f32>),
    Float64(Vec<f64>),
    Vector2(Vec<[f32; 2]>),
    Vector3(Vec<[f32; 3]>),
    Vector4(Vec<[f32; 4]>),
    Matrix3(Vec<[f32; 9]>),
    Matrix4(Vec<[f32; 16]>),
    String(StringTable),          // interned string storage
    IntArray(Vec<Vec<i32>>),      // variable-length arrays
    FloatArray(Vec<Vec<f32>>),
    StringArray(Vec<Vec<u32>>),   // indices into StringTable
    Dict(Vec<HashMap<String, AttributeValue>>),
}
```

Each attribute has:
- Name (`String`)
- Type info (`AttributeType` enum)
- Default value (used when adding new elements)
- Storage class (`AttributeClass`)
- Type qualifiers: `none`, `point`, `vector`, `normal`, `color`, `quaternion`, `matrix` (affects transform behavior)

**Typed attribute handles** for compile-time access:

```rust
let handle: AttribHandle<Vector3> = geo.find_attrib::<Vector3>(AttribClass::Point, "N")?;
let normal: &[f32; 3] = geo.get_attrib(handle, point);
```

### Groups

Four group types matching Houdini:

- **Point groups** — `BitVec` sized to point count
- **Primitive groups** — `BitVec` sized to prim count
- **Vertex groups** — `BitVec` sized to vertex count
- **Edge groups** — `HashSet<(PrimHandle, u8)>` storing (prim, local_edge_index) pairs

All groups are named. Boolean operations (union, intersect, subtract, complement) operate on whole bitset words for speed.

```rust
geo.create_point_group("selected");
geo.point_group_mut("selected").set(pt, true);
for pt in geo.points_in_group("selected") { ... }
```

### Iterators

```rust
// Basic iteration
geo.points()              // all points
geo.prims()               // all primitives
geo.vertices()            // all vertices

// Topology traversal
prim.vertices(&geo)       // vertices of a primitive
vertex.point(&geo)        // point a vertex references
point.vertices(&geo)      // all vertices referencing this point

// Filtered by group
geo.points_in_group("selected")
geo.prims_in_group("walls")

// Parallel (via rayon)
geo.par_points().for_each(|pt| { ... });
```

---

## 1b. Math (`glam` + `procgeo-core::math`)

Uses `glam` as the math foundation — no custom math crate.

**From glam (re-exported):**
- `Vec2`, `Vec3`, `Vec4` (f32) and `DVec2`, `DVec3`, `DVec4` (f64)
- `Mat3`, `Mat4`, `DMat4`
- `Quat`, `DQuat`
- `Affine3A` (SIMD-aligned affine transform)
- All SIMD-accelerated (SSE2/SSE4.1/NEON) out of the box

**In `procgeo-core::math` (geometry-specific utilities glam doesn't cover):**
- `BBox` — axis-aligned bounding box with min/max, expand, contains, intersect
- Noise functions: Perlin, Simplex, Worley/Voronoi, curl noise — matching VEX `noise()` signatures
- `fit()`, `efit()`, `smooth()`, `ease()` — Houdini-style interpolation helpers
- Batch SIMD operations: transform N points by a matrix, batch normalize, batch distance (using `wide` crate for 4/8-wide processing over arrays, complementing glam's per-element SIMD)

---

## 2. Performance Architecture

### SoA / Columnar Layout

All attribute data is stored column-wise. Position `P` is stored as three separate `Vec<f32>` (x, y, z) enabling SIMD processing of each component independently.

### SIMD

- Per-element: `glam` provides SIMD-accelerated vector/matrix ops (SSE2/SSE4.1/NEON) automatically
- Batch processing: `wide` crate for 4/8-wide operations over arrays of data
- Key batch SIMD paths:
  - Transform N points by a matrix in one pass
  - Batch normalize, batch distance, bounding box over point arrays
  - Attribute arithmetic (add, multiply, lerp over contiguous `Vec<f32>` storage)
- `wide` operates on 4/8 elements per iteration with scalar remainder

### Allocation

- `bumpalo` bump allocator for temporary per-SOP scratch data
- `Geometry::with_capacity(points, prims)` pre-allocates storage
- `SmallVec<[VertexHandle; 4]>` for per-primitive vertex lists
- String interning via `StringTable` — deduplicates string attribute values

### Spatial Indexing

Built lazily on first query, cached on `Geometry`, invalidated on mutation:

- **BVH** (bounding volume hierarchy) — ray casting, overlap queries, used by Ray SOP, Boolean
- **KD-tree** — nearest-neighbor queries, used by Fuse, Scatter, Point Cloud SOPs, Proximity
- Built using `rstar` crate or custom implementation for tighter control

### Parallelism

- `rayon` for data-parallel iteration over points/prims/vertices
- Auto-parallelization threshold: SOPs parallelize when element count exceeds ~10k
- Thread-local scratch buffers via `rayon::ThreadLocal` to avoid contention
- Independent SOP branches in future graph evaluation cook concurrently

---

## 3. SOP Layer (`procgeo-sops`)

### SOP Trait

```rust
pub trait Sop {
    type Params: Default;

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError>;
    fn input_count(&self) -> (usize, usize); // (min_inputs, max_inputs)
    fn name(&self) -> &'static str;
}
```

- Stateless — all state in `Params` and input `Geometry`
- Functional — returns a new `Geometry` (no mutation of inputs)
- `execute_in_place(&self, geo: &mut Geometry, params: &Self::Params)` escape hatch for perf-critical SOPs

### Composability

```rust
let geo = Geometry::new()
    .apply(BoxSop, BoxParams { size: vec3(1.0, 1.0, 1.0), ..default() })
    .apply(SubdivideSop, SubdivideParams { depth: 2, ..default() })
    .apply(NormalSop, NormalParams::default());
```

### SOP Params

Each SOP's `Params` struct mirrors Houdini's parameter interface:

- All fields have `Default` matching Houdini's defaults
- Enums for mode/dropdown parameters (e.g., `BooleanOp::Union | Intersect | Subtract`)
- `#[derive(Clone, Debug, Serialize, Deserialize)]` on all params for binding serialization

### Feature-Gated Categories

Each category is a Cargo feature in `procgeo-sops`:

| Feature | Category | Node Count |
|---------|----------|-----------|
| `creation` | Box, Sphere, Grid, Tube, Circle, Line, Curve, Font, Torus, etc. | 27 |
| `attributes` | Attribute Create, Copy, Transfer, Wrangle, Promote, Delete, etc. | 38 |
| `transform` | Transform, Mirror, Match Size, Extract Centroid, etc. | 12 |
| `groups` | Group, Group Combine, Group Expression, Group Promote, etc. | 17 |
| `topology` | Fuse, Divide, Remesh, Sort, Clean, Connectivity, Dissolve, etc. | 31 |
| `normals` | Normal, Facet, Comb | 3 |
| `copy` | Copy to Points, Copy and Transform, Copy to Curves, etc. | 5 |
| `scatter` | Scatter, Scatter and Align, Spray Paint | 4 |
| `delete` | Blast, Delete, Split, Clip, Separate Pieces | 6 |
| `polygon` | PolyExtrude, PolyBevel, PolyFill, PolyBridge, PolyReduce, etc. | 17 |
| `boolean` | Boolean, Boolean Fracture | 2 |
| `reshape` | Bend, Subdivide, Smooth, Lattice, Shrinkwrap, Carve, etc. | 37 |
| `edges` | Edge Collapse, Divide, Flip, Fracture, Relax, etc. | 9 |
| `measure` | Measure, Distance, Shortest Path, Winding Number, etc. | 16 |
| `uvs` | UV Project, Flatten, Layout, Unwrap, etc. | 15 |
| `vdb` | VDB ops (requires OpenVDB C bindings) | 36 |
| `volumes` | Volume operations | 51 |
| `heightfield` | Terrain/HeightField | 43 |
| `curves` | Spline ops | 13 |
| `voronoi` | Voronoi Fracture, Split, Adjacency | 4 |
| `packing` | Pack, Unpack, Repack, etc. | 13 |
| `points` | Point Cloud, Point Generate, Point Deform, etc. | 15 |
| `color` | Color, Material, Rest Position | 12 |
| `merge` | Merge, Assemble, Connect Adjacent Pieces | 4 |
| `utility` | Null, Switch, Cache, TimeShift, Enumerate, etc. | 33 |
| `io_nodes` | File, Alembic, glTF import nodes | 38 |
| `vellum` | Vellum simulation | 16 |
| `rbd` | RBD destruction | 30 |
| `pyro` | Pyro/fire simulation | 13 |
| `fluids` | Fluid/ocean simulation | 22 |
| `crowds` | Crowd simulation/agents | 35 |
| `kinefx` | KineFX rigging | 48 |
| `hair` | Hair/grooming | 31 |
| `muscles` | Muscle simulation | 29 |
| `fem` | Finite elements | 7 |
| `clouds` | Clouds/sky | 16 |
| `feathers` | Feathers | 23 |
| `motion` | Motion clips/dynamics | 23 |
| `ml` | Machine learning | 19 |
| `labs` | SideFX Labs | 217 |

Default features: `creation`, `transform`, `attributes`, `groups`, `topology`, `normals`, `copy`, `scatter`, `delete`, `polygon`, `merge`, `utility`, `points`, `color`, `edges`, `measure`.

Total: **~1,155 SOPs** across **~40 feature categories**.

### Expression / Wrangle System

For Attribute Wrangle, Group Expression, Volume Wrangle, and similar SOPs:

- Simple expression evaluator supporting VEX-like syntax
- Per-element execution with attribute access via `@name` syntax
- Local variables: `@ptnum`, `@primnum`, `@numpt`, `@numprim`, `@Time`, `@Frame`
- Built-in functions: `sin`, `cos`, `tan`, `sqrt`, `pow`, `abs`, `floor`, `ceil`, `round`, `fit`, `lerp`, `clamp`, `rand`, `noise`, `length`, `normalize`, `dot`, `cross`, `set`, `vector`, `matrix`
- Assignment: `@P.y = sin(@P.x * 2.0);`, `@Cd = {1, 0, 0};`
- Compiled to a simple bytecode VM for fast per-element execution

---

## 4. I/O Layer (`procgeo-io`)

### Plugin Trait

```rust
pub trait GeometryWriter {
    fn write(&self, geo: &Geometry, writer: &mut dyn Write) -> Result<(), IoError>;
    fn extensions(&self) -> &[&str];
}

pub trait GeometryReader {
    fn read(&self, reader: &mut dyn Read) -> Result<Geometry, IoError>;
    fn extensions(&self) -> &[&str];
}
```

### Built-in Formats (feature-gated)

| Feature | Format | Read | Write | Notes |
|---------|--------|------|-------|-------|
| `obj` | Wavefront OBJ | Yes | Yes | Mesh-only, universal |
| `gltf` | glTF 2.0 / GLB | Yes | Yes | Full mesh + materials, via `gltf` crate |

### Future Format Crates

- `procgeo-io-usd` — USD/USDA/USDC (depends on OpenUSD C++ libs)
- `procgeo-io-houdini` — `.geo` (JSON) / `.bgeo` (binary)
- `procgeo-io-ply` — PLY for point clouds

### Format Registry

```rust
let registry = IoRegistry::default(); // auto-registers enabled formats
registry.write_file(&geo, "output.glb")?;
let geo = registry.read_file("input.obj")?;
```

Extension-based dispatch. Custom formats register via `registry.register_writer(MyFormat)`.

---

## 5. Bindings

### TypeScript via napi-rs (`bindings/procgeo-node/`)

- Package: `@procgeo/core` on npm
- Every SOP exposed as a function matching its Houdini name
- `Geometry` class wrapping Rust struct, exposed to JS
- Params as plain JS objects (napi-rs serde conversion)
- Chainable API: `new Geometry().apply(box()).apply(subdivide({ depth: 2 }))`
- Export: `writeObj(geo, path)`, `writeGltf(geo, path)`
- TypeScript types auto-generated from Rust param structs

### Python via PyO3 (`bindings/procgeo-py/`)

- Package: `procgeo` on PyPI
- Pythonic naming: `attribute_create()`, `copy_to_points()`
- `Geometry` class with `__repr__` showing point/prim counts
- NumPy integration for bulk attribute access (zero-copy via buffer protocol)
- Built with maturin, installable via `pip install procgeo` or `uv pip install procgeo`

### Binding Principles

- Both bindings depend on the `procgeo` umbrella crate
- Thin wrappers only — convert language-native types to Rust params, call SOP, wrap result
- No business logic in bindings
- Error handling: Rust `SopError` maps to JS/Python exceptions with clear messages

---

## 6. Testing Strategy

### Unit Tests (per crate)

- `procgeo-core`: geometry creation, attribute CRUD, group operations, topology traversal
- `procgeo-core::math`: noise functions, batch SIMD ops, BBox
- `procgeo-sops`: each SOP tested individually with known inputs/expected outputs

### Geometry Validation Tests

For each SOP, validate against Houdini reference data:

- Point count, primitive count, vertex count
- Attribute values (within float tolerance)
- Topology (vertex-point connectivity)
- Group membership
- Bounding box

Test pattern:
```rust
#[test]
fn test_box_default() {
    let geo = BoxSop.execute(&[], &BoxParams::default()).unwrap();
    assert_eq!(geo.num_points(), 8);
    assert_eq!(geo.num_prims(), 6);
    assert_eq!(geo.num_vertices(), 24);
    // Check bounding box is unit cube centered at origin
    let bbox = geo.bounding_box();
    assert_approx_eq!(bbox.min, [-0.5, -0.5, -0.5]);
    assert_approx_eq!(bbox.max, [0.5, 0.5, 0.5]);
}
```

### Integration Tests

- SOP chaining: multi-step workflows producing correct geometry
- I/O round-trip: write → read → compare geometry
- Binding tests: TS (vitest) and Python (pytest) calling the same SOP chains

### Property-Based Tests

- `proptest` for fuzz-testing attribute operations, group boolean logic
- Random geometry → SOP → validate invariants (no NaN positions, valid topology, consistent attribute sizes)

---

## 7. Future: Node Graph (`procgeo-graph`)

The stateless `Sop` trait is designed to support a future node graph crate:

```rust
let mut graph = SopGraph::new();
let box_n = graph.add(BoxSop, BoxParams::default());
let subdiv_n = graph.add(SubdivideSop, SubdivideParams { depth: 2, .. });
graph.connect(box_n, 0, subdiv_n, 0);
let result = graph.evaluate(subdiv_n)?;
```

Features for later:
- Topological sort evaluation
- Per-node output caching (dirty propagation on param/input change)
- Lazy evaluation (only cook upstream of requested node)
- Parallel branch cooking
- Graph serialization (save/load networks)

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `glam` | Vector/matrix/quat math (SIMD-accelerated) |
| `wide` | Batch SIMD over arrays (f32x4, f32x8) |
| `rayon` | Data parallelism |
| `smallvec` | Inline vertex lists |
| `bumpalo` | Bump allocation for scratch data |
| `bitvec` | Bitset-based groups |
| `rstar` | R-tree spatial indexing |
| `gltf` | glTF 2.0 read/write |
| `serde` | Param serialization |
| `thiserror` | Error types |
| `napi` / `napi-derive` | TypeScript bindings |
| `pyo3` | Python bindings |
| `maturin` | Python build tool |
| `numpy` (PyO3) | NumPy buffer protocol |
| `approx` | Float comparison in tests |
| `proptest` | Property-based testing |
