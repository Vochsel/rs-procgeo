# ProcGeo Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the essential manipulation SOPs that enable real procedural workflows: attribute operations, groups, deletion, copy-to-points, subdivide, scatter, poly extrude, sort, fuse, connectivity, measure, and enumerate.

**Architecture:** All SOPs follow the existing Sop trait pattern. New feature flags added to procgeo-sops. Some SOPs require new utility functions on Geometry (e.g., point deletion, topology queries).

**Tech Stack:** Same as Phase 1. `rand` crate added for Scatter SOP randomization.

**Spec:** `docs/superpowers/specs/2026-04-06-procgeo-design.md`

---

## File Structure (new files only)

```
crates/
  procgeo-core/
    src/
      geometry.rs                    # Add point/prim deletion, topology helpers
  procgeo-sops/
    src/
      lib.rs                         # Add new feature gates
      attributes/
        mod.rs
        create.rs                    # Attribute Create SOP
        delete.rs                    # Attribute Delete SOP
        promote.rs                   # Attribute Promote SOP
        rename.rs                    # Attribute Rename SOP
      groups/
        mod.rs
        group_create.rs              # Group Create SOP
        group_combine.rs             # Group Combine SOP
      delete/
        mod.rs
        blast.rs                     # Blast SOP
        delete_sop.rs                # Delete SOP
      copy/
        mod.rs
        copy_to_points.rs            # Copy to Points SOP
      reshape/
        mod.rs
        subdivide.rs                 # Subdivide SOP (linear)
        poly_extrude.rs              # PolyExtrude SOP
      scatter/
        mod.rs
        scatter.rs                   # Scatter SOP
      topology/
        mod.rs
        sort.rs                      # Sort SOP
        fuse.rs                      # Fuse SOP
        connectivity.rs              # Connectivity SOP
      measure/
        mod.rs
        measure.rs                   # Measure SOP
      utility/
        mod.rs
        enumerate.rs                 # Enumerate SOP
```

---

### Task 1: Add rand dependency and new feature flags

**Files:**
- Modify: `Cargo.toml` (workspace)
- Modify: `crates/procgeo-sops/Cargo.toml`
- Modify: `crates/procgeo-sops/src/lib.rs`

- [ ] Add `rand = "0.9"` to workspace dependencies
- [ ] Add `rand` to procgeo-sops dependencies
- [ ] Add feature flags: `attributes`, `groups_sops`, `delete`, `copy`, `reshape`, `scatter`, `topology`, `measure_sops`, `utility_sops`
- [ ] Update default features to include all new flags
- [ ] Add conditional module declarations in lib.rs
- [ ] Create empty mod.rs stubs for each new module
- [ ] Verify `cargo build -p procgeo-sops` compiles
- [ ] Commit: "feat(sops): add Phase 2 feature flags and module stubs"

---

### Task 2: Attribute Create SOP

**Files:**
- Create: `crates/procgeo-sops/src/attributes/create.rs`
- Modify: `crates/procgeo-sops/src/attributes/mod.rs`

The Attribute Create SOP creates or overwrites an attribute on geometry.

- [ ] Implement AttribCreateParams:
  - `name: String` (default "attrib1")
  - `class: AttribClass` (default Point)
  - `attrib_type: AttribType` (default Float)
  - `value_int: i32` (default 0)
  - `value_float: f32` (default 0.0)
  - `value_vector3: [f32; 3]` (default [0,0,0])
  - `value_string: String` (default "")
  - `qualifier: TypeQualifier` (default None)

- [ ] Implement AttribCreateSop (1 input):
  - Clone input geometry
  - Based on attrib_type, create the appropriate AttribDefault and call add_attrib
  - If a non-default value is specified, set it on all elements

- [ ] Tests:
  - create_float_attrib (create "pscale" float on points, verify default value)
  - create_vector3_attrib (create "Cd" color, verify all points have value)
  - create_on_prims (create int on primitives)
  - create_on_detail (create string on detail)

- [ ] Commit: "feat(sops): add Attribute Create SOP"

---

### Task 3: Attribute Delete and Rename SOPs

**Files:**
- Create: `crates/procgeo-sops/src/attributes/delete.rs`
- Create: `crates/procgeo-sops/src/attributes/rename.rs`
- Modify: `crates/procgeo-sops/src/attributes/mod.rs`

- [ ] AttribDeleteSop: takes name + class, deletes the attribute. Params: name (String), class (AttribClass).
  - Tests: delete existing attrib, delete nonexistent (no error, just passes through)

- [ ] AttribRenameSop: renames an attribute. Params: from_name, to_name, class.
  - Implement by getting raw attribute, changing its name field, re-inserting in the map under new key, removing old key.
  - Tests: rename "Cd" to "color", verify old name gone and new name has same data

- [ ] Commit: "feat(sops): add Attribute Delete and Rename SOPs"

---

### Task 4: Attribute Promote SOP

**Files:**
- Create: `crates/procgeo-sops/src/attributes/promote.rs`
- Modify: `crates/procgeo-sops/src/attributes/mod.rs`

Promotes attributes between classes (e.g., point → prim, prim → detail). This is one of Houdini's most-used attribute SOPs.

- [ ] AttribPromoteParams:
  - `name: String`
  - `from_class: AttribClass`
  - `to_class: AttribClass`
  - `method: PromoteMethod` enum: First, Last, Min, Max, Average (default Average)
  - `delete_original: bool` (default true)

- [ ] Implement for Float and Vector3 types (the most common):
  - Point → Primitive: for each prim, gather values from its points, apply method
  - Point → Detail: gather all point values, apply method
  - Primitive → Point: for each point, find prims referencing it, gather values, apply method
  - Primitive → Detail: gather all prim values, apply method

- [ ] Tests:
  - promote_point_to_prim_average (grid with varying point float, verify prim gets average)
  - promote_point_to_detail (verify single detail value)
  - promote_with_delete_original (verify source attrib removed)

- [ ] Commit: "feat(sops): add Attribute Promote SOP"

---

### Task 5: Group Create and Group Combine SOPs

**Files:**
- Create: `crates/procgeo-sops/src/groups/group_create.rs`
- Create: `crates/procgeo-sops/src/groups/group_combine.rs`
- Modify: `crates/procgeo-sops/src/groups/mod.rs`

- [ ] GroupCreateParams:
  - `name: String` (default "group1")
  - `group_type: GroupType` enum: Points, Primitives (default Points)
  - `mode: GroupCreateMode` enum: Range, BoundingBox, Normal (default Range)
  - `range_start: usize`, `range_end: usize` — for Range mode
  - `bbox_min: Vec3`, `bbox_max: Vec3` — for BoundingBox mode
  - `normal_direction: Vec3`, `normal_angle: f32` — for Normal mode (angle in degrees)

- [ ] GroupCreateSop implementation:
  - Range: add elements with indices in [start, end) to group
  - BoundingBox: add points inside the bbox
  - Normal: add primitives whose face normal is within angle of direction

- [ ] Tests:
  - group_by_range (first 4 points of a grid)
  - group_by_bbox (points inside a region)
  - group_by_normal (top-facing prims of a box)

- [ ] GroupCombineParams:
  - `name_a: String`, `name_b: String`, `result: String`
  - `operation: GroupBooleanOp` enum: Union, Intersect, Subtract, Complement
  - `group_type: GroupType`

- [ ] GroupCombineSop: apply boolean op on two existing groups, store result
  - Tests: union, intersect, subtract of two point groups

- [ ] Commit: "feat(sops): add Group Create and Group Combine SOPs"

---

### Task 6: Blast and Delete SOPs

**Files:**
- Create: `crates/procgeo-sops/src/delete/blast.rs`
- Create: `crates/procgeo-sops/src/delete/delete_sop.rs`
- Modify: `crates/procgeo-sops/src/delete/mod.rs`

These SOPs need the ability to delete elements from Geometry. First, add helper methods to Geometry for rebuilding without certain elements.

- [ ] Add to Geometry a method: `rebuild_without_prims(prims_to_keep: &[bool]) -> Geometry`
  - Creates a new Geometry, copies only kept primitives and their referenced points
  - Remaps point indices
  - Also add `rebuild_without_points(points_to_keep: &[bool]) -> Geometry`
  - For points: remove prims that reference removed points

- [ ] BlastSop: delete elements by group name. Params: group_name (String), group_type (Points/Primitives), negate (bool — if true, keep only group members).
  - Tests: blast_by_prim_group (remove 2 faces from box), blast_negate (keep only group), blast_points (remove points and their prims)

- [ ] DeleteSop: delete by pattern/range. Params: entity_type (Points/Primitives), operation (ByRange/ByPattern), range_start, range_end, pattern (String for future expression support).
  - Tests: delete_prim_range (remove first 3 prims), delete_points_range

- [ ] Commit: "feat(sops): add Blast and Delete SOPs with geometry rebuild"

---

### Task 7: Copy to Points SOP

**Files:**
- Create: `crates/procgeo-sops/src/copy/copy_to_points.rs`
- Modify: `crates/procgeo-sops/src/copy/mod.rs`

The most powerful SOP — copies geometry onto every point of a target.

- [ ] CopyToPointsParams:
  - `use_template_point_attribs: bool` (default true — copy N, up, etc from target points)

- [ ] CopyToPointsSop (2 inputs: source geometry, target points):
  - For each point in input[1] (target), create a copy of input[0] (source)
  - Translate each copy to the target point's position
  - If target point has "N" attribute, orient the copy (align Y to N)
  - Merge all copies into one Geometry
  - Add "copynum" int point attribute tracking which copy each point belongs to

- [ ] Tests:
  - copy_box_to_line_points (box copied to 5 points along a line — 40 pts, 30 prims)
  - copy_preserves_topology (each copy has correct vertex count)
  - copy_with_normals (copies oriented when target has N attribute)

- [ ] Commit: "feat(sops): add Copy to Points SOP"

---

### Task 8: Subdivide SOP (Linear)

**Files:**
- Create: `crates/procgeo-sops/src/reshape/subdivide.rs`
- Modify: `crates/procgeo-sops/src/reshape/mod.rs`

Linear subdivision: split each quad into 4 quads, each triangle into 4 triangles.

- [ ] SubdivideParams:
  - `depth: u32` (default 1)

- [ ] SubdivideSop (1 input):
  - For each face: compute edge midpoints and face center, create 4 sub-faces for quads (3 for triangles)
  - Share edge midpoints between adjacent faces (use a HashMap<(min_pt, max_pt), PointHandle> to deduplicate)
  - Apply recursively for depth > 1

- [ ] Tests:
  - subdivide_quad (single quad → 4 quads, 9 points)
  - subdivide_triangle (single tri → 4 tris, 6 points)
  - subdivide_box (box → 24 quads at depth 1)
  - subdivide_depth_2 (single quad at depth 2 → 16 quads)

- [ ] Commit: "feat(sops): add Subdivide SOP with linear subdivision"

---

### Task 9: Scatter SOP

**Files:**
- Create: `crates/procgeo-sops/src/scatter/scatter.rs`
- Modify: `crates/procgeo-sops/src/scatter/mod.rs`

Scatter random points on the surface of a mesh.

- [ ] ScatterParams:
  - `count: u32` (default 100)
  - `seed: u64` (default 0)
  - `relax_iterations: u32` (default 0, for future use)

- [ ] ScatterSop (1 input):
  - Compute area of each face (triangle fan from first vertex)
  - Build CDF (cumulative distribution) weighted by face area
  - For each scatter point: pick a face from CDF, generate random barycentric coords, compute position
  - Create new Geometry with just the scattered points (no prims)
  - Add "sourceprim" int point attribute tracking which face each point came from

- [ ] Tests:
  - scatter_on_grid (100 points on a grid, all within grid bbox)
  - scatter_count (verify exact point count)
  - scatter_deterministic (same seed → same positions)
  - scatter_area_weighted (scatter on mesh with varying face sizes, verify more points on larger faces)

- [ ] Commit: "feat(sops): add Scatter SOP with area-weighted distribution"

---

### Task 10: PolyExtrude SOP

**Files:**
- Create: `crates/procgeo-sops/src/reshape/poly_extrude.rs`
- Modify: `crates/procgeo-sops/src/reshape/mod.rs`

Extrude faces along their normals.

- [ ] PolyExtrudeParams:
  - `distance: f32` (default 1.0)
  - `inset: f32` (default 0.0)
  - `output_front: bool` (default true)
  - `output_side: bool` (default true)

- [ ] PolyExtrudeSop (1 input):
  - For each face: compute face normal, duplicate face points, offset by distance * normal
  - If inset > 0: move duplicated points toward face center
  - Create side quads connecting original edges to extruded edges
  - Optionally create front face (the extruded top)
  - Keep original face as back (or remove it)

- [ ] Tests:
  - extrude_single_quad (1 quad → 1 top + 4 sides = 5 faces, 8 pts)
  - extrude_box (6 faces → each gets 4 sides + top = 30 new faces)
  - extrude_with_inset (verify top face is smaller)
  - extrude_distance (verify extruded points are at correct offset)

- [ ] Commit: "feat(sops): add PolyExtrude SOP"

---

### Task 11: Sort, Fuse, Connectivity SOPs

**Files:**
- Create: `crates/procgeo-sops/src/topology/sort.rs`
- Create: `crates/procgeo-sops/src/topology/fuse.rs`
- Create: `crates/procgeo-sops/src/topology/connectivity.rs`
- Modify: `crates/procgeo-sops/src/topology/mod.rs`

- [ ] SortSop: reorder points or primitives. SortParams: entity (Points/Prims), mode (ByAxis, Reverse, Random), axis (X/Y/Z for ByAxis), seed (u64 for Random).
  - Rebuild geometry with elements in new order
  - Tests: sort_points_by_x (verify ascending X), sort_reverse, sort_prims_by_axis

- [ ] FuseSop: merge points within a distance tolerance. FuseParams: distance (f32, default 0.001).
  - For each point, find all points within distance. Use spatial hashing or brute force for now.
  - Merge: keep first point, remap all vertex references to the kept point, remove duplicate points.
  - Tests: fuse_coincident (two boxes at same position → 8 points instead of 16), fuse_tolerance (points just outside tolerance not fused)

- [ ] ConnectivitySop: assign a "class" integer attribute based on connected components. ConnectivityParams: attrib_name (String, default "class"), class (AttribClass, default Primitive).
  - Build adjacency graph (prims sharing points), flood fill to find components, assign class IDs
  - Tests: connectivity_single_mesh (all same class), connectivity_two_boxes (two separate boxes → two classes)

- [ ] Commit: "feat(sops): add Sort, Fuse, and Connectivity SOPs"

---

### Task 12: Measure and Enumerate SOPs

**Files:**
- Create: `crates/procgeo-sops/src/measure/measure.rs`
- Create: `crates/procgeo-sops/src/utility/enumerate.rs`
- Modify: `crates/procgeo-sops/src/measure/mod.rs`
- Modify: `crates/procgeo-sops/src/utility/mod.rs`

- [ ] MeasureSop: compute geometric measurements. MeasureParams: type (Area, Perimeter, Curvature), attrib_name (String, defaults to "area"/"perimeter").
  - Area mode: compute area of each face, store as prim float attribute. Also compute total as detail attrib.
  - Perimeter mode: compute perimeter of each face.
  - Tests: measure_area_quad (1x1 quad = area 1.0), measure_area_box (6 faces, total area 6.0), measure_perimeter

- [ ] EnumerateSop: add sequential index attribute. EnumerateParams: name (String, default "index"), class (AttribClass, default Point), start (i32, default 0).
  - Tests: enumerate_points (0,1,2,...), enumerate_prims, enumerate_with_offset (start=10)

- [ ] Commit: "feat(sops): add Measure and Enumerate SOPs"

---

### Task 13: Update umbrella crate and integration tests

**Files:**
- Modify: `crates/procgeo/Cargo.toml`
- Modify: `crates/procgeo/src/lib.rs`
- Modify: `crates/procgeo/tests/integration.rs`

- [ ] Forward all new features from procgeo-sops through procgeo
- [ ] Add new modules to prelude
- [ ] Add integration tests:
  - test_scatter_on_subdivided_grid (grid → subdivide → scatter → verify points within bbox)
  - test_copy_to_scattered_points (grid → scatter 10 pts → copy box to those points)
  - test_extrude_then_measure (box → extrude all faces → measure areas)
  - test_blast_by_group (box → create group of top face → blast → 5 faces remain)
  - test_enumerate_and_promote (enumerate points → promote to detail → verify count)

- [ ] Run `cargo test --workspace`
- [ ] Commit: "feat: Phase 2 umbrella exports and integration tests"
