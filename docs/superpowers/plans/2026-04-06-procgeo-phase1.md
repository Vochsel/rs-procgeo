# ProcGeo Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the core geometry model, SOP trait, creation SOPs (Box, Grid, Line, Circle, Sphere, Tube, Torus), Normal SOP, Transform SOP, Merge SOP, and OBJ export — a working foundation for all future SOP development.

**Architecture:** Cargo workspace with `procgeo-core` (geometry model + attributes + groups), `procgeo-sops` (SOP trait + implementations), `procgeo-io` (export), and `procgeo` (umbrella). All data stored SoA for SIMD-friendliness. SOPs are stateless functions taking `&[&Geometry]` and returning `Result<Geometry>`.

**Tech Stack:** Rust (edition 2024), glam (SIMD math), smallvec (inline vertex lists), bitvec (groups), serde (param serialization), thiserror (errors), approx (test float comparison)

**Spec:** `docs/superpowers/specs/2026-04-06-procgeo-design.md`

---

## File Structure

```
Cargo.toml                          # workspace root
crates/
  procgeo-core/
    Cargo.toml
    src/
      lib.rs                        # re-exports, Geometry struct
      handle.rs                     # PointHandle, VertexHandle, PrimHandle
      point.rs                      # PointStorage (SoA: x, y, z vecs)
      vertex.rs                     # VertexStorage
      primitive.rs                  # Primitive enum + PrimStorage
      attribute.rs                  # AttributeStorage, AttributeMap, AttribHandle
      group.rs                      # PointGroup, PrimGroup, VertexGroup, EdgeGroup
      math/
        mod.rs                      # re-exports
        bbox.rs                     # BBox (axis-aligned bounding box)
      error.rs                      # CoreError
  procgeo-sops/
    Cargo.toml
    src/
      lib.rs                        # Sop trait, SopError, apply() chain
      creation/
        mod.rs                      # re-exports
        box_sop.rs                  # Box SOP
        grid.rs                     # Grid SOP
        line.rs                     # Line SOP
        circle.rs                   # Circle SOP
        sphere.rs                   # Sphere SOP
        tube.rs                     # Tube SOP
        torus.rs                    # Torus SOP
      transform/
        mod.rs
        transform_sop.rs            # Transform SOP
      normals/
        mod.rs
        normal.rs                   # Normal SOP
      merge/
        mod.rs
        merge.rs                    # Merge SOP
  procgeo-io/
    Cargo.toml
    src/
      lib.rs                        # GeometryWriter/Reader traits, IoRegistry
      obj.rs                        # OBJ writer + reader
  procgeo/
    Cargo.toml
    src/
      lib.rs                        # umbrella re-exports
```

---

### Task 1: Workspace and Cargo Setup

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/procgeo-core/Cargo.toml`
- Create: `crates/procgeo-sops/Cargo.toml`
- Create: `crates/procgeo-io/Cargo.toml`
- Create: `crates/procgeo/Cargo.toml`

- [ ] **Step 1: Create workspace root Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/procgeo-core",
    "crates/procgeo-sops",
    "crates/procgeo-io",
    "crates/procgeo",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
repository = "https://github.com/vochsel/rs-procgeo"

[workspace.dependencies]
glam = "0.29"
smallvec = { version = "1.13", features = ["serde"] }
bitvec = "1.0"
serde = { version = "1.0", features = ["derive"] }
thiserror = "2.0"
approx = "0.5"
```

- [ ] **Step 2: Create procgeo-core Cargo.toml**

```toml
[package]
name = "procgeo-core"
version.workspace = true
edition.workspace = true

[dependencies]
glam = { workspace = true }
smallvec = { workspace = true }
bitvec = { workspace = true }
serde = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
approx = { workspace = true }
```

- [ ] **Step 3: Create procgeo-sops Cargo.toml**

```toml
[package]
name = "procgeo-sops"
version.workspace = true
edition.workspace = true

[dependencies]
procgeo-core = { path = "../procgeo-core" }
glam = { workspace = true }
serde = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
approx = { workspace = true }

[features]
default = ["creation", "transform", "normals", "merge"]
creation = []
transform = []
normals = []
merge = []
```

- [ ] **Step 4: Create procgeo-io Cargo.toml**

```toml
[package]
name = "procgeo-io"
version.workspace = true
edition.workspace = true

[dependencies]
procgeo-core = { path = "../procgeo-core" }
thiserror = { workspace = true }

[dev-dependencies]
approx = { workspace = true }
procgeo-sops = { path = "../procgeo-sops" }

[features]
default = ["obj"]
obj = []
```

- [ ] **Step 5: Create procgeo umbrella Cargo.toml**

```toml
[package]
name = "procgeo"
version.workspace = true
edition.workspace = true

[dependencies]
procgeo-core = { path = "../procgeo-core" }
procgeo-sops = { path = "../procgeo-sops" }
procgeo-io = { path = "../procgeo-io" }
```

- [ ] **Step 6: Create stub lib.rs files for each crate**

Create minimal `src/lib.rs` for each crate so the workspace compiles:

`crates/procgeo-core/src/lib.rs`:
```rust
pub mod error;
```

`crates/procgeo-core/src/error.rs`:
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("attribute '{name}' not found")]
    AttributeNotFound { name: String },
    #[error("attribute '{name}' type mismatch: expected {expected}, got {got}")]
    AttributeTypeMismatch { name: String, expected: String, got: String },
    #[error("group '{name}' not found")]
    GroupNotFound { name: String },
    #[error("invalid handle: index {index} out of bounds (count: {count})")]
    InvalidHandle { index: u32, count: u32 },
    #[error("invalid topology: {0}")]
    InvalidTopology(String),
}
```

`crates/procgeo-sops/src/lib.rs`:
```rust
// stub
```

`crates/procgeo-io/src/lib.rs`:
```rust
// stub
```

`crates/procgeo/src/lib.rs`:
```rust
pub use procgeo_core as core;
pub use procgeo_sops as sops;
pub use procgeo_io as io;
```

- [ ] **Step 7: Verify workspace compiles**

Run: `cargo build`
Expected: Compiles with no errors.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: initialize cargo workspace with procgeo crate structure"
```

---

### Task 2: Core Handles

**Files:**
- Create: `crates/procgeo-core/src/handle.rs`
- Modify: `crates/procgeo-core/src/lib.rs`

- [ ] **Step 1: Write handle tests in handle.rs**

```rust
// crates/procgeo-core/src/handle.rs

use std::fmt;

/// Handle to a point in a Geometry.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PointHandle(pub(crate) u32);

/// Handle to a vertex in a Geometry.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VertexHandle(pub(crate) u32);

/// Handle to a primitive in a Geometry.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrimHandle(pub(crate) u32);

macro_rules! impl_handle {
    ($name:ident) => {
        impl $name {
            /// Create a handle from a raw index.
            #[inline]
            pub fn from_index(index: usize) -> Self {
                Self(index as u32)
            }

            /// Get the raw index.
            #[inline]
            pub fn index(self) -> usize {
                self.0 as usize
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

impl_handle!(PointHandle);
impl_handle!(VertexHandle);
impl_handle!(PrimHandle);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_from_index() {
        let h = PointHandle::from_index(42);
        assert_eq!(h.index(), 42);
    }

    #[test]
    fn test_handle_equality() {
        let a = PrimHandle::from_index(5);
        let b = PrimHandle::from_index(5);
        let c = PrimHandle::from_index(6);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_handle_ordering() {
        let a = VertexHandle::from_index(1);
        let b = VertexHandle::from_index(3);
        assert!(a < b);
    }

    #[test]
    fn test_handle_debug() {
        let h = PointHandle::from_index(7);
        assert_eq!(format!("{:?}", h), "PointHandle(7)");
    }
}
```

- [ ] **Step 2: Export handles from lib.rs**

Update `crates/procgeo-core/src/lib.rs`:
```rust
pub mod error;
pub mod handle;

pub use handle::{PointHandle, PrimHandle, VertexHandle};
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p procgeo-core`
Expected: All 4 handle tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/procgeo-core/src/handle.rs crates/procgeo-core/src/lib.rs
git commit -m "feat(core): add typed handles for points, vertices, primitives"
```

---

### Task 3: Point Storage (SoA)

**Files:**
- Create: `crates/procgeo-core/src/point.rs`
- Modify: `crates/procgeo-core/src/lib.rs`

- [ ] **Step 1: Implement PointStorage with SoA layout**

```rust
// crates/procgeo-core/src/point.rs

use crate::handle::PointHandle;
use glam::Vec3;

/// SoA storage for point positions.
/// Stores x, y, z in separate contiguous vectors for SIMD-friendly access.
pub struct PointStorage {
    pub(crate) x: Vec<f32>,
    pub(crate) y: Vec<f32>,
    pub(crate) z: Vec<f32>,
    count: u32,
}

impl PointStorage {
    pub fn new() -> Self {
        Self {
            x: Vec::new(),
            y: Vec::new(),
            z: Vec::new(),
            count: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            x: Vec::with_capacity(capacity),
            y: Vec::with_capacity(capacity),
            z: Vec::with_capacity(capacity),
            count: 0,
        }
    }

    /// Add a point, returns its handle.
    pub fn add(&mut self, pos: Vec3) -> PointHandle {
        let handle = PointHandle(self.count);
        self.x.push(pos.x);
        self.y.push(pos.y);
        self.z.push(pos.z);
        self.count += 1;
        handle
    }

    /// Get position of a point.
    #[inline]
    pub fn position(&self, handle: PointHandle) -> Vec3 {
        let i = handle.index();
        Vec3::new(self.x[i], self.y[i], self.z[i])
    }

    /// Set position of a point.
    #[inline]
    pub fn set_position(&mut self, handle: PointHandle, pos: Vec3) {
        let i = handle.index();
        self.x[i] = pos.x;
        self.y[i] = pos.y;
        self.z[i] = pos.z;
    }

    /// Number of points.
    #[inline]
    pub fn len(&self) -> usize {
        self.count as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Iterate over all point handles.
    pub fn iter(&self) -> impl Iterator<Item = PointHandle> {
        (0..self.count).map(PointHandle)
    }

    /// Get raw x slice (for SIMD batch ops).
    pub fn x_slice(&self) -> &[f32] {
        &self.x
    }

    /// Get raw y slice (for SIMD batch ops).
    pub fn y_slice(&self) -> &[f32] {
        &self.y
    }

    /// Get raw z slice (for SIMD batch ops).
    pub fn z_slice(&self) -> &[f32] {
        &self.z
    }

    /// Reserve capacity for additional points.
    pub fn reserve(&mut self, additional: usize) {
        self.x.reserve(additional);
        self.y.reserve(additional);
        self.z.reserve(additional);
    }

    /// Clear all points.
    pub fn clear(&mut self) {
        self.x.clear();
        self.y.clear();
        self.z.clear();
        self.count = 0;
    }
}

impl Default for PointStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get() {
        let mut pts = PointStorage::new();
        let h0 = pts.add(Vec3::new(1.0, 2.0, 3.0));
        let h1 = pts.add(Vec3::new(4.0, 5.0, 6.0));

        assert_eq!(pts.len(), 2);
        assert_eq!(h0.index(), 0);
        assert_eq!(h1.index(), 1);

        let p0 = pts.position(h0);
        assert_eq!(p0, Vec3::new(1.0, 2.0, 3.0));

        let p1 = pts.position(h1);
        assert_eq!(p1, Vec3::new(4.0, 5.0, 6.0));
    }

    #[test]
    fn test_set_position() {
        let mut pts = PointStorage::new();
        let h = pts.add(Vec3::ZERO);
        pts.set_position(h, Vec3::new(7.0, 8.0, 9.0));
        assert_eq!(pts.position(h), Vec3::new(7.0, 8.0, 9.0));
    }

    #[test]
    fn test_soa_layout() {
        let mut pts = PointStorage::new();
        pts.add(Vec3::new(1.0, 4.0, 7.0));
        pts.add(Vec3::new(2.0, 5.0, 8.0));
        pts.add(Vec3::new(3.0, 6.0, 9.0));

        // SoA: x values are contiguous
        assert_eq!(pts.x_slice(), &[1.0, 2.0, 3.0]);
        assert_eq!(pts.y_slice(), &[4.0, 5.0, 6.0]);
        assert_eq!(pts.z_slice(), &[7.0, 8.0, 9.0]);
    }

    #[test]
    fn test_iter() {
        let mut pts = PointStorage::new();
        pts.add(Vec3::ZERO);
        pts.add(Vec3::ONE);
        pts.add(Vec3::X);

        let handles: Vec<_> = pts.iter().collect();
        assert_eq!(handles.len(), 3);
        assert_eq!(handles[0].index(), 0);
        assert_eq!(handles[1].index(), 1);
        assert_eq!(handles[2].index(), 2);
    }

    #[test]
    fn test_with_capacity() {
        let pts = PointStorage::with_capacity(100);
        assert_eq!(pts.len(), 0);
        assert!(pts.is_empty());
    }
}
```

- [ ] **Step 2: Export from lib.rs**

Update `crates/procgeo-core/src/lib.rs`:
```rust
pub mod error;
pub mod handle;
pub mod point;

pub use handle::{PointHandle, PrimHandle, VertexHandle};
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p procgeo-core`
Expected: All point tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/procgeo-core/src/point.rs crates/procgeo-core/src/lib.rs
git commit -m "feat(core): add SoA point storage with contiguous x/y/z layout"
```

---

### Task 4: Vertex and Primitive Storage

**Files:**
- Create: `crates/procgeo-core/src/vertex.rs`
- Create: `crates/procgeo-core/src/primitive.rs`
- Modify: `crates/procgeo-core/src/lib.rs`

- [ ] **Step 1: Implement VertexStorage**

```rust
// crates/procgeo-core/src/vertex.rs

use crate::handle::{PointHandle, PrimHandle, VertexHandle};

/// Storage for vertices. Each vertex references a point and belongs to a primitive.
pub struct VertexStorage {
    /// Which point each vertex references.
    pub(crate) point_refs: Vec<PointHandle>,
    /// Which primitive each vertex belongs to.
    pub(crate) prim_refs: Vec<PrimHandle>,
    count: u32,
}

impl VertexStorage {
    pub fn new() -> Self {
        Self {
            point_refs: Vec::new(),
            prim_refs: Vec::new(),
            count: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            point_refs: Vec::with_capacity(capacity),
            prim_refs: Vec::with_capacity(capacity),
            count: 0,
        }
    }

    /// Add a vertex referencing a point, belonging to a primitive.
    pub fn add(&mut self, point: PointHandle, prim: PrimHandle) -> VertexHandle {
        let handle = VertexHandle(self.count);
        self.point_refs.push(point);
        self.prim_refs.push(prim);
        self.count += 1;
        handle
    }

    /// Get the point this vertex references.
    #[inline]
    pub fn point(&self, handle: VertexHandle) -> PointHandle {
        self.point_refs[handle.index()]
    }

    /// Get the primitive this vertex belongs to.
    #[inline]
    pub fn prim(&self, handle: VertexHandle) -> PrimHandle {
        self.prim_refs[handle.index()]
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.count as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = VertexHandle> {
        (0..self.count).map(VertexHandle)
    }

    pub fn reserve(&mut self, additional: usize) {
        self.point_refs.reserve(additional);
        self.prim_refs.reserve(additional);
    }

    pub fn clear(&mut self) {
        self.point_refs.clear();
        self.prim_refs.clear();
        self.count = 0;
    }
}

impl Default for VertexStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_add_and_query() {
        let mut verts = VertexStorage::new();
        let pt = PointHandle::from_index(5);
        let pr = PrimHandle::from_index(2);
        let vh = verts.add(pt, pr);

        assert_eq!(verts.len(), 1);
        assert_eq!(verts.point(vh), pt);
        assert_eq!(verts.prim(vh), pr);
    }

    #[test]
    fn test_multiple_vertices_same_point() {
        let mut verts = VertexStorage::new();
        let pt = PointHandle::from_index(0);
        let pr0 = PrimHandle::from_index(0);
        let pr1 = PrimHandle::from_index(1);

        let v0 = verts.add(pt, pr0);
        let v1 = verts.add(pt, pr1);

        // Both vertices reference the same point
        assert_eq!(verts.point(v0), pt);
        assert_eq!(verts.point(v1), pt);
        // But belong to different prims
        assert_ne!(verts.prim(v0), verts.prim(v1));
    }
}
```

- [ ] **Step 2: Implement Primitive types and PrimStorage**

```rust
// crates/procgeo-core/src/primitive.rs

use crate::handle::{PrimHandle, VertexHandle};
use smallvec::SmallVec;

/// Whether a polygon is open (polyline) or closed (face).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PolyType {
    Open,
    Closed,
}

/// A polygon primitive: a sequence of vertices forming an open or closed polygon.
#[derive(Clone, Debug)]
pub struct PolygonPrim {
    pub vertices: SmallVec<[VertexHandle; 4]>,
    pub poly_type: PolyType,
}

/// Primitive enum — each variant holds its specific data.
/// Start with Polygon only; other types added as SOPs need them.
#[derive(Clone, Debug)]
pub enum Primitive {
    Polygon(PolygonPrim),
}

impl Primitive {
    /// Get the vertices of this primitive.
    pub fn vertices(&self) -> &[VertexHandle] {
        match self {
            Primitive::Polygon(p) => &p.vertices,
        }
    }

    /// Number of vertices in this primitive.
    pub fn vertex_count(&self) -> usize {
        match self {
            Primitive::Polygon(p) => p.vertices.len(),
        }
    }
}

/// Storage for all primitives.
pub struct PrimStorage {
    pub(crate) prims: Vec<Primitive>,
}

impl PrimStorage {
    pub fn new() -> Self {
        Self { prims: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            prims: Vec::with_capacity(capacity),
        }
    }

    /// Add a primitive, returns its handle.
    pub fn add(&mut self, prim: Primitive) -> PrimHandle {
        let handle = PrimHandle::from_index(self.prims.len());
        self.prims.push(prim);
        handle
    }

    /// Get a primitive by handle.
    #[inline]
    pub fn get(&self, handle: PrimHandle) -> &Primitive {
        &self.prims[handle.index()]
    }

    /// Get a mutable reference to a primitive.
    #[inline]
    pub fn get_mut(&mut self, handle: PrimHandle) -> &mut Primitive {
        &mut self.prims[handle.index()]
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.prims.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.prims.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = PrimHandle> {
        (0..self.prims.len()).map(|i| PrimHandle::from_index(i))
    }

    pub fn clear(&mut self) {
        self.prims.clear();
    }
}

impl Default for PrimStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_polygon() {
        let mut storage = PrimStorage::new();
        let verts: SmallVec<[VertexHandle; 4]> = smallvec::smallvec![
            VertexHandle::from_index(0),
            VertexHandle::from_index(1),
            VertexHandle::from_index(2),
        ];
        let prim = Primitive::Polygon(PolygonPrim {
            vertices: verts,
            poly_type: PolyType::Closed,
        });
        let handle = storage.add(prim);

        assert_eq!(storage.len(), 1);
        assert_eq!(storage.get(handle).vertex_count(), 3);
    }

    #[test]
    fn test_polygon_vertices() {
        let v0 = VertexHandle::from_index(10);
        let v1 = VertexHandle::from_index(11);
        let v2 = VertexHandle::from_index(12);
        let v3 = VertexHandle::from_index(13);
        let prim = Primitive::Polygon(PolygonPrim {
            vertices: smallvec::smallvec![v0, v1, v2, v3],
            poly_type: PolyType::Closed,
        });

        assert_eq!(prim.vertices(), &[v0, v1, v2, v3]);
        assert_eq!(prim.vertex_count(), 4);
    }

    #[test]
    fn test_smallvec_inline_for_quad() {
        // SmallVec<[VertexHandle; 4]> should store up to 4 verts inline
        let verts: SmallVec<[VertexHandle; 4]> = smallvec::smallvec![
            VertexHandle::from_index(0),
            VertexHandle::from_index(1),
            VertexHandle::from_index(2),
            VertexHandle::from_index(3),
        ];
        // 4 verts fit inline — no heap allocation
        assert!(!verts.spilled());
    }
}
```

- [ ] **Step 3: Export from lib.rs**

Update `crates/procgeo-core/src/lib.rs`:
```rust
pub mod error;
pub mod handle;
pub mod point;
pub mod primitive;
pub mod vertex;

pub use handle::{PointHandle, PrimHandle, VertexHandle};
pub use primitive::{PolyType, PolygonPrim, Primitive, PrimStorage};
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p procgeo-core`
Expected: All tests pass (handle + point + vertex + primitive).

- [ ] **Step 5: Commit**

```bash
git add crates/procgeo-core/src/vertex.rs crates/procgeo-core/src/primitive.rs crates/procgeo-core/src/lib.rs
git commit -m "feat(core): add vertex storage and polygon primitive types"
```

---

### Task 5: Attribute System

**Files:**
- Create: `crates/procgeo-core/src/attribute.rs`
- Modify: `crates/procgeo-core/src/lib.rs`

- [ ] **Step 1: Implement attribute types, storage, and the attribute map**

This is the largest single module. Key components:

1. `AttributeType` — enum of supported types (Int, Float, Vector3, etc.)
2. `AttributeClass` — Point, Vertex, Primitive, Detail
3. `TypeQualifier` — none, point, vector, normal, color, quaternion, matrix
4. `AttributeStorage` — enum wrapping typed `Vec<T>` for each type
5. `Attribute` — name + type + qualifier + default + storage
6. `AttributeMap` — HashMap<(AttributeClass, String), Attribute> managing all attributes
7. `AttribHandle<T>` — typed handle for compile-time safe attribute access

```rust
// crates/procgeo-core/src/attribute.rs

use std::collections::HashMap;
use std::marker::PhantomData;
use crate::error::CoreError;

/// Attribute class matching Houdini's four levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AttribClass {
    Point,
    Vertex,
    Primitive,
    Detail,
}

/// Type of data stored in an attribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AttribType {
    Int,
    Int64,
    Float,
    Float64,
    Vector2,
    Vector3,
    Vector4,
    Matrix3,
    Matrix4,
    String,
}

/// How transforms should affect this attribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum TypeQualifier {
    #[default]
    None,
    Point,
    Vector,
    Normal,
    Color,
    Quaternion,
    Matrix,
}

/// Typed storage for attribute data. Each variant is a contiguous Vec.
#[derive(Clone, Debug)]
pub enum AttribStorage {
    Int(Vec<i32>),
    Int64(Vec<i64>),
    Float(Vec<f32>),
    Float64(Vec<f64>),
    Vector2(Vec<[f32; 2]>),
    Vector3(Vec<[f32; 3]>),
    Vector4(Vec<[f32; 4]>),
    Matrix3(Vec<[f32; 9]>),
    Matrix4(Vec<[f32; 16]>),
    String(Vec<std::string::String>),
}

impl AttribStorage {
    /// Number of elements stored.
    pub fn len(&self) -> usize {
        match self {
            Self::Int(v) => v.len(),
            Self::Int64(v) => v.len(),
            Self::Float(v) => v.len(),
            Self::Float64(v) => v.len(),
            Self::Vector2(v) => v.len(),
            Self::Vector3(v) => v.len(),
            Self::Vector4(v) => v.len(),
            Self::Matrix3(v) => v.len(),
            Self::Matrix4(v) => v.len(),
            Self::String(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the type of this storage.
    pub fn attrib_type(&self) -> AttribType {
        match self {
            Self::Int(_) => AttribType::Int,
            Self::Int64(_) => AttribType::Int64,
            Self::Float(_) => AttribType::Float,
            Self::Float64(_) => AttribType::Float64,
            Self::Vector2(_) => AttribType::Vector2,
            Self::Vector3(_) => AttribType::Vector3,
            Self::Vector4(_) => AttribType::Vector4,
            Self::Matrix3(_) => AttribType::Matrix3,
            Self::Matrix4(_) => AttribType::Matrix4,
            Self::String(_) => AttribType::String,
        }
    }

    /// Resize storage to `new_len`, filling new elements with default.
    pub fn resize_with_default(&mut self, new_len: usize, default: &AttribDefault) {
        match (self, default) {
            (Self::Int(v), AttribDefault::Int(d)) => v.resize(new_len, *d),
            (Self::Int64(v), AttribDefault::Int64(d)) => v.resize(new_len, *d),
            (Self::Float(v), AttribDefault::Float(d)) => v.resize(new_len, *d),
            (Self::Float64(v), AttribDefault::Float64(d)) => v.resize(new_len, *d),
            (Self::Vector2(v), AttribDefault::Vector2(d)) => v.resize(new_len, *d),
            (Self::Vector3(v), AttribDefault::Vector3(d)) => v.resize(new_len, *d),
            (Self::Vector4(v), AttribDefault::Vector4(d)) => v.resize(new_len, *d),
            (Self::Matrix3(v), AttribDefault::Matrix3(d)) => v.resize(new_len, *d),
            (Self::Matrix4(v), AttribDefault::Matrix4(d)) => v.resize(new_len, *d),
            (Self::String(v), AttribDefault::String(d)) => v.resize(new_len, d.clone()),
            _ => panic!("storage/default type mismatch"),
        }
    }
}

/// Default value for an attribute, used when new elements are created.
#[derive(Clone, Debug)]
pub enum AttribDefault {
    Int(i32),
    Int64(i64),
    Float(f32),
    Float64(f64),
    Vector2([f32; 2]),
    Vector3([f32; 3]),
    Vector4([f32; 4]),
    Matrix3([f32; 9]),
    Matrix4([f32; 16]),
    String(std::string::String),
}

impl AttribDefault {
    pub fn attrib_type(&self) -> AttribType {
        match self {
            Self::Int(_) => AttribType::Int,
            Self::Int64(_) => AttribType::Int64,
            Self::Float(_) => AttribType::Float,
            Self::Float64(_) => AttribType::Float64,
            Self::Vector2(_) => AttribType::Vector2,
            Self::Vector3(_) => AttribType::Vector3,
            Self::Vector4(_) => AttribType::Vector4,
            Self::Matrix3(_) => AttribType::Matrix3,
            Self::Matrix4(_) => AttribType::Matrix4,
            Self::String(_) => AttribType::String,
        }
    }

    /// Create empty storage matching this default's type.
    pub fn empty_storage(&self) -> AttribStorage {
        match self {
            Self::Int(_) => AttribStorage::Int(Vec::new()),
            Self::Int64(_) => AttribStorage::Int64(Vec::new()),
            Self::Float(_) => AttribStorage::Float(Vec::new()),
            Self::Float64(_) => AttribStorage::Float64(Vec::new()),
            Self::Vector2(_) => AttribStorage::Vector2(Vec::new()),
            Self::Vector3(_) => AttribStorage::Vector3(Vec::new()),
            Self::Vector4(_) => AttribStorage::Vector4(Vec::new()),
            Self::Matrix3(_) => AttribStorage::Matrix3(Vec::new()),
            Self::Matrix4(_) => AttribStorage::Matrix4(Vec::new()),
            Self::String(_) => AttribStorage::String(Vec::new()),
        }
    }
}

/// A single attribute: name + type + storage.
#[derive(Clone, Debug)]
pub struct Attribute {
    pub name: std::string::String,
    pub class: AttribClass,
    pub qualifier: TypeQualifier,
    pub default: AttribDefault,
    pub storage: AttribStorage,
}

/// Typed handle for compile-time safe attribute access.
/// The type parameter `T` is the element type (e.g., `[f32; 3]` for Vector3).
pub struct AttribHandle<T> {
    pub(crate) class: AttribClass,
    pub(crate) name: std::string::String,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T> Clone for AttribHandle<T> {
    fn clone(&self) -> Self {
        Self {
            class: self.class,
            name: self.name.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<T> std::fmt::Debug for AttribHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AttribHandle<{}>({:?}, {:?})", std::any::type_name::<T>(), self.class, self.name)
    }
}

/// Trait for types that can be stored in attributes.
pub trait AttribValue: Clone + 'static {
    fn attrib_type() -> AttribType;
    fn default_value() -> AttribDefault;
    fn get_from_storage(storage: &AttribStorage, index: usize) -> Option<&Self>;
    fn get_from_storage_mut(storage: &mut AttribStorage, index: usize) -> Option<&mut Self>;
    fn set_in_storage(storage: &mut AttribStorage, index: usize, value: Self);
    fn get_slice(storage: &AttribStorage) -> Option<&[Self]>;
    fn get_slice_mut(storage: &mut AttribStorage) -> Option<&mut [Self]>;
}

macro_rules! impl_attrib_value {
    ($rust_type:ty, $variant:ident, $default:expr) => {
        impl AttribValue for $rust_type {
            fn attrib_type() -> AttribType { AttribType::$variant }
            fn default_value() -> AttribDefault { AttribDefault::$variant($default) }

            fn get_from_storage(storage: &AttribStorage, index: usize) -> Option<&Self> {
                match storage {
                    AttribStorage::$variant(v) => v.get(index),
                    _ => None,
                }
            }

            fn get_from_storage_mut(storage: &mut AttribStorage, index: usize) -> Option<&mut Self> {
                match storage {
                    AttribStorage::$variant(v) => v.get_mut(index),
                    _ => None,
                }
            }

            fn set_in_storage(storage: &mut AttribStorage, index: usize, value: Self) {
                match storage {
                    AttribStorage::$variant(v) => v[index] = value,
                    _ => panic!("type mismatch in set_in_storage"),
                }
            }

            fn get_slice(storage: &AttribStorage) -> Option<&[Self]> {
                match storage {
                    AttribStorage::$variant(v) => Some(v.as_slice()),
                    _ => None,
                }
            }

            fn get_slice_mut(storage: &mut AttribStorage) -> Option<&mut [Self]> {
                match storage {
                    AttribStorage::$variant(v) => Some(v.as_mut_slice()),
                    _ => None,
                }
            }
        }
    };
}

impl_attrib_value!(i32, Int, 0);
impl_attrib_value!(i64, Int64, 0);
impl_attrib_value!(f32, Float, 0.0);
impl_attrib_value!(f64, Float64, 0.0);
impl_attrib_value!([f32; 2], Vector2, [0.0; 2]);
impl_attrib_value!([f32; 3], Vector3, [0.0; 3]);
impl_attrib_value!([f32; 4], Vector4, [0.0; 4]);
impl_attrib_value!([f32; 9], Matrix3, [0.0; 9]);
impl_attrib_value!([f32; 16], Matrix4, [0.0; 16]);

// String needs special handling
impl AttribValue for std::string::String {
    fn attrib_type() -> AttribType { AttribType::String }
    fn default_value() -> AttribDefault { AttribDefault::String(std::string::String::new()) }

    fn get_from_storage(storage: &AttribStorage, index: usize) -> Option<&Self> {
        match storage { AttribStorage::String(v) => v.get(index), _ => None }
    }

    fn get_from_storage_mut(storage: &mut AttribStorage, index: usize) -> Option<&mut Self> {
        match storage { AttribStorage::String(v) => v.get_mut(index), _ => None }
    }

    fn set_in_storage(storage: &mut AttribStorage, index: usize, value: Self) {
        match storage { AttribStorage::String(v) => v[index] = value, _ => panic!("type mismatch") }
    }

    fn get_slice(storage: &AttribStorage) -> Option<&[Self]> {
        match storage { AttribStorage::String(v) => Some(v.as_slice()), _ => None }
    }

    fn get_slice_mut(storage: &mut AttribStorage) -> Option<&mut [Self]> {
        match storage { AttribStorage::String(v) => Some(v.as_mut_slice()), _ => None }
    }
}

/// Map of all attributes, keyed by (class, name).
#[derive(Clone, Debug, Default)]
pub struct AttributeMap {
    attrs: HashMap<(AttribClass, std::string::String), Attribute>,
}

impl AttributeMap {
    pub fn new() -> Self {
        Self { attrs: HashMap::new() }
    }

    /// Create a new attribute. Returns error if it already exists with a different type.
    pub fn create(
        &mut self,
        class: AttribClass,
        name: &str,
        default: AttribDefault,
        qualifier: TypeQualifier,
        initial_size: usize,
    ) -> Result<(), CoreError> {
        let key = (class, name.to_string());
        if let Some(existing) = self.attrs.get(&key) {
            if existing.storage.attrib_type() != default.attrib_type() {
                return Err(CoreError::AttributeTypeMismatch {
                    name: name.to_string(),
                    expected: format!("{:?}", existing.storage.attrib_type()),
                    got: format!("{:?}", default.attrib_type()),
                });
            }
            return Ok(()); // already exists with same type
        }
        let mut storage = default.empty_storage();
        storage.resize_with_default(initial_size, &default);
        self.attrs.insert(key, Attribute {
            name: name.to_string(),
            class,
            qualifier,
            default,
            storage,
        });
        Ok(())
    }

    /// Get a typed handle for an attribute.
    pub fn find<T: AttribValue>(&self, class: AttribClass, name: &str) -> Result<AttribHandle<T>, CoreError> {
        let key = (class, name.to_string());
        match self.attrs.get(&key) {
            Some(attr) => {
                if attr.storage.attrib_type() != T::attrib_type() {
                    return Err(CoreError::AttributeTypeMismatch {
                        name: name.to_string(),
                        expected: format!("{:?}", T::attrib_type()),
                        got: format!("{:?}", attr.storage.attrib_type()),
                    });
                }
                Ok(AttribHandle {
                    class,
                    name: name.to_string(),
                    _phantom: PhantomData,
                })
            }
            None => Err(CoreError::AttributeNotFound { name: name.to_string() }),
        }
    }

    /// Get a value using a typed handle.
    pub fn get<T: AttribValue>(&self, handle: &AttribHandle<T>, index: usize) -> Option<&T> {
        let key = (handle.class, handle.name.clone());
        self.attrs.get(&key).and_then(|attr| T::get_from_storage(&attr.storage, index))
    }

    /// Set a value using a typed handle.
    pub fn set<T: AttribValue>(&mut self, handle: &AttribHandle<T>, index: usize, value: T) {
        let key = (handle.class, handle.name.clone());
        if let Some(attr) = self.attrs.get_mut(&key) {
            T::set_in_storage(&mut attr.storage, index, value);
        }
    }

    /// Get the raw attribute data.
    pub fn get_raw(&self, class: AttribClass, name: &str) -> Option<&Attribute> {
        self.attrs.get(&(class, name.to_string()))
    }

    /// Get mutable raw attribute data.
    pub fn get_raw_mut(&mut self, class: AttribClass, name: &str) -> Option<&mut Attribute> {
        self.attrs.get_mut(&(class, name.to_string()))
    }

    /// Delete an attribute.
    pub fn delete(&mut self, class: AttribClass, name: &str) -> bool {
        self.attrs.remove(&(class, name.to_string())).is_some()
    }

    /// Resize all attributes of a given class to new_size.
    pub fn resize_class(&mut self, class: AttribClass, new_size: usize) {
        for attr in self.attrs.values_mut() {
            if attr.class == class {
                let default = attr.default.clone();
                attr.storage.resize_with_default(new_size, &default);
            }
        }
    }

    /// List all attribute names for a class.
    pub fn names(&self, class: AttribClass) -> Vec<&str> {
        self.attrs.iter()
            .filter(|((c, _), _)| *c == class)
            .map(|((_, name), _)| name.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_find_attribute() {
        let mut map = AttributeMap::new();
        map.create(
            AttribClass::Point,
            "Cd",
            AttribDefault::Vector3([1.0, 1.0, 1.0]),
            TypeQualifier::Color,
            4,
        ).unwrap();

        let handle = map.find::<[f32; 3]>(AttribClass::Point, "Cd").unwrap();
        let val = map.get(&handle, 0).unwrap();
        assert_eq!(*val, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_set_attribute_value() {
        let mut map = AttributeMap::new();
        map.create(
            AttribClass::Point,
            "pscale",
            AttribDefault::Float(1.0),
            TypeQualifier::None,
            3,
        ).unwrap();

        let handle = map.find::<f32>(AttribClass::Point, "pscale").unwrap();
        map.set(&handle, 1, 2.5);
        assert_eq!(*map.get(&handle, 1).unwrap(), 2.5);
        assert_eq!(*map.get(&handle, 0).unwrap(), 1.0); // default
    }

    #[test]
    fn test_type_mismatch_error() {
        let mut map = AttributeMap::new();
        map.create(
            AttribClass::Point,
            "id",
            AttribDefault::Int(0),
            TypeQualifier::None,
            1,
        ).unwrap();

        // Try to find as float — should fail
        let result = map.find::<f32>(AttribClass::Point, "id");
        assert!(result.is_err());
    }

    #[test]
    fn test_attribute_not_found() {
        let map = AttributeMap::new();
        let result = map.find::<f32>(AttribClass::Point, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_attribute() {
        let mut map = AttributeMap::new();
        map.create(AttribClass::Point, "test", AttribDefault::Int(0), TypeQualifier::None, 1).unwrap();
        assert!(map.delete(AttribClass::Point, "test"));
        assert!(!map.delete(AttribClass::Point, "test")); // already deleted
    }

    #[test]
    fn test_resize_class() {
        let mut map = AttributeMap::new();
        map.create(AttribClass::Point, "Cd", AttribDefault::Vector3([0.0; 3]), TypeQualifier::Color, 2).unwrap();
        map.create(AttribClass::Point, "pscale", AttribDefault::Float(1.0), TypeQualifier::None, 2).unwrap();

        map.resize_class(AttribClass::Point, 5);

        let h_cd = map.find::<[f32; 3]>(AttribClass::Point, "Cd").unwrap();
        let h_ps = map.find::<f32>(AttribClass::Point, "pscale").unwrap();

        // New elements filled with defaults
        assert_eq!(*map.get(&h_cd, 4).unwrap(), [0.0; 3]);
        assert_eq!(*map.get(&h_ps, 4).unwrap(), 1.0);
    }

    #[test]
    fn test_names() {
        let mut map = AttributeMap::new();
        map.create(AttribClass::Point, "Cd", AttribDefault::Vector3([0.0; 3]), TypeQualifier::Color, 1).unwrap();
        map.create(AttribClass::Point, "N", AttribDefault::Vector3([0.0; 3]), TypeQualifier::Normal, 1).unwrap();
        map.create(AttribClass::Primitive, "shop_materialpath", AttribDefault::String(String::new()), TypeQualifier::None, 1).unwrap();

        let mut pt_names = map.names(AttribClass::Point);
        pt_names.sort();
        assert_eq!(pt_names, vec!["Cd", "N"]);

        let prim_names = map.names(AttribClass::Primitive);
        assert_eq!(prim_names, vec!["shop_materialpath"]);
    }
}
```

- [ ] **Step 2: Export from lib.rs**

Update `crates/procgeo-core/src/lib.rs`:
```rust
pub mod attribute;
pub mod error;
pub mod handle;
pub mod point;
pub mod primitive;
pub mod vertex;

pub use attribute::{AttribClass, AttribDefault, AttribHandle, AttribStorage, AttribType, AttribValue, Attribute, AttributeMap, TypeQualifier};
pub use handle::{PointHandle, PrimHandle, VertexHandle};
pub use primitive::{PolyType, PolygonPrim, Primitive, PrimStorage};
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p procgeo-core`
Expected: All attribute tests pass alongside existing tests.

- [ ] **Step 4: Commit**

```bash
git add crates/procgeo-core/src/attribute.rs crates/procgeo-core/src/lib.rs
git commit -m "feat(core): add typed attribute system with storage, handles, and class support"
```

---

### Task 6: Groups

**Files:**
- Create: `crates/procgeo-core/src/group.rs`
- Modify: `crates/procgeo-core/src/lib.rs`

- [ ] **Step 1: Implement group types and GroupMap**

```rust
// crates/procgeo-core/src/group.rs

use std::collections::{HashMap, HashSet};
use bitvec::prelude::*;
use crate::handle::{PointHandle, PrimHandle, VertexHandle};

/// A bitset-based element group.
#[derive(Clone, Debug)]
pub struct ElementGroup {
    bits: BitVec,
}

impl ElementGroup {
    pub fn new(size: usize) -> Self {
        Self { bits: bitvec![0; size] }
    }

    #[inline]
    pub fn contains(&self, index: usize) -> bool {
        self.bits.get(index).map(|b| *b).unwrap_or(false)
    }

    #[inline]
    pub fn set(&mut self, index: usize, value: bool) {
        if index < self.bits.len() {
            self.bits.set(index, value);
        }
    }

    #[inline]
    pub fn add(&mut self, index: usize) {
        self.set(index, true);
    }

    #[inline]
    pub fn remove(&mut self, index: usize) {
        self.set(index, false);
    }

    /// Number of elements in the group.
    pub fn count(&self) -> usize {
        self.bits.count_ones()
    }

    /// Total capacity (number of elements in the geometry).
    pub fn size(&self) -> usize {
        self.bits.len()
    }

    /// Resize the group to match element count.
    pub fn resize(&mut self, new_size: usize) {
        self.bits.resize(new_size, false);
    }

    /// Iterate over indices that are in the group.
    pub fn iter_set(&self) -> impl Iterator<Item = usize> + '_ {
        self.bits.iter_ones()
    }

    /// Union with another group (OR).
    pub fn union(&mut self, other: &ElementGroup) {
        self.bits |= &other.bits;
    }

    /// Intersect with another group (AND).
    pub fn intersect(&mut self, other: &ElementGroup) {
        self.bits &= &other.bits;
    }

    /// Subtract another group (AND NOT).
    pub fn subtract(&mut self, other: &ElementGroup) {
        let negated = !other.bits.clone();
        self.bits &= &negated;
    }

    /// Complement (NOT).
    pub fn complement(&mut self) {
        self.bits = !self.bits.clone();
    }

    /// Clear all membership.
    pub fn clear(&mut self) {
        self.bits.fill(false);
    }
}

/// An edge group stores (prim_handle, local_edge_index) pairs.
#[derive(Clone, Debug, Default)]
pub struct EdgeGroup {
    edges: HashSet<(PrimHandle, u8)>,
}

impl EdgeGroup {
    pub fn new() -> Self {
        Self { edges: HashSet::new() }
    }

    pub fn add(&mut self, prim: PrimHandle, edge_index: u8) {
        self.edges.insert((prim, edge_index));
    }

    pub fn remove(&mut self, prim: PrimHandle, edge_index: u8) {
        self.edges.remove(&(prim, edge_index));
    }

    pub fn contains(&self, prim: PrimHandle, edge_index: u8) -> bool {
        self.edges.contains(&(prim, edge_index))
    }

    pub fn count(&self) -> usize {
        self.edges.len()
    }

    pub fn clear(&mut self) {
        self.edges.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &(PrimHandle, u8)> {
        self.edges.iter()
    }
}

/// Manages all named groups for a Geometry.
#[derive(Clone, Debug, Default)]
pub struct GroupMap {
    point_groups: HashMap<String, ElementGroup>,
    prim_groups: HashMap<String, ElementGroup>,
    vertex_groups: HashMap<String, ElementGroup>,
    edge_groups: HashMap<String, EdgeGroup>,
}

impl GroupMap {
    pub fn new() -> Self {
        Self::default()
    }

    // -- Point groups --

    pub fn create_point_group(&mut self, name: &str, size: usize) {
        self.point_groups.entry(name.to_string()).or_insert_with(|| ElementGroup::new(size));
    }

    pub fn point_group(&self, name: &str) -> Option<&ElementGroup> {
        self.point_groups.get(name)
    }

    pub fn point_group_mut(&mut self, name: &str) -> Option<&mut ElementGroup> {
        self.point_groups.get_mut(name)
    }

    pub fn delete_point_group(&mut self, name: &str) -> bool {
        self.point_groups.remove(name).is_some()
    }

    pub fn point_group_names(&self) -> impl Iterator<Item = &str> {
        self.point_groups.keys().map(|s| s.as_str())
    }

    /// Resize all point groups to match new point count.
    pub fn resize_point_groups(&mut self, new_size: usize) {
        for group in self.point_groups.values_mut() {
            group.resize(new_size);
        }
    }

    // -- Primitive groups --

    pub fn create_prim_group(&mut self, name: &str, size: usize) {
        self.prim_groups.entry(name.to_string()).or_insert_with(|| ElementGroup::new(size));
    }

    pub fn prim_group(&self, name: &str) -> Option<&ElementGroup> {
        self.prim_groups.get(name)
    }

    pub fn prim_group_mut(&mut self, name: &str) -> Option<&mut ElementGroup> {
        self.prim_groups.get_mut(name)
    }

    pub fn delete_prim_group(&mut self, name: &str) -> bool {
        self.prim_groups.remove(name).is_some()
    }

    pub fn prim_group_names(&self) -> impl Iterator<Item = &str> {
        self.prim_groups.keys().map(|s| s.as_str())
    }

    pub fn resize_prim_groups(&mut self, new_size: usize) {
        for group in self.prim_groups.values_mut() {
            group.resize(new_size);
        }
    }

    // -- Vertex groups --

    pub fn create_vertex_group(&mut self, name: &str, size: usize) {
        self.vertex_groups.entry(name.to_string()).or_insert_with(|| ElementGroup::new(size));
    }

    pub fn vertex_group(&self, name: &str) -> Option<&ElementGroup> {
        self.vertex_groups.get(name)
    }

    pub fn vertex_group_mut(&mut self, name: &str) -> Option<&mut ElementGroup> {
        self.vertex_groups.get_mut(name)
    }

    pub fn resize_vertex_groups(&mut self, new_size: usize) {
        for group in self.vertex_groups.values_mut() {
            group.resize(new_size);
        }
    }

    // -- Edge groups --

    pub fn create_edge_group(&mut self, name: &str) {
        self.edge_groups.entry(name.to_string()).or_insert_with(EdgeGroup::new);
    }

    pub fn edge_group(&self, name: &str) -> Option<&EdgeGroup> {
        self.edge_groups.get(name)
    }

    pub fn edge_group_mut(&mut self, name: &str) -> Option<&mut EdgeGroup> {
        self.edge_groups.get_mut(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_group_basic() {
        let mut g = ElementGroup::new(10);
        assert_eq!(g.count(), 0);

        g.add(3);
        g.add(7);
        assert!(g.contains(3));
        assert!(g.contains(7));
        assert!(!g.contains(5));
        assert_eq!(g.count(), 2);

        g.remove(3);
        assert!(!g.contains(3));
        assert_eq!(g.count(), 1);
    }

    #[test]
    fn test_group_boolean_ops() {
        let mut a = ElementGroup::new(8);
        a.add(0); a.add(1); a.add(2); a.add(3);

        let mut b = ElementGroup::new(8);
        b.add(2); b.add(3); b.add(4); b.add(5);

        // Union
        let mut u = a.clone();
        u.union(&b);
        assert_eq!(u.count(), 6); // 0,1,2,3,4,5

        // Intersect
        let mut i = a.clone();
        i.intersect(&b);
        assert_eq!(i.count(), 2); // 2,3

        // Subtract
        let mut s = a.clone();
        s.subtract(&b);
        assert_eq!(s.count(), 2); // 0,1
        assert!(s.contains(0));
        assert!(s.contains(1));
        assert!(!s.contains(2));
    }

    #[test]
    fn test_group_complement() {
        let mut g = ElementGroup::new(4);
        g.add(1);
        g.complement();
        assert!(g.contains(0));
        assert!(!g.contains(1));
        assert!(g.contains(2));
        assert!(g.contains(3));
    }

    #[test]
    fn test_group_iter_set() {
        let mut g = ElementGroup::new(10);
        g.add(2); g.add(5); g.add(8);
        let indices: Vec<_> = g.iter_set().collect();
        assert_eq!(indices, vec![2, 5, 8]);
    }

    #[test]
    fn test_edge_group() {
        let mut eg = EdgeGroup::new();
        let p0 = PrimHandle::from_index(0);
        eg.add(p0, 2);
        assert!(eg.contains(p0, 2));
        assert!(!eg.contains(p0, 1));
        assert_eq!(eg.count(), 1);
    }

    #[test]
    fn test_group_map() {
        let mut gm = GroupMap::new();
        gm.create_point_group("selected", 10);
        gm.point_group_mut("selected").unwrap().add(3);
        assert!(gm.point_group("selected").unwrap().contains(3));

        gm.create_prim_group("walls", 5);
        gm.prim_group_mut("walls").unwrap().add(0);
        assert!(gm.prim_group("walls").unwrap().contains(0));
    }

    #[test]
    fn test_group_resize() {
        let mut gm = GroupMap::new();
        gm.create_point_group("test", 4);
        gm.point_group_mut("test").unwrap().add(2);

        gm.resize_point_groups(8);
        assert!(gm.point_group("test").unwrap().contains(2));
        assert!(!gm.point_group("test").unwrap().contains(6));
        assert_eq!(gm.point_group("test").unwrap().size(), 8);
    }
}
```

- [ ] **Step 2: Export from lib.rs**

Add `pub mod group;` and re-exports to `crates/procgeo-core/src/lib.rs`:
```rust
pub mod attribute;
pub mod error;
pub mod group;
pub mod handle;
pub mod point;
pub mod primitive;
pub mod vertex;

pub use attribute::{AttribClass, AttribDefault, AttribHandle, AttribStorage, AttribType, AttribValue, Attribute, AttributeMap, TypeQualifier};
pub use group::{EdgeGroup, ElementGroup, GroupMap};
pub use handle::{PointHandle, PrimHandle, VertexHandle};
pub use primitive::{PolyType, PolygonPrim, Primitive, PrimStorage};
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p procgeo-core`
Expected: All group tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/procgeo-core/src/group.rs crates/procgeo-core/src/lib.rs
git commit -m "feat(core): add bitset-based groups with boolean operations"
```

---

### Task 7: BBox and Math Utilities

**Files:**
- Create: `crates/procgeo-core/src/math/mod.rs`
- Create: `crates/procgeo-core/src/math/bbox.rs`
- Modify: `crates/procgeo-core/src/lib.rs`

- [ ] **Step 1: Implement BBox**

```rust
// crates/procgeo-core/src/math/bbox.rs

use glam::Vec3;

/// Axis-aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BBox {
    /// Create a new empty (inverted) bounding box.
    pub fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }

    /// Create a bounding box from min and max corners.
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Expand the bounding box to include a point.
    pub fn expand_point(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    /// Expand to include another bounding box.
    pub fn expand_bbox(&mut self, other: &BBox) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    /// Center of the bounding box.
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Size (dimensions) of the bounding box.
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    /// Check if a point is inside the bounding box.
    pub fn contains(&self, point: Vec3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x
            && point.y >= self.min.y && point.y <= self.max.y
            && point.z >= self.min.z && point.z <= self.max.z
    }

    /// Check if two bounding boxes overlap.
    pub fn intersects(&self, other: &BBox) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x
            && self.min.y <= other.max.y && self.max.y >= other.min.y
            && self.min.z <= other.max.z && self.max.z >= other.min.z
    }

    /// Whether this is a valid (non-empty) bounding box.
    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z
    }

    /// Compute bounding box from SoA point arrays.
    pub fn from_soa(x: &[f32], y: &[f32], z: &[f32]) -> Self {
        let mut bbox = Self::empty();
        for i in 0..x.len() {
            bbox.expand_point(Vec3::new(x[i], y[i], z[i]));
        }
        bbox
    }
}

impl Default for BBox {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_bbox() {
        let bbox = BBox::empty();
        assert!(!bbox.is_valid());
    }

    #[test]
    fn test_expand_point() {
        let mut bbox = BBox::empty();
        bbox.expand_point(Vec3::new(1.0, 2.0, 3.0));
        bbox.expand_point(Vec3::new(-1.0, -2.0, -3.0));
        assert_eq!(bbox.min, Vec3::new(-1.0, -2.0, -3.0));
        assert_eq!(bbox.max, Vec3::new(1.0, 2.0, 3.0));
        assert!(bbox.is_valid());
    }

    #[test]
    fn test_center_and_size() {
        let bbox = BBox::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(bbox.center(), Vec3::ZERO);
        assert_eq!(bbox.size(), Vec3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn test_contains() {
        let bbox = BBox::new(Vec3::ZERO, Vec3::ONE);
        assert!(bbox.contains(Vec3::new(0.5, 0.5, 0.5)));
        assert!(!bbox.contains(Vec3::new(1.5, 0.5, 0.5)));
    }

    #[test]
    fn test_intersects() {
        let a = BBox::new(Vec3::ZERO, Vec3::ONE);
        let b = BBox::new(Vec3::splat(0.5), Vec3::splat(1.5));
        let c = BBox::new(Vec3::splat(2.0), Vec3::splat(3.0));
        assert!(a.intersects(&b));
        assert!(!a.intersects(&c));
    }

    #[test]
    fn test_from_soa() {
        let x = [1.0, -1.0, 0.5];
        let y = [2.0, -2.0, 0.0];
        let z = [3.0, -3.0, 1.0];
        let bbox = BBox::from_soa(&x, &y, &z);
        assert_eq!(bbox.min, Vec3::new(-1.0, -2.0, -3.0));
        assert_eq!(bbox.max, Vec3::new(1.0, 2.0, 3.0));
    }
}
```

```rust
// crates/procgeo-core/src/math/mod.rs

pub mod bbox;

pub use bbox::BBox;

/// Remap a value from one range to another (Houdini's fit()).
#[inline]
pub fn fit(value: f32, old_min: f32, old_max: f32, new_min: f32, new_max: f32) -> f32 {
    let t = (value - old_min) / (old_max - old_min);
    new_min + t * (new_max - new_min)
}

/// Clamped fit — clamps value to [old_min, old_max] before remapping.
#[inline]
pub fn efit(value: f32, old_min: f32, old_max: f32, new_min: f32, new_max: f32) -> f32 {
    let clamped = value.clamp(old_min, old_max);
    fit(clamped, old_min, old_max, new_min, new_max)
}

/// Smooth hermite interpolation (Houdini's smooth()).
#[inline]
pub fn smooth(value: f32, min: f32, max: f32) -> f32 {
    let t = ((value - min) / (max - min)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fit() {
        assert!((fit(0.5, 0.0, 1.0, 0.0, 10.0) - 5.0).abs() < 1e-6);
        assert!((fit(0.0, 0.0, 1.0, 10.0, 20.0) - 10.0).abs() < 1e-6);
        assert!((fit(1.0, 0.0, 1.0, 10.0, 20.0) - 20.0).abs() < 1e-6);
    }

    #[test]
    fn test_efit_clamps() {
        assert!((efit(2.0, 0.0, 1.0, 0.0, 10.0) - 10.0).abs() < 1e-6);
        assert!((efit(-1.0, 0.0, 1.0, 0.0, 10.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_smooth() {
        assert!((smooth(0.0, 0.0, 1.0) - 0.0).abs() < 1e-6);
        assert!((smooth(1.0, 0.0, 1.0) - 1.0).abs() < 1e-6);
        assert!((smooth(0.5, 0.0, 1.0) - 0.5).abs() < 1e-6);
    }
}
```

- [ ] **Step 2: Export from lib.rs**

Add `pub mod math;` to `crates/procgeo-core/src/lib.rs`.

- [ ] **Step 3: Run tests**

Run: `cargo test -p procgeo-core`
Expected: All math tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/procgeo-core/src/math/ crates/procgeo-core/src/lib.rs
git commit -m "feat(core): add BBox and Houdini-style math helpers (fit, smooth)"
```

---

### Task 8: Geometry Struct

The central `Geometry` struct that ties together points, vertices, primitives, attributes, and groups. This is the API surface all SOPs interact with.

**Files:**
- Create: `crates/procgeo-core/src/geometry.rs`
- Modify: `crates/procgeo-core/src/lib.rs`

- [ ] **Step 1: Implement the Geometry struct**

```rust
// crates/procgeo-core/src/geometry.rs

use glam::Vec3;
use smallvec::SmallVec;

use crate::attribute::{AttribClass, AttribDefault, AttribHandle, AttribValue, AttributeMap, TypeQualifier};
use crate::error::CoreError;
use crate::group::GroupMap;
use crate::handle::{PointHandle, PrimHandle, VertexHandle};
use crate::math::BBox;
use crate::point::PointStorage;
use crate::primitive::{PolyType, PolygonPrim, PrimStorage, Primitive};
use crate::vertex::VertexStorage;

/// The central geometry container. Owns all points, vertices, primitives,
/// attributes, and groups.
pub struct Geometry {
    pub(crate) points: PointStorage,
    pub(crate) vertices: VertexStorage,
    pub(crate) primitives: PrimStorage,
    pub(crate) attributes: AttributeMap,
    pub(crate) groups: GroupMap,
}

impl Geometry {
    /// Create an empty geometry.
    pub fn new() -> Self {
        Self {
            points: PointStorage::new(),
            vertices: VertexStorage::new(),
            primitives: PrimStorage::new(),
            attributes: AttributeMap::new(),
            groups: GroupMap::new(),
        }
    }

    /// Create a geometry with pre-allocated capacity.
    pub fn with_capacity(points: usize, prims: usize) -> Self {
        Self {
            points: PointStorage::with_capacity(points),
            vertices: VertexStorage::with_capacity(prims * 4), // estimate 4 verts per prim
            primitives: PrimStorage::with_capacity(prims),
            attributes: AttributeMap::new(),
            groups: GroupMap::new(),
        }
    }

    // -- Points --

    /// Add a point at the given position.
    pub fn add_point(&mut self, pos: Vec3) -> PointHandle {
        let handle = self.points.add(pos);
        self.attributes.resize_class(AttribClass::Point, self.points.len());
        self.groups.resize_point_groups(self.points.len());
        handle
    }

    /// Get a point's position.
    #[inline]
    pub fn point_pos(&self, handle: PointHandle) -> Vec3 {
        self.points.position(handle)
    }

    /// Set a point's position.
    #[inline]
    pub fn set_point_pos(&mut self, handle: PointHandle, pos: Vec3) {
        self.points.set_position(handle, pos);
    }

    /// Number of points.
    #[inline]
    pub fn num_points(&self) -> usize {
        self.points.len()
    }

    /// Iterate over all point handles.
    pub fn points(&self) -> impl Iterator<Item = PointHandle> {
        self.points.iter()
    }

    // -- Vertices --

    /// Number of vertices.
    #[inline]
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Get the point a vertex references.
    #[inline]
    pub fn vertex_point(&self, handle: VertexHandle) -> PointHandle {
        self.vertices.point(handle)
    }

    /// Get the primitive a vertex belongs to.
    #[inline]
    pub fn vertex_prim(&self, handle: VertexHandle) -> PrimHandle {
        self.vertices.prim(handle)
    }

    // -- Primitives --

    /// Add a closed polygon from a slice of point handles.
    pub fn add_polygon(&mut self, points: &[PointHandle], poly_type: PolyType) -> PrimHandle {
        let prim_handle = PrimHandle::from_index(self.primitives.len());
        let mut vert_handles = SmallVec::with_capacity(points.len());
        for &pt in points {
            let vh = self.vertices.add(pt, prim_handle);
            vert_handles.push(vh);
        }
        self.attributes.resize_class(AttribClass::Vertex, self.vertices.len());
        self.groups.resize_vertex_groups(self.vertices.len());

        let prim = Primitive::Polygon(PolygonPrim {
            vertices: vert_handles,
            poly_type,
        });
        let handle = self.primitives.add(prim);
        self.attributes.resize_class(AttribClass::Primitive, self.primitives.len());
        self.groups.resize_prim_groups(self.primitives.len());
        handle
    }

    /// Add a face (closed polygon).
    pub fn add_face(&mut self, points: &[PointHandle]) -> PrimHandle {
        self.add_polygon(points, PolyType::Closed)
    }

    /// Add an open polyline.
    pub fn add_polyline(&mut self, points: &[PointHandle]) -> PrimHandle {
        self.add_polygon(points, PolyType::Open)
    }

    /// Number of primitives.
    #[inline]
    pub fn num_prims(&self) -> usize {
        self.primitives.len()
    }

    /// Iterate over all primitive handles.
    pub fn prims(&self) -> impl Iterator<Item = PrimHandle> {
        self.primitives.iter()
    }

    /// Get a primitive by handle.
    pub fn prim(&self, handle: PrimHandle) -> &Primitive {
        self.primitives.get(handle)
    }

    /// Get vertices of a primitive.
    pub fn prim_vertices(&self, handle: PrimHandle) -> &[VertexHandle] {
        self.primitives.get(handle).vertices()
    }

    /// Get point handles for a primitive (resolving through vertices).
    pub fn prim_points(&self, handle: PrimHandle) -> Vec<PointHandle> {
        self.prim_vertices(handle)
            .iter()
            .map(|vh| self.vertices.point(*vh))
            .collect()
    }

    // -- Attributes --

    /// Create an attribute.
    pub fn add_attrib(
        &mut self,
        class: AttribClass,
        name: &str,
        default: AttribDefault,
        qualifier: TypeQualifier,
    ) -> Result<(), CoreError> {
        let size = match class {
            AttribClass::Point => self.num_points(),
            AttribClass::Vertex => self.num_vertices(),
            AttribClass::Primitive => self.num_prims(),
            AttribClass::Detail => 1,
        };
        self.attributes.create(class, name, default, qualifier, size)
    }

    /// Find a typed attribute handle.
    pub fn find_attrib<T: AttribValue>(&self, class: AttribClass, name: &str) -> Result<AttribHandle<T>, CoreError> {
        self.attributes.find::<T>(class, name)
    }

    /// Get an attribute value.
    pub fn get_attrib<T: AttribValue>(&self, handle: &AttribHandle<T>, index: usize) -> Option<&T> {
        self.attributes.get(handle, index)
    }

    /// Set an attribute value.
    pub fn set_attrib<T: AttribValue>(&mut self, handle: &AttribHandle<T>, index: usize, value: T) {
        self.attributes.set(handle, index, value);
    }

    /// Get a reference to the attribute map.
    pub fn attributes(&self) -> &AttributeMap {
        &self.attributes
    }

    /// Get a mutable reference to the attribute map.
    pub fn attributes_mut(&mut self) -> &mut AttributeMap {
        &mut self.attributes
    }

    // -- Groups --

    /// Get a reference to the group map.
    pub fn groups(&self) -> &GroupMap {
        &self.groups
    }

    /// Get a mutable reference to the group map.
    pub fn groups_mut(&mut self) -> &mut GroupMap {
        &mut self.groups
    }

    /// Create a point group.
    pub fn create_point_group(&mut self, name: &str) {
        self.groups.create_point_group(name, self.num_points());
    }

    /// Create a primitive group.
    pub fn create_prim_group(&mut self, name: &str) {
        self.groups.create_prim_group(name, self.num_prims());
    }

    // -- Spatial --

    /// Compute the bounding box of all points.
    pub fn bounding_box(&self) -> BBox {
        BBox::from_soa(self.points.x_slice(), self.points.y_slice(), self.points.z_slice())
    }

    // -- Raw access for SOPs --

    /// Raw point storage access.
    pub fn point_storage(&self) -> &PointStorage {
        &self.points
    }

    pub fn point_storage_mut(&mut self) -> &mut PointStorage {
        &mut self.points
    }
}

impl Default for Geometry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Geometry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Geometry(points: {}, prims: {}, vertices: {})",
            self.num_points(),
            self.num_prims(),
            self.num_vertices()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_geometry() {
        let geo = Geometry::new();
        assert_eq!(geo.num_points(), 0);
        assert_eq!(geo.num_prims(), 0);
        assert_eq!(geo.num_vertices(), 0);
    }

    #[test]
    fn test_add_points() {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::ZERO);
        let p1 = geo.add_point(Vec3::ONE);

        assert_eq!(geo.num_points(), 2);
        assert_eq!(geo.point_pos(p0), Vec3::ZERO);
        assert_eq!(geo.point_pos(p1), Vec3::ONE);
    }

    #[test]
    fn test_add_triangle() {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        let p1 = geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        let p2 = geo.add_point(Vec3::new(0.0, 1.0, 0.0));

        let face = geo.add_face(&[p0, p1, p2]);
        assert_eq!(geo.num_prims(), 1);
        assert_eq!(geo.num_vertices(), 3);
        assert_eq!(geo.prim_points(face), vec![p0, p1, p2]);
    }

    #[test]
    fn test_shared_points() {
        // Two triangles sharing an edge (2 points)
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        let p1 = geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        let p2 = geo.add_point(Vec3::new(0.0, 1.0, 0.0));
        let p3 = geo.add_point(Vec3::new(1.0, 1.0, 0.0));

        geo.add_face(&[p0, p1, p2]);
        geo.add_face(&[p1, p3, p2]);

        assert_eq!(geo.num_points(), 4);
        assert_eq!(geo.num_prims(), 2);
        assert_eq!(geo.num_vertices(), 6); // 3 + 3, even though points shared
    }

    #[test]
    fn test_bounding_box() {
        let mut geo = Geometry::new();
        geo.add_point(Vec3::new(-1.0, -2.0, -3.0));
        geo.add_point(Vec3::new(1.0, 2.0, 3.0));
        geo.add_point(Vec3::ZERO);

        let bbox = geo.bounding_box();
        assert_eq!(bbox.min, Vec3::new(-1.0, -2.0, -3.0));
        assert_eq!(bbox.max, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_attributes_on_geometry() {
        let mut geo = Geometry::new();
        geo.add_point(Vec3::ZERO);
        geo.add_point(Vec3::ONE);

        geo.add_attrib(
            AttribClass::Point,
            "Cd",
            AttribDefault::Vector3([1.0, 1.0, 1.0]),
            TypeQualifier::Color,
        ).unwrap();

        let h = geo.find_attrib::<[f32; 3]>(AttribClass::Point, "Cd").unwrap();
        assert_eq!(*geo.get_attrib(&h, 0).unwrap(), [1.0, 1.0, 1.0]);

        geo.set_attrib(&h, 0, [1.0, 0.0, 0.0]);
        assert_eq!(*geo.get_attrib(&h, 0).unwrap(), [1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_attributes_auto_resize_on_add_point() {
        let mut geo = Geometry::new();
        geo.add_attrib(
            AttribClass::Point,
            "id",
            AttribDefault::Int(0),
            TypeQualifier::None,
        ).unwrap();

        geo.add_point(Vec3::ZERO);
        geo.add_point(Vec3::ONE);
        geo.add_point(Vec3::X);

        // Attribute should have been resized to 3
        let h = geo.find_attrib::<i32>(AttribClass::Point, "id").unwrap();
        assert_eq!(*geo.get_attrib(&h, 2).unwrap(), 0); // default value
    }

    #[test]
    fn test_groups_on_geometry() {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::ZERO);
        let p1 = geo.add_point(Vec3::ONE);
        let _p2 = geo.add_point(Vec3::X);

        geo.create_point_group("selected");
        geo.groups_mut().point_group_mut("selected").unwrap().add(p0.index());
        geo.groups_mut().point_group_mut("selected").unwrap().add(p1.index());

        let group = geo.groups().point_group("selected").unwrap();
        assert!(group.contains(p0.index()));
        assert!(group.contains(p1.index()));
        assert_eq!(group.count(), 2);
    }

    #[test]
    fn test_polyline() {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::ZERO);
        let p1 = geo.add_point(Vec3::X);
        let p2 = geo.add_point(Vec3::new(2.0, 0.0, 0.0));

        let line = geo.add_polyline(&[p0, p1, p2]);
        match geo.prim(line) {
            Primitive::Polygon(p) => assert_eq!(p.poly_type, PolyType::Open),
            _ => panic!("expected polygon"),
        }
    }
}
```

- [ ] **Step 2: Export from lib.rs**

Update `crates/procgeo-core/src/lib.rs` to add `pub mod geometry;` and re-export `Geometry`:
```rust
pub mod attribute;
pub mod error;
pub mod geometry;
pub mod group;
pub mod handle;
pub mod math;
pub mod point;
pub mod primitive;
pub mod vertex;

pub use attribute::{AttribClass, AttribDefault, AttribHandle, AttribStorage, AttribType, AttribValue, Attribute, AttributeMap, TypeQualifier};
pub use geometry::Geometry;
pub use group::{EdgeGroup, ElementGroup, GroupMap};
pub use handle::{PointHandle, PrimHandle, VertexHandle};
pub use primitive::{PolyType, PolygonPrim, Primitive, PrimStorage};
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p procgeo-core`
Expected: All geometry tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/procgeo-core/src/geometry.rs crates/procgeo-core/src/lib.rs
git commit -m "feat(core): add Geometry struct tying together points, prims, attributes, groups"
```

---

### Task 9: SOP Trait and Error Types

**Files:**
- Modify: `crates/procgeo-sops/src/lib.rs`

- [ ] **Step 1: Implement the Sop trait, SopError, and apply chain**

```rust
// crates/procgeo-sops/src/lib.rs

use procgeo_core::Geometry;
use thiserror::Error;

#[cfg(feature = "creation")]
pub mod creation;
#[cfg(feature = "transform")]
pub mod transform;
#[cfg(feature = "normals")]
pub mod normals;
#[cfg(feature = "merge")]
pub mod merge;

#[derive(Debug, Error)]
pub enum SopError {
    #[error("wrong number of inputs: expected {expected_min}-{expected_max}, got {got}")]
    WrongInputCount {
        expected_min: usize,
        expected_max: usize,
        got: usize,
    },
    #[error("invalid parameter: {0}")]
    InvalidParam(String),
    #[error("core error: {0}")]
    Core(#[from] procgeo_core::error::CoreError),
    #[error("{0}")]
    Other(String),
}

/// The SOP trait — each SOP is a stateless processor that takes geometry and
/// params, producing new geometry.
pub trait Sop {
    type Params: Default;

    /// Execute the SOP on the given inputs with the given parameters.
    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError>;

    /// Minimum and maximum number of inputs this SOP accepts.
    fn input_count(&self) -> (usize, usize);

    /// The name of this SOP (matches Houdini node name).
    fn name(&self) -> &'static str;

    /// Validate input count.
    fn validate_inputs(&self, inputs: &[&Geometry]) -> Result<(), SopError> {
        let (min, max) = self.input_count();
        let got = inputs.len();
        if got < min || got > max {
            return Err(SopError::WrongInputCount {
                expected_min: min,
                expected_max: max,
                got,
            });
        }
        Ok(())
    }
}

/// Extension trait on Geometry for SOP chaining.
impl Geometry {
    /// Apply a SOP to this geometry as the first input.
    pub fn apply<S: Sop>(self, sop: &S, params: &S::Params) -> Result<Geometry, SopError> {
        sop.execute(&[&self], params)
    }
}

/// Convenience function to run a zero-input SOP (generators like Box, Sphere).
pub fn generate<S: Sop>(sop: &S, params: &S::Params) -> Result<Geometry, SopError> {
    sop.execute(&[], params)
}
```

- [ ] **Step 2: Create stub modules for categories**

Create empty module files so the feature-gated imports don't fail:

`crates/procgeo-sops/src/creation/mod.rs`:
```rust
// Creation SOPs: Box, Grid, Line, Circle, Sphere, Tube, Torus
```

`crates/procgeo-sops/src/transform/mod.rs`:
```rust
// Transform SOPs
```

`crates/procgeo-sops/src/normals/mod.rs`:
```rust
// Normal SOPs
```

`crates/procgeo-sops/src/merge/mod.rs`:
```rust
// Merge SOPs
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p procgeo-sops`
Expected: Compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add crates/procgeo-sops/
git commit -m "feat(sops): add Sop trait, SopError, and apply chain"
```

---

### Task 10: Box SOP

The first creation SOP. Generates a unit box (8 points, 6 quad faces) matching Houdini's Box SOP defaults.

**Files:**
- Create: `crates/procgeo-sops/src/creation/box_sop.rs`
- Modify: `crates/procgeo-sops/src/creation/mod.rs`

- [ ] **Step 1: Implement Box SOP with tests**

```rust
// crates/procgeo-sops/src/creation/box_sop.rs

use glam::Vec3;
use procgeo_core::{Geometry, PolyType};
use crate::{Sop, SopError};

/// Parameters for the Box SOP, matching Houdini's parameter interface.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BoxParams {
    /// Size of the box along each axis.
    pub size: Vec3,
    /// Center position of the box.
    pub center: Vec3,
}

impl Default for BoxParams {
    fn default() -> Self {
        Self {
            size: Vec3::ONE,
            center: Vec3::ZERO,
        }
    }
}

/// Box SOP — generates an axis-aligned box.
pub struct BoxSop;

impl Sop for BoxSop {
    type Params = BoxParams;

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        let half = params.size * 0.5;
        let c = params.center;
        let mut geo = Geometry::with_capacity(8, 6);

        // 8 corner points of the box
        // Houdini box vertex ordering (matching Houdini's convention):
        // Bottom face (y = -half.y): 0-3, Top face (y = +half.y): 4-7
        let p0 = geo.add_point(c + Vec3::new(-half.x, -half.y, -half.z));
        let p1 = geo.add_point(c + Vec3::new(half.x, -half.y, -half.z));
        let p2 = geo.add_point(c + Vec3::new(half.x, -half.y, half.z));
        let p3 = geo.add_point(c + Vec3::new(-half.x, -half.y, half.z));
        let p4 = geo.add_point(c + Vec3::new(-half.x, half.y, -half.z));
        let p5 = geo.add_point(c + Vec3::new(half.x, half.y, -half.z));
        let p6 = geo.add_point(c + Vec3::new(half.x, half.y, half.z));
        let p7 = geo.add_point(c + Vec3::new(-half.x, half.y, half.z));

        // 6 quad faces (outward-facing winding)
        geo.add_face(&[p0, p3, p2, p1]); // bottom (-Y)
        geo.add_face(&[p4, p5, p6, p7]); // top (+Y)
        geo.add_face(&[p0, p1, p5, p4]); // front (-Z)
        geo.add_face(&[p2, p3, p7, p6]); // back (+Z)
        geo.add_face(&[p0, p4, p7, p3]); // left (-X)
        geo.add_face(&[p1, p2, p6, p5]); // right (+X)

        Ok(geo)
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 0) // generator — no inputs
    }

    fn name(&self) -> &'static str {
        "box"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use procgeo_core::math::BBox;

    #[test]
    fn test_box_default() {
        let geo = BoxSop.execute(&[], &BoxParams::default()).unwrap();
        assert_eq!(geo.num_points(), 8);
        assert_eq!(geo.num_prims(), 6);
        assert_eq!(geo.num_vertices(), 24); // 6 quads × 4 verts

        let bbox = geo.bounding_box();
        assert!((bbox.min - Vec3::splat(-0.5)).length() < 1e-6);
        assert!((bbox.max - Vec3::splat(0.5)).length() < 1e-6);
    }

    #[test]
    fn test_box_custom_size() {
        let params = BoxParams {
            size: Vec3::new(2.0, 4.0, 6.0),
            center: Vec3::ZERO,
        };
        let geo = BoxSop.execute(&[], &params).unwrap();
        let bbox = geo.bounding_box();
        assert!((bbox.min - Vec3::new(-1.0, -2.0, -3.0)).length() < 1e-6);
        assert!((bbox.max - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-6);
    }

    #[test]
    fn test_box_with_center() {
        let params = BoxParams {
            size: Vec3::ONE,
            center: Vec3::new(10.0, 20.0, 30.0),
        };
        let geo = BoxSop.execute(&[], &params).unwrap();
        let bbox = geo.bounding_box();
        assert!((bbox.center() - Vec3::new(10.0, 20.0, 30.0)).length() < 1e-6);
    }

    #[test]
    fn test_box_all_faces_are_quads() {
        let geo = BoxSop.execute(&[], &BoxParams::default()).unwrap();
        for ph in geo.prims() {
            assert_eq!(geo.prim(ph).vertex_count(), 4, "all box faces should be quads");
        }
    }

    #[test]
    fn test_box_rejects_inputs() {
        let input = Geometry::new();
        let result = BoxSop.execute(&[&input], &BoxParams::default());
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Export from creation/mod.rs**

```rust
// crates/procgeo-sops/src/creation/mod.rs

mod box_sop;

pub use box_sop::{BoxParams, BoxSop};
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p procgeo-sops`
Expected: All box tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/procgeo-sops/src/creation/
git commit -m "feat(sops): add Box SOP with size and center params"
```

---

### Task 11: Grid SOP

Generates a planar grid of quads. Houdini default: 10×10 rows/cols, size 10×10, on XZ plane.

**Files:**
- Create: `crates/procgeo-sops/src/creation/grid.rs`
- Modify: `crates/procgeo-sops/src/creation/mod.rs`

- [ ] **Step 1: Implement Grid SOP with tests**

```rust
// crates/procgeo-sops/src/creation/grid.rs

use glam::Vec3;
use procgeo_core::Geometry;
use crate::{Sop, SopError};

/// Orientation plane for the grid.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum GridOrientation {
    #[default]
    XZ,
    XY,
    YZ,
}

/// Parameters for the Grid SOP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GridParams {
    /// Size of the grid.
    pub size: [f32; 2],
    /// Number of rows (subdivisions along second axis).
    pub rows: u32,
    /// Number of columns (subdivisions along first axis).
    pub cols: u32,
    /// Center position.
    pub center: Vec3,
    /// Orientation plane.
    pub orientation: GridOrientation,
}

impl Default for GridParams {
    fn default() -> Self {
        Self {
            size: [10.0, 10.0],
            rows: 10,
            cols: 10,
            center: Vec3::ZERO,
            orientation: GridOrientation::XZ,
        }
    }
}

pub struct GridSop;

impl Sop for GridSop {
    type Params = GridParams;

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        if params.rows < 2 || params.cols < 2 {
            return Err(SopError::InvalidParam(
                "rows and cols must be >= 2".to_string(),
            ));
        }

        let rows = params.rows as usize;
        let cols = params.cols as usize;
        let num_points = rows * cols;
        let num_prims = (rows - 1) * (cols - 1);

        let mut geo = Geometry::with_capacity(num_points, num_prims);

        let half_w = params.size[0] * 0.5;
        let half_h = params.size[1] * 0.5;

        // Generate points
        for r in 0..rows {
            let v = r as f32 / (rows - 1) as f32; // 0..1
            for c in 0..cols {
                let u = c as f32 / (cols - 1) as f32; // 0..1
                let a = -half_w + u * params.size[0];
                let b = -half_h + v * params.size[1];

                let pos = match params.orientation {
                    GridOrientation::XZ => params.center + Vec3::new(a, 0.0, b),
                    GridOrientation::XY => params.center + Vec3::new(a, b, 0.0),
                    GridOrientation::YZ => params.center + Vec3::new(0.0, a, b),
                };
                geo.add_point(pos);
            }
        }

        // Generate quad faces
        for r in 0..(rows - 1) {
            for c in 0..(cols - 1) {
                let i = r * cols + c;
                let pts = [
                    procgeo_core::PointHandle::from_index(i),
                    procgeo_core::PointHandle::from_index(i + 1),
                    procgeo_core::PointHandle::from_index(i + cols + 1),
                    procgeo_core::PointHandle::from_index(i + cols),
                ];
                geo.add_face(&pts);
            }
        }

        Ok(geo)
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 0)
    }

    fn name(&self) -> &'static str {
        "grid"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_default() {
        let geo = GridSop.execute(&[], &GridParams::default()).unwrap();
        // 10×10 grid = 100 points, 9×9 = 81 quads, 81×4 = 324 verts
        assert_eq!(geo.num_points(), 100);
        assert_eq!(geo.num_prims(), 81);
        assert_eq!(geo.num_vertices(), 324);
    }

    #[test]
    fn test_grid_2x2() {
        let params = GridParams {
            rows: 2,
            cols: 2,
            size: [1.0, 1.0],
            ..Default::default()
        };
        let geo = GridSop.execute(&[], &params).unwrap();
        assert_eq!(geo.num_points(), 4);
        assert_eq!(geo.num_prims(), 1);
        assert_eq!(geo.num_vertices(), 4);
    }

    #[test]
    fn test_grid_3x3() {
        let params = GridParams {
            rows: 3,
            cols: 3,
            size: [2.0, 2.0],
            ..Default::default()
        };
        let geo = GridSop.execute(&[], &params).unwrap();
        assert_eq!(geo.num_points(), 9);
        assert_eq!(geo.num_prims(), 4);
    }

    #[test]
    fn test_grid_bounding_box() {
        let params = GridParams {
            size: [4.0, 6.0],
            rows: 5,
            cols: 5,
            ..Default::default()
        };
        let geo = GridSop.execute(&[], &params).unwrap();
        let bbox = geo.bounding_box();
        assert!((bbox.min.x - (-2.0)).abs() < 1e-6);
        assert!((bbox.max.x - 2.0).abs() < 1e-6);
        assert!((bbox.min.z - (-3.0)).abs() < 1e-6);
        assert!((bbox.max.z - 3.0).abs() < 1e-6);
        assert!((bbox.min.y).abs() < 1e-6); // flat on XZ
        assert!((bbox.max.y).abs() < 1e-6);
    }

    #[test]
    fn test_grid_xy_orientation() {
        let params = GridParams {
            orientation: GridOrientation::XY,
            rows: 3,
            cols: 3,
            size: [2.0, 2.0],
            ..Default::default()
        };
        let geo = GridSop.execute(&[], &params).unwrap();
        let bbox = geo.bounding_box();
        // Should be flat on Z
        assert!((bbox.min.z).abs() < 1e-6);
        assert!((bbox.max.z).abs() < 1e-6);
    }

    #[test]
    fn test_grid_rejects_small() {
        let params = GridParams {
            rows: 1,
            cols: 3,
            ..Default::default()
        };
        assert!(GridSop.execute(&[], &params).is_err());
    }
}
```

- [ ] **Step 2: Export from creation/mod.rs**

Add to `crates/procgeo-sops/src/creation/mod.rs`:
```rust
mod box_sop;
mod grid;

pub use box_sop::{BoxParams, BoxSop};
pub use grid::{GridOrientation, GridParams, GridSop};
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p procgeo-sops`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/procgeo-sops/src/creation/
git commit -m "feat(sops): add Grid SOP with rows/cols and orientation"
```

---

### Task 12: Line, Circle, Sphere, Tube, Torus SOPs

Implement the remaining core creation SOPs. Each follows the same pattern as Box/Grid.

**Files:**
- Create: `crates/procgeo-sops/src/creation/line.rs`
- Create: `crates/procgeo-sops/src/creation/circle.rs`
- Create: `crates/procgeo-sops/src/creation/sphere.rs`
- Create: `crates/procgeo-sops/src/creation/tube.rs`
- Create: `crates/procgeo-sops/src/creation/torus.rs`
- Modify: `crates/procgeo-sops/src/creation/mod.rs`

- [ ] **Step 1: Implement Line SOP**

```rust
// crates/procgeo-sops/src/creation/line.rs

use glam::Vec3;
use procgeo_core::Geometry;
use crate::{Sop, SopError};

/// Parameters for the Line SOP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LineParams {
    /// Start point of the line.
    pub origin: Vec3,
    /// Direction of the line.
    pub direction: Vec3,
    /// Length of the line.
    pub length: f32,
    /// Number of points along the line.
    pub points: u32,
}

impl Default for LineParams {
    fn default() -> Self {
        Self {
            origin: Vec3::ZERO,
            direction: Vec3::Y,
            length: 1.0,
            points: 2,
        }
    }
}

pub struct LineSop;

impl Sop for LineSop {
    type Params = LineParams;

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        if params.points < 2 {
            return Err(SopError::InvalidParam("points must be >= 2".to_string()));
        }

        let n = params.points as usize;
        let mut geo = Geometry::with_capacity(n, 1);
        let dir = params.direction.normalize_or_zero() * params.length;

        let mut pt_handles = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / (n - 1) as f32;
            let pos = params.origin + dir * t;
            pt_handles.push(geo.add_point(pos));
        }

        // Create an open polyline
        geo.add_polyline(&pt_handles);

        Ok(geo)
    }

    fn input_count(&self) -> (usize, usize) { (0, 0) }
    fn name(&self) -> &'static str { "line" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_default() {
        let geo = LineSop.execute(&[], &LineParams::default()).unwrap();
        assert_eq!(geo.num_points(), 2);
        assert_eq!(geo.num_prims(), 1);

        let p0 = geo.point_pos(procgeo_core::PointHandle::from_index(0));
        let p1 = geo.point_pos(procgeo_core::PointHandle::from_index(1));
        assert!((p0 - Vec3::ZERO).length() < 1e-6);
        assert!((p1 - Vec3::Y).length() < 1e-6);
    }

    #[test]
    fn test_line_multiple_points() {
        let params = LineParams {
            points: 5,
            length: 4.0,
            direction: Vec3::X,
            ..Default::default()
        };
        let geo = LineSop.execute(&[], &params).unwrap();
        assert_eq!(geo.num_points(), 5);

        // Points should be evenly spaced along X
        let p2 = geo.point_pos(procgeo_core::PointHandle::from_index(2));
        assert!((p2.x - 2.0).abs() < 1e-6); // midpoint
    }
}
```

- [ ] **Step 2: Implement Circle SOP**

```rust
// crates/procgeo-sops/src/creation/circle.rs

use glam::Vec3;
use procgeo_core::Geometry;
use crate::{Sop, SopError};

/// Parameters for the Circle SOP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CircleParams {
    /// Radius of the circle.
    pub radius: f32,
    /// Center position.
    pub center: Vec3,
    /// Number of divisions (points around the circle).
    pub divisions: u32,
    /// Orientation plane.
    pub orientation: super::grid::GridOrientation,
}

impl Default for CircleParams {
    fn default() -> Self {
        Self {
            radius: 1.0,
            center: Vec3::ZERO,
            divisions: 40,
            orientation: super::grid::GridOrientation::XZ,
        }
    }
}

pub struct CircleSop;

impl Sop for CircleSop {
    type Params = CircleParams;

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        if params.divisions < 3 {
            return Err(SopError::InvalidParam("divisions must be >= 3".to_string()));
        }

        let n = params.divisions as usize;
        let mut geo = Geometry::with_capacity(n, 1);

        let mut pt_handles = Vec::with_capacity(n);
        for i in 0..n {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / n as f32;
            let (sin_a, cos_a) = angle.sin_cos();
            let a = cos_a * params.radius;
            let b = sin_a * params.radius;

            let pos = match params.orientation {
                super::grid::GridOrientation::XZ => params.center + Vec3::new(a, 0.0, b),
                super::grid::GridOrientation::XY => params.center + Vec3::new(a, b, 0.0),
                super::grid::GridOrientation::YZ => params.center + Vec3::new(0.0, a, b),
            };
            pt_handles.push(geo.add_point(pos));
        }

        // Closed polygon (single face)
        geo.add_face(&pt_handles);

        Ok(geo)
    }

    fn input_count(&self) -> (usize, usize) { (0, 0) }
    fn name(&self) -> &'static str { "circle" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circle_default() {
        let geo = CircleSop.execute(&[], &CircleParams::default()).unwrap();
        assert_eq!(geo.num_points(), 40);
        assert_eq!(geo.num_prims(), 1);
        assert_eq!(geo.num_vertices(), 40);
    }

    #[test]
    fn test_circle_radius() {
        let params = CircleParams {
            radius: 5.0,
            divisions: 8,
            ..Default::default()
        };
        let geo = CircleSop.execute(&[], &params).unwrap();

        // All points should be at distance ~5.0 from center
        for ph in geo.points() {
            let pos = geo.point_pos(ph);
            let dist = Vec3::new(pos.x, 0.0, pos.z).length();
            assert!((dist - 5.0).abs() < 1e-5);
        }
    }
}
```

- [ ] **Step 3: Implement Sphere SOP**

```rust
// crates/procgeo-sops/src/creation/sphere.rs

use glam::Vec3;
use procgeo_core::{Geometry, PointHandle};
use crate::{Sop, SopError};

/// Parameters for the Sphere SOP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SphereParams {
    /// Radius of the sphere.
    pub radius: Vec3,
    /// Center position.
    pub center: Vec3,
    /// Number of rows (latitude divisions).
    pub rows: u32,
    /// Number of columns (longitude divisions).
    pub cols: u32,
}

impl Default for SphereParams {
    fn default() -> Self {
        Self {
            radius: Vec3::splat(0.5),
            center: Vec3::ZERO,
            rows: 12,
            cols: 24,
        }
    }
}

pub struct SphereSop;

impl Sop for SphereSop {
    type Params = SphereParams;

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        if params.rows < 2 || params.cols < 3 {
            return Err(SopError::InvalidParam(
                "rows must be >= 2 and cols must be >= 3".to_string(),
            ));
        }

        let rows = params.rows as usize;
        let cols = params.cols as usize;

        // UV sphere: poles + (rows-1) rings of cols points each
        let num_points = 2 + (rows - 1) * cols;
        let num_prims = cols + (rows - 2) * cols + cols; // top tris + middle quads + bottom tris
        let mut geo = Geometry::with_capacity(num_points, num_prims);

        // Top pole
        let top = geo.add_point(params.center + Vec3::new(0.0, params.radius.y, 0.0));

        // Middle rings
        let mut ring_pts: Vec<Vec<PointHandle>> = Vec::with_capacity(rows - 1);
        for r in 1..rows {
            let phi = std::f32::consts::PI * r as f32 / rows as f32;
            let (sin_phi, cos_phi) = phi.sin_cos();
            let mut ring = Vec::with_capacity(cols);
            for c in 0..cols {
                let theta = 2.0 * std::f32::consts::PI * c as f32 / cols as f32;
                let (sin_theta, cos_theta) = theta.sin_cos();
                let pos = params.center + Vec3::new(
                    params.radius.x * sin_phi * cos_theta,
                    params.radius.y * cos_phi,
                    params.radius.z * sin_phi * sin_theta,
                );
                ring.push(geo.add_point(pos));
            }
            ring_pts.push(ring);
        }

        // Bottom pole
        let bottom = geo.add_point(params.center + Vec3::new(0.0, -params.radius.y, 0.0));

        // Top cap triangles
        for c in 0..cols {
            let next = (c + 1) % cols;
            geo.add_face(&[top, ring_pts[0][c], ring_pts[0][next]]);
        }

        // Middle quad strips
        for r in 0..(ring_pts.len() - 1) {
            for c in 0..cols {
                let next = (c + 1) % cols;
                geo.add_face(&[
                    ring_pts[r][c],
                    ring_pts[r + 1][c],
                    ring_pts[r + 1][next],
                    ring_pts[r][next],
                ]);
            }
        }

        // Bottom cap triangles
        let last_ring = ring_pts.len() - 1;
        for c in 0..cols {
            let next = (c + 1) % cols;
            geo.add_face(&[ring_pts[last_ring][c], bottom, ring_pts[last_ring][next]]);
        }

        Ok(geo)
    }

    fn input_count(&self) -> (usize, usize) { (0, 0) }
    fn name(&self) -> &'static str { "sphere" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_default() {
        let geo = SphereSop.execute(&[], &SphereParams::default()).unwrap();
        // 2 poles + (12-1)*24 = 2 + 264 = 266 points
        assert_eq!(geo.num_points(), 266);
        // 24 top tris + 10*24 middle quads + 24 bottom tris = 24+240+24 = 288
        assert_eq!(geo.num_prims(), 288);
    }

    #[test]
    fn test_sphere_bounding_box() {
        let geo = SphereSop.execute(&[], &SphereParams::default()).unwrap();
        let bbox = geo.bounding_box();
        // Should be approximately [-0.5, -0.5, -0.5] to [0.5, 0.5, 0.5]
        assert!((bbox.min.y - (-0.5)).abs() < 1e-5);
        assert!((bbox.max.y - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_sphere_minimal() {
        let params = SphereParams {
            rows: 2,
            cols: 3,
            ..Default::default()
        };
        let geo = SphereSop.execute(&[], &params).unwrap();
        // 2 poles + 1 ring of 3 = 5 points
        assert_eq!(geo.num_points(), 5);
        // 3 top tris + 3 bottom tris = 6
        assert_eq!(geo.num_prims(), 6);
    }
}
```

- [ ] **Step 4: Implement Tube SOP**

```rust
// crates/procgeo-sops/src/creation/tube.rs

use glam::Vec3;
use procgeo_core::{Geometry, PointHandle};
use crate::{Sop, SopError};

/// Whether the tube has end caps.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum TubeCap {
    #[default]
    None,
    Top,
    Bottom,
    Both,
}

/// Parameters for the Tube SOP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TubeParams {
    /// Radius at the bottom.
    pub radius_bottom: f32,
    /// Radius at the top.
    pub radius_top: f32,
    /// Height of the tube.
    pub height: f32,
    /// Center position.
    pub center: Vec3,
    /// Number of divisions around the circumference.
    pub cols: u32,
    /// Number of rows along the height.
    pub rows: u32,
    /// End caps.
    pub caps: TubeCap,
}

impl Default for TubeParams {
    fn default() -> Self {
        Self {
            radius_bottom: 0.5,
            radius_top: 0.5,
            height: 1.0,
            center: Vec3::ZERO,
            cols: 24,
            rows: 2,
            caps: TubeCap::None,
        }
    }
}

pub struct TubeSop;

impl Sop for TubeSop {
    type Params = TubeParams;

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        if params.cols < 3 || params.rows < 2 {
            return Err(SopError::InvalidParam("cols >= 3 and rows >= 2 required".to_string()));
        }

        let cols = params.cols as usize;
        let rows = params.rows as usize;
        let half_h = params.height * 0.5;
        let mut geo = Geometry::new();

        // Generate ring points
        let mut rings: Vec<Vec<PointHandle>> = Vec::with_capacity(rows);
        for r in 0..rows {
            let t = r as f32 / (rows - 1) as f32;
            let y = -half_h + t * params.height;
            let radius = params.radius_bottom + t * (params.radius_top - params.radius_bottom);
            let mut ring = Vec::with_capacity(cols);
            for c in 0..cols {
                let angle = 2.0 * std::f32::consts::PI * c as f32 / cols as f32;
                let pos = params.center + Vec3::new(
                    radius * angle.cos(),
                    y,
                    radius * angle.sin(),
                );
                ring.push(geo.add_point(pos));
            }
            rings.push(ring);
        }

        // Quad faces between rings
        for r in 0..(rows - 1) {
            for c in 0..cols {
                let next = (c + 1) % cols;
                geo.add_face(&[
                    rings[r][c],
                    rings[r][next],
                    rings[r + 1][next],
                    rings[r + 1][c],
                ]);
            }
        }

        // Caps
        let cap_bottom = matches!(params.caps, TubeCap::Bottom | TubeCap::Both);
        let cap_top = matches!(params.caps, TubeCap::Top | TubeCap::Both);

        if cap_bottom {
            let center_pt = geo.add_point(params.center + Vec3::new(0.0, -half_h, 0.0));
            for c in 0..cols {
                let next = (c + 1) % cols;
                geo.add_face(&[center_pt, rings[0][next], rings[0][c]]);
            }
        }

        if cap_top {
            let center_pt = geo.add_point(params.center + Vec3::new(0.0, half_h, 0.0));
            for c in 0..cols {
                let next = (c + 1) % cols;
                geo.add_face(&[center_pt, rings[rows - 1][c], rings[rows - 1][next]]);
            }
        }

        Ok(geo)
    }

    fn input_count(&self) -> (usize, usize) { (0, 0) }
    fn name(&self) -> &'static str { "tube" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tube_default() {
        let geo = TubeSop.execute(&[], &TubeParams::default()).unwrap();
        // 2 rows × 24 cols = 48 points, 24 quads
        assert_eq!(geo.num_points(), 48);
        assert_eq!(geo.num_prims(), 24);
    }

    #[test]
    fn test_tube_with_caps() {
        let params = TubeParams {
            caps: TubeCap::Both,
            cols: 8,
            rows: 2,
            ..Default::default()
        };
        let geo = TubeSop.execute(&[], &params).unwrap();
        // 16 ring pts + 2 cap centers = 18, 8 side quads + 8 top tris + 8 bottom tris = 24
        assert_eq!(geo.num_points(), 18);
        assert_eq!(geo.num_prims(), 24);
    }

    #[test]
    fn test_tube_cone() {
        let params = TubeParams {
            radius_top: 0.0,
            radius_bottom: 1.0,
            cols: 6,
            rows: 2,
            ..Default::default()
        };
        let geo = TubeSop.execute(&[], &params).unwrap();
        // Top ring points all at (0,y,0) since radius_top=0
        let top_ring_start = 6; // second ring starts at index 6
        for i in top_ring_start..12 {
            let pos = geo.point_pos(PointHandle::from_index(i));
            assert!((pos.x).abs() < 1e-6);
            assert!((pos.z).abs() < 1e-6);
        }
    }
}
```

- [ ] **Step 5: Implement Torus SOP**

```rust
// crates/procgeo-sops/src/creation/torus.rs

use glam::Vec3;
use procgeo_core::Geometry;
use crate::{Sop, SopError};

/// Parameters for the Torus SOP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TorusParams {
    /// Major radius (center of tube to center of torus).
    pub radius_outer: f32,
    /// Minor radius (radius of the tube).
    pub radius_inner: f32,
    /// Center position.
    pub center: Vec3,
    /// Number of rows (around the tube cross-section).
    pub rows: u32,
    /// Number of columns (around the ring).
    pub cols: u32,
}

impl Default for TorusParams {
    fn default() -> Self {
        Self {
            radius_outer: 1.0,
            radius_inner: 0.3,
            center: Vec3::ZERO,
            rows: 12,
            cols: 24,
        }
    }
}

pub struct TorusSop;

impl Sop for TorusSop {
    type Params = TorusParams;

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        if params.rows < 3 || params.cols < 3 {
            return Err(SopError::InvalidParam("rows and cols must be >= 3".to_string()));
        }

        let rows = params.rows as usize;
        let cols = params.cols as usize;
        let num_points = rows * cols;
        let mut geo = Geometry::with_capacity(num_points, num_points);

        // Generate points
        for c in 0..cols {
            let theta = 2.0 * std::f32::consts::PI * c as f32 / cols as f32;
            let (sin_theta, cos_theta) = theta.sin_cos();
            for r in 0..rows {
                let phi = 2.0 * std::f32::consts::PI * r as f32 / rows as f32;
                let (sin_phi, cos_phi) = phi.sin_cos();
                let x = (params.radius_outer + params.radius_inner * cos_phi) * cos_theta;
                let y = params.radius_inner * sin_phi;
                let z = (params.radius_outer + params.radius_inner * cos_phi) * sin_theta;
                geo.add_point(params.center + Vec3::new(x, y, z));
            }
        }

        // Generate quad faces
        for c in 0..cols {
            let next_c = (c + 1) % cols;
            for r in 0..rows {
                let next_r = (r + 1) % rows;
                let i00 = procgeo_core::PointHandle::from_index(c * rows + r);
                let i01 = procgeo_core::PointHandle::from_index(c * rows + next_r);
                let i10 = procgeo_core::PointHandle::from_index(next_c * rows + r);
                let i11 = procgeo_core::PointHandle::from_index(next_c * rows + next_r);
                geo.add_face(&[i00, i10, i11, i01]);
            }
        }

        Ok(geo)
    }

    fn input_count(&self) -> (usize, usize) { (0, 0) }
    fn name(&self) -> &'static str { "torus" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_torus_default() {
        let geo = TorusSop.execute(&[], &TorusParams::default()).unwrap();
        // 12 rows × 24 cols = 288 points, 288 quads
        assert_eq!(geo.num_points(), 288);
        assert_eq!(geo.num_prims(), 288);
    }

    #[test]
    fn test_torus_minimal() {
        let params = TorusParams {
            rows: 3,
            cols: 3,
            ..Default::default()
        };
        let geo = TorusSop.execute(&[], &params).unwrap();
        assert_eq!(geo.num_points(), 9);
        assert_eq!(geo.num_prims(), 9);
    }

    #[test]
    fn test_torus_symmetry() {
        let geo = TorusSop.execute(&[], &TorusParams::default()).unwrap();
        let bbox = geo.bounding_box();
        // Torus should be symmetric about center
        assert!((bbox.center() - Vec3::ZERO).length() < 0.1);
    }
}
```

- [ ] **Step 6: Update creation/mod.rs**

```rust
// crates/procgeo-sops/src/creation/mod.rs

mod box_sop;
mod circle;
mod grid;
mod line;
mod sphere;
mod torus;
mod tube;

pub use box_sop::{BoxParams, BoxSop};
pub use circle::{CircleParams, CircleSop};
pub use grid::{GridOrientation, GridParams, GridSop};
pub use line::{LineParams, LineSop};
pub use sphere::{SphereParams, SphereSop};
pub use torus::{TorusParams, TorusSop};
pub use tube::{TubeCap, TubeParams, TubeSop};
```

- [ ] **Step 7: Run all tests**

Run: `cargo test -p procgeo-sops`
Expected: All creation SOP tests pass.

- [ ] **Step 8: Commit**

```bash
git add crates/procgeo-sops/src/creation/
git commit -m "feat(sops): add Line, Circle, Sphere, Tube, Torus creation SOPs"
```

---

### Task 13: Transform, Normal, and Merge SOPs

The essential manipulation SOPs.

**Files:**
- Create: `crates/procgeo-sops/src/transform/transform_sop.rs`
- Create: `crates/procgeo-sops/src/normals/normal.rs`
- Create: `crates/procgeo-sops/src/merge/merge.rs`
- Modify: `crates/procgeo-sops/src/transform/mod.rs`
- Modify: `crates/procgeo-sops/src/normals/mod.rs`
- Modify: `crates/procgeo-sops/src/merge/mod.rs`

- [ ] **Step 1: Implement Transform SOP**

```rust
// crates/procgeo-sops/src/transform/transform_sop.rs

use glam::{Mat4, Quat, Vec3};
use procgeo_core::Geometry;
use crate::{Sop, SopError};

/// Parameters for the Transform SOP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TransformParams {
    /// Translation.
    pub translate: Vec3,
    /// Rotation in degrees (Euler angles: X, Y, Z).
    pub rotate: Vec3,
    /// Scale.
    pub scale: Vec3,
    /// Pivot point for rotation and scale.
    pub pivot: Vec3,
}

impl Default for TransformParams {
    fn default() -> Self {
        Self {
            translate: Vec3::ZERO,
            rotate: Vec3::ZERO,
            scale: Vec3::ONE,
            pivot: Vec3::ZERO,
        }
    }
}

pub struct TransformSop;

impl Sop for TransformSop {
    type Params = TransformParams;

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        let input = inputs[0];
        let mut geo = input.clone();

        // Build transform matrix: translate to pivot, scale, rotate, translate back, then translate
        let rot_rad = params.rotate * (std::f32::consts::PI / 180.0);
        let rotation = Quat::from_euler(glam::EulerRot::XYZ, rot_rad.x, rot_rad.y, rot_rad.z);

        let mat = Mat4::from_translation(params.translate)
            * Mat4::from_translation(params.pivot)
            * Mat4::from_quat(rotation)
            * Mat4::from_scale(params.scale)
            * Mat4::from_translation(-params.pivot);

        // Apply transform to all points
        for ph in geo.points().collect::<Vec<_>>() {
            let pos = geo.point_pos(ph);
            let transformed = mat.transform_point3(pos);
            geo.set_point_pos(ph, transformed);
        }

        Ok(geo)
    }

    fn input_count(&self) -> (usize, usize) { (1, 1) }
    fn name(&self) -> &'static str { "transform" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::{BoxSop, BoxParams};
    use crate::Sop;

    #[test]
    fn test_transform_translate() {
        let geo = BoxSop.execute(&[], &BoxParams::default()).unwrap();
        let params = TransformParams {
            translate: Vec3::new(10.0, 0.0, 0.0),
            ..Default::default()
        };
        let result = TransformSop.execute(&[&geo], &params).unwrap();
        let bbox = result.bounding_box();
        assert!((bbox.center().x - 10.0).abs() < 1e-5);
    }

    #[test]
    fn test_transform_scale() {
        let geo = BoxSop.execute(&[], &BoxParams::default()).unwrap();
        let params = TransformParams {
            scale: Vec3::splat(2.0),
            ..Default::default()
        };
        let result = TransformSop.execute(&[&geo], &params).unwrap();
        let bbox = result.bounding_box();
        assert!((bbox.size().x - 2.0).abs() < 1e-5);
    }

    #[test]
    fn test_transform_identity() {
        let geo = BoxSop.execute(&[], &BoxParams::default()).unwrap();
        let result = TransformSop.execute(&[&geo], &TransformParams::default()).unwrap();
        // Should be unchanged
        assert_eq!(result.num_points(), geo.num_points());
        let orig_bbox = geo.bounding_box();
        let new_bbox = result.bounding_box();
        assert!((orig_bbox.center() - new_bbox.center()).length() < 1e-5);
    }

    #[test]
    fn test_transform_requires_input() {
        let result = TransformSop.execute(&[], &TransformParams::default());
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Implement Normal SOP**

This requires `Clone` on `Geometry`. Add `#[derive(Clone)]` or manual impl.

```rust
// crates/procgeo-sops/src/normals/normal.rs

use glam::Vec3;
use procgeo_core::{AttribClass, AttribDefault, Geometry, Primitive, TypeQualifier};
use crate::{Sop, SopError};

/// Parameters for the Normal SOP.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NormalParams {
    // Future: weighting mode, cusp angle, etc.
}

pub struct NormalSop;

impl Sop for NormalSop {
    type Params = NormalParams;

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        let input = inputs[0];
        let mut geo = input.clone();

        // Create N attribute on points
        geo.add_attrib(
            AttribClass::Point,
            "N",
            AttribDefault::Vector3([0.0, 0.0, 0.0]),
            TypeQualifier::Normal,
        ).map_err(SopError::Core)?;

        let n_handle = geo.find_attrib::<[f32; 3]>(AttribClass::Point, "N")
            .map_err(SopError::Core)?;

        // Accumulate face normals onto points (area-weighted)
        for ph in geo.prims().collect::<Vec<_>>() {
            let pts = geo.prim_points(ph);
            if pts.len() < 3 {
                continue;
            }

            // Compute face normal using Newell's method for arbitrary polygons
            let mut face_normal = Vec3::ZERO;
            let n = pts.len();
            for i in 0..n {
                let curr = geo.point_pos(pts[i]);
                let next = geo.point_pos(pts[(i + 1) % n]);
                face_normal.x += (curr.y - next.y) * (curr.z + next.z);
                face_normal.y += (curr.z - next.z) * (curr.x + next.x);
                face_normal.z += (curr.x - next.x) * (curr.y + next.y);
            }
            // Don't normalize yet — length represents area (area weighting)

            // Accumulate onto each point of this face
            for &pt in &pts {
                let existing = *geo.get_attrib(&n_handle, pt.index()).unwrap();
                let accumulated = Vec3::from(existing) + face_normal;
                geo.set_attrib(&n_handle, pt.index(), accumulated.to_array());
            }
        }

        // Normalize all point normals
        for ph in geo.points().collect::<Vec<_>>() {
            let n = Vec3::from(*geo.get_attrib(&n_handle, ph.index()).unwrap());
            let normalized = n.normalize_or_zero();
            geo.set_attrib(&n_handle, ph.index(), normalized.to_array());
        }

        Ok(geo)
    }

    fn input_count(&self) -> (usize, usize) { (1, 1) }
    fn name(&self) -> &'static str { "normal" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::{BoxSop, BoxParams, GridSop, GridParams};
    use crate::Sop;

    #[test]
    fn test_normal_on_grid() {
        let grid = GridSop.execute(&[], &GridParams {
            rows: 3,
            cols: 3,
            size: [2.0, 2.0],
            ..Default::default()
        }).unwrap();

        let geo = NormalSop.execute(&[&grid], &NormalParams::default()).unwrap();

        // Grid on XZ plane — all normals should point up (+Y)
        let n_handle = geo.find_attrib::<[f32; 3]>(AttribClass::Point, "N").unwrap();
        for ph in geo.points() {
            let n = Vec3::from(*geo.get_attrib(&n_handle, ph.index()).unwrap());
            assert!((n.y.abs() - 1.0).abs() < 1e-5, "normal Y should be ±1, got {:?}", n);
            assert!(n.x.abs() < 1e-5);
            assert!(n.z.abs() < 1e-5);
        }
    }

    #[test]
    fn test_normal_on_box() {
        let box_geo = BoxSop.execute(&[], &BoxParams::default()).unwrap();
        let geo = NormalSop.execute(&[&box_geo], &NormalParams::default()).unwrap();

        // All normals should be unit length
        let n_handle = geo.find_attrib::<[f32; 3]>(AttribClass::Point, "N").unwrap();
        for ph in geo.points() {
            let n = Vec3::from(*geo.get_attrib(&n_handle, ph.index()).unwrap());
            assert!((n.length() - 1.0).abs() < 1e-4, "normal should be unit length, got {}", n.length());
        }
    }
}
```

- [ ] **Step 3: Implement Merge SOP**

```rust
// crates/procgeo-sops/src/merge/merge.rs

use procgeo_core::{Geometry, PointHandle, PolyType, Primitive};
use crate::{Sop, SopError};

/// Parameters for the Merge SOP.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MergeParams {
    // Future: merge mode, attribute conflict resolution
}

pub struct MergeSop;

impl Sop for MergeSop {
    type Params = MergeParams;

    fn execute(&self, inputs: &[&Geometry], _params: &Self::Params) -> Result<Geometry, SopError> {
        if inputs.is_empty() {
            return Ok(Geometry::new());
        }

        // Count total points and prims for pre-allocation
        let total_points: usize = inputs.iter().map(|g| g.num_points()).sum();
        let total_prims: usize = inputs.iter().map(|g| g.num_prims()).sum();
        let mut geo = Geometry::with_capacity(total_points, total_prims);

        for input in inputs {
            // Track point offset for this input
            let point_offset = geo.num_points();

            // Copy all points
            for ph in input.points() {
                geo.add_point(input.point_pos(ph));
            }

            // Copy all primitives, remapping point indices
            for ph in input.prims() {
                let orig_pts = input.prim_points(ph);
                let remapped: Vec<PointHandle> = orig_pts
                    .iter()
                    .map(|p| PointHandle::from_index(p.index() + point_offset))
                    .collect();

                match input.prim(ph) {
                    Primitive::Polygon(poly) => {
                        geo.add_polygon(&remapped, poly.poly_type);
                    }
                }
            }
        }

        Ok(geo)
    }

    fn input_count(&self) -> (usize, usize) { (0, usize::MAX) }
    fn name(&self) -> &'static str { "merge" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::{BoxSop, BoxParams, GridSop, GridParams};
    use crate::Sop;

    #[test]
    fn test_merge_two_boxes() {
        let box1 = BoxSop.execute(&[], &BoxParams::default()).unwrap();
        let box2 = BoxSop.execute(&[], &BoxParams {
            center: glam::Vec3::new(5.0, 0.0, 0.0),
            ..Default::default()
        }).unwrap();

        let merged = MergeSop.execute(&[&box1, &box2], &MergeParams::default()).unwrap();
        assert_eq!(merged.num_points(), 16); // 8 + 8
        assert_eq!(merged.num_prims(), 12); // 6 + 6
    }

    #[test]
    fn test_merge_empty() {
        let geo = MergeSop.execute(&[], &MergeParams::default()).unwrap();
        assert_eq!(geo.num_points(), 0);
    }

    #[test]
    fn test_merge_single() {
        let box1 = BoxSop.execute(&[], &BoxParams::default()).unwrap();
        let merged = MergeSop.execute(&[&box1], &MergeParams::default()).unwrap();
        assert_eq!(merged.num_points(), 8);
        assert_eq!(merged.num_prims(), 6);
    }

    #[test]
    fn test_merge_preserves_topology() {
        let grid = GridSop.execute(&[], &GridParams {
            rows: 2, cols: 2, size: [1.0, 1.0], ..Default::default()
        }).unwrap();
        let box1 = BoxSop.execute(&[], &BoxParams::default()).unwrap();

        let merged = MergeSop.execute(&[&grid, &box1], &MergeParams::default()).unwrap();

        // Check that the first prim (from grid) has correct remapped points
        let first_prim_pts = merged.prim_points(procgeo_core::PrimHandle::from_index(0));
        assert!(first_prim_pts.iter().all(|p| p.index() < 4)); // grid has 4 points

        // Second set of prims should reference offset points
        let box_prim = merged.prim_points(procgeo_core::PrimHandle::from_index(1));
        assert!(box_prim.iter().all(|p| p.index() >= 4));
    }
}
```

- [ ] **Step 4: Add Clone to Geometry**

The Transform and Normal SOPs need `geo.clone()`. Add Clone derives/impls to Geometry and its storage types.

In `crates/procgeo-core/src/point.rs`, add `#[derive(Clone)]` on `PointStorage` (or `impl Clone`).
In `crates/procgeo-core/src/vertex.rs`, add `#[derive(Clone)]` on `VertexStorage`.
In `crates/procgeo-core/src/primitive.rs`, add `#[derive(Clone)]` on `PrimStorage`.
In `crates/procgeo-core/src/geometry.rs`, add `#[derive(Clone)]` on `Geometry` (or manual impl since it contains non-derive types).

Since all fields already implement Clone (PointStorage, VertexStorage, PrimStorage, AttributeMap, GroupMap), just derive Clone:

```rust
// In point.rs, add before struct:
#[derive(Clone)]
pub struct PointStorage { ... }

// In vertex.rs:
#[derive(Clone)]
pub struct VertexStorage { ... }

// In primitive.rs:
#[derive(Clone)]
pub struct PrimStorage { ... }

// In geometry.rs:
#[derive(Clone)]
pub struct Geometry { ... }
```

- [ ] **Step 5: Update module files**

`crates/procgeo-sops/src/transform/mod.rs`:
```rust
mod transform_sop;
pub use transform_sop::{TransformParams, TransformSop};
```

`crates/procgeo-sops/src/normals/mod.rs`:
```rust
mod normal;
pub use normal::{NormalParams, NormalSop};
```

`crates/procgeo-sops/src/merge/mod.rs`:
```rust
mod merge;
pub use merge::{MergeParams, MergeSop};
```

- [ ] **Step 6: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass across all crates.

- [ ] **Step 7: Commit**

```bash
git add crates/
git commit -m "feat(sops): add Transform, Normal, and Merge SOPs"
```

---

### Task 14: OBJ I/O

**Files:**
- Modify: `crates/procgeo-io/src/lib.rs`
- Create: `crates/procgeo-io/src/obj.rs`

- [ ] **Step 1: Implement I/O traits and OBJ writer/reader**

```rust
// crates/procgeo-io/src/lib.rs

use procgeo_core::Geometry;
use thiserror::Error;
use std::io::{Read, Write};
use std::path::Path;

#[cfg(feature = "obj")]
pub mod obj;

#[derive(Debug, Error)]
pub enum IoError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
}

/// Trait for writing geometry to a format.
pub trait GeometryWriter {
    fn write(&self, geo: &Geometry, writer: &mut dyn Write) -> Result<(), IoError>;
    fn extensions(&self) -> &[&str];
}

/// Trait for reading geometry from a format.
pub trait GeometryReader {
    fn read(&self, reader: &mut dyn Read) -> Result<Geometry, IoError>;
    fn extensions(&self) -> &[&str];
}

/// Write geometry to a file, detecting format from extension.
pub fn write_file(geo: &Geometry, path: &Path) -> Result<(), IoError> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| IoError::UnsupportedFormat("no extension".to_string()))?;

    match ext {
        #[cfg(feature = "obj")]
        "obj" => {
            let mut file = std::fs::File::create(path)?;
            obj::ObjWriter.write(geo, &mut file)
        }
        _ => Err(IoError::UnsupportedFormat(ext.to_string())),
    }
}

/// Read geometry from a file, detecting format from extension.
pub fn read_file(path: &Path) -> Result<Geometry, IoError> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| IoError::UnsupportedFormat("no extension".to_string()))?;

    match ext {
        #[cfg(feature = "obj")]
        "obj" => {
            let mut file = std::fs::File::open(path)?;
            obj::ObjReader.read(&mut file)
        }
        _ => Err(IoError::UnsupportedFormat(ext.to_string())),
    }
}
```

```rust
// crates/procgeo-io/src/obj.rs

use std::io::{BufRead, BufReader, Read, Write};
use procgeo_core::{Geometry, PointHandle, Primitive, PolyType};
use crate::{GeometryReader, GeometryWriter, IoError};

/// Wavefront OBJ writer.
pub struct ObjWriter;

impl GeometryWriter for ObjWriter {
    fn write(&self, geo: &Geometry, writer: &mut dyn Write) -> Result<(), IoError> {
        writeln!(writer, "# ProcGeo OBJ export")?;
        writeln!(writer, "# Points: {} Primitives: {}", geo.num_points(), geo.num_prims())?;

        // Write vertices
        for ph in geo.points() {
            let p = geo.point_pos(ph);
            writeln!(writer, "v {} {} {}", p.x, p.y, p.z)?;
        }

        // Write normals if N attribute exists
        let has_normals = geo.find_attrib::<[f32; 3]>(
            procgeo_core::AttribClass::Point, "N"
        ).is_ok();

        if has_normals {
            let n_handle = geo.find_attrib::<[f32; 3]>(
                procgeo_core::AttribClass::Point, "N"
            ).unwrap();
            for ph in geo.points() {
                let n = geo.get_attrib(&n_handle, ph.index()).unwrap();
                writeln!(writer, "vn {} {} {}", n[0], n[1], n[2])?;
            }
        }

        // Write faces (OBJ uses 1-based indices)
        for ph in geo.prims() {
            let prim = geo.prim(ph);
            match prim {
                Primitive::Polygon(poly) => {
                    if poly.poly_type == PolyType::Closed {
                        write!(writer, "f")?;
                        let pts = geo.prim_points(ph);
                        for pt in &pts {
                            let idx = pt.index() + 1; // 1-based
                            if has_normals {
                                write!(writer, " {}//{}", idx, idx)?;
                            } else {
                                write!(writer, " {}", idx)?;
                            }
                        }
                        writeln!(writer)?;
                    } else {
                        // Open polyline — write as 'l' command
                        write!(writer, "l")?;
                        let pts = geo.prim_points(ph);
                        for pt in &pts {
                            write!(writer, " {}", pt.index() + 1)?;
                        }
                        writeln!(writer)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn extensions(&self) -> &[&str] {
        &["obj"]
    }
}

/// Wavefront OBJ reader.
pub struct ObjReader;

impl GeometryReader for ObjReader {
    fn read(&self, reader: &mut dyn Read) -> Result<Geometry, IoError> {
        let buf = BufReader::new(reader);
        let mut geo = Geometry::new();

        for line in buf.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "v" => {
                    if parts.len() < 4 {
                        return Err(IoError::Parse("vertex needs 3 components".to_string()));
                    }
                    let x: f32 = parts[1].parse().map_err(|e| IoError::Parse(format!("{}", e)))?;
                    let y: f32 = parts[2].parse().map_err(|e| IoError::Parse(format!("{}", e)))?;
                    let z: f32 = parts[3].parse().map_err(|e| IoError::Parse(format!("{}", e)))?;
                    geo.add_point(glam::Vec3::new(x, y, z));
                }
                "f" => {
                    let mut face_pts = Vec::new();
                    for part in &parts[1..] {
                        // Handle formats: "1", "1/2", "1/2/3", "1//3"
                        let idx_str = part.split('/').next().unwrap();
                        let idx: usize = idx_str.parse::<usize>()
                            .map_err(|e| IoError::Parse(format!("{}", e)))?;
                        face_pts.push(PointHandle::from_index(idx - 1)); // 0-based
                    }
                    geo.add_face(&face_pts);
                }
                "l" => {
                    let mut line_pts = Vec::new();
                    for part in &parts[1..] {
                        let idx: usize = part.parse::<usize>()
                            .map_err(|e| IoError::Parse(format!("{}", e)))?;
                        line_pts.push(PointHandle::from_index(idx - 1));
                    }
                    geo.add_polyline(&line_pts);
                }
                _ => {} // skip vn, vt, etc. for now
            }
        }

        Ok(geo)
    }

    fn extensions(&self) -> &[&str] {
        &["obj"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_triangle() -> Geometry {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(glam::Vec3::new(0.0, 0.0, 0.0));
        let p1 = geo.add_point(glam::Vec3::new(1.0, 0.0, 0.0));
        let p2 = geo.add_point(glam::Vec3::new(0.0, 1.0, 0.0));
        geo.add_face(&[p0, p1, p2]);
        geo
    }

    #[test]
    fn test_obj_write() {
        let geo = make_triangle();
        let mut buf = Vec::new();
        ObjWriter.write(&geo, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("v 0 0 0"));
        assert!(output.contains("v 1 0 0"));
        assert!(output.contains("v 0 1 0"));
        assert!(output.contains("f 1 2 3"));
    }

    #[test]
    fn test_obj_roundtrip() {
        let geo = make_triangle();
        let mut buf = Vec::new();
        ObjWriter.write(&geo, &mut buf).unwrap();

        let read_geo = ObjReader.read(&mut buf.as_slice()).unwrap();
        assert_eq!(read_geo.num_points(), 3);
        assert_eq!(read_geo.num_prims(), 1);
        assert_eq!(read_geo.num_vertices(), 3);

        // Check positions match
        for i in 0..3 {
            let orig = geo.point_pos(PointHandle::from_index(i));
            let read = read_geo.point_pos(PointHandle::from_index(i));
            assert!((orig - read).length() < 1e-5);
        }
    }

    #[test]
    fn test_obj_roundtrip_box() {
        use procgeo_sops::creation::{BoxSop, BoxParams};
        use procgeo_sops::Sop;

        let geo = BoxSop.execute(&[], &BoxParams::default()).unwrap();
        let mut buf = Vec::new();
        ObjWriter.write(&geo, &mut buf).unwrap();

        let read_geo = ObjReader.read(&mut buf.as_slice()).unwrap();
        assert_eq!(read_geo.num_points(), 8);
        assert_eq!(read_geo.num_prims(), 6);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass including I/O roundtrip.

- [ ] **Step 3: Commit**

```bash
git add crates/procgeo-io/
git commit -m "feat(io): add OBJ reader/writer with roundtrip support"
```

---

### Task 15: Umbrella Crate and Integration Tests

**Files:**
- Modify: `crates/procgeo/src/lib.rs`
- Create: `tests/integration.rs`

- [ ] **Step 1: Set up the umbrella crate**

```rust
// crates/procgeo/src/lib.rs

//! ProcGeo — Procedural geometry library inspired by Houdini SOPs.
//!
//! # Quick Start
//! ```rust
//! use procgeo::prelude::*;
//!
//! let geo = BoxSop.execute(&[], &BoxParams::default()).unwrap();
//! assert_eq!(geo.num_points(), 8);
//! ```

pub use procgeo_core as core;
pub use procgeo_sops as sops;
pub use procgeo_io as io;

/// Convenience prelude importing the most common types.
pub mod prelude {
    pub use procgeo_core::{
        AttribClass, AttribDefault, AttribHandle, AttribValue, Geometry,
        PointHandle, PrimHandle, VertexHandle, PolyType, TypeQualifier,
    };
    pub use procgeo_core::math::BBox;
    pub use procgeo_sops::{Sop, SopError, generate};

    #[cfg(feature = "creation")]
    pub use procgeo_sops::creation::*;
    #[cfg(feature = "transform")]
    pub use procgeo_sops::transform::*;
    #[cfg(feature = "normals")]
    pub use procgeo_sops::normals::*;
    #[cfg(feature = "merge")]
    pub use procgeo_sops::merge::*;
}
```

Update `crates/procgeo/Cargo.toml` to forward features:
```toml
[package]
name = "procgeo"
version.workspace = true
edition.workspace = true

[dependencies]
procgeo-core = { path = "../procgeo-core" }
procgeo-sops = { path = "../procgeo-sops" }
procgeo-io = { path = "../procgeo-io" }

[features]
default = ["creation", "transform", "normals", "merge", "obj"]
creation = ["procgeo-sops/creation"]
transform = ["procgeo-sops/transform"]
normals = ["procgeo-sops/normals"]
merge = ["procgeo-sops/merge"]
obj = ["procgeo-io/obj"]
```

- [ ] **Step 2: Write integration tests**

```rust
// tests/integration.rs
// (This file lives at the workspace root: rs-procgeo/tests/integration.rs)
// Add to root Cargo.toml or use procgeo as dependency.

// Actually, integration tests should go in crates/procgeo/tests/integration.rs

```

Create `crates/procgeo/tests/integration.rs`:

```rust
use procgeo::prelude::*;
use glam::Vec3;

#[test]
fn test_box_to_obj_roundtrip() {
    let geo = BoxSop.execute(&[], &BoxParams::default()).unwrap();
    assert_eq!(geo.num_points(), 8);
    assert_eq!(geo.num_prims(), 6);

    let mut buf = Vec::new();
    procgeo::io::obj::ObjWriter.write(&geo, &mut buf).unwrap();
    let read_geo = procgeo::io::obj::ObjReader.read(&mut buf.as_slice()).unwrap();

    assert_eq!(read_geo.num_points(), 8);
    assert_eq!(read_geo.num_prims(), 6);
}

#[test]
fn test_sop_chaining() {
    // Box → Transform → Normal
    let geo = BoxSop.execute(&[], &BoxParams::default()).unwrap();
    let geo = TransformSop.execute(&[&geo], &TransformParams {
        translate: Vec3::new(5.0, 0.0, 0.0),
        scale: Vec3::splat(2.0),
        ..Default::default()
    }).unwrap();
    let geo = NormalSop.execute(&[&geo], &NormalParams::default()).unwrap();

    assert_eq!(geo.num_points(), 8);
    let bbox = geo.bounding_box();
    assert!((bbox.center().x - 5.0).abs() < 1e-4);
    assert!((bbox.size().x - 2.0).abs() < 1e-4);

    // Check normals exist and are unit length
    let n_handle = geo.find_attrib::<[f32; 3]>(AttribClass::Point, "N").unwrap();
    for ph in geo.points() {
        let n = Vec3::from(*geo.get_attrib(&n_handle, ph.index()).unwrap());
        assert!((n.length() - 1.0).abs() < 1e-3);
    }
}

#[test]
fn test_merge_different_sops() {
    let box_geo = BoxSop.execute(&[], &BoxParams::default()).unwrap();
    let grid_geo = GridSop.execute(&[], &GridParams {
        rows: 3,
        cols: 3,
        size: [2.0, 2.0],
        ..Default::default()
    }).unwrap();
    let sphere_geo = SphereSop.execute(&[], &SphereParams {
        rows: 4,
        cols: 6,
        ..Default::default()
    }).unwrap();

    let merged = MergeSop.execute(
        &[&box_geo, &grid_geo, &sphere_geo],
        &MergeParams::default(),
    ).unwrap();

    assert_eq!(
        merged.num_points(),
        box_geo.num_points() + grid_geo.num_points() + sphere_geo.num_points()
    );
    assert_eq!(
        merged.num_prims(),
        box_geo.num_prims() + grid_geo.num_prims() + sphere_geo.num_prims()
    );
}

#[test]
fn test_geometry_apply_chain() {
    let geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    let geo = geo.apply(&TransformSop, &TransformParams {
        translate: Vec3::new(1.0, 2.0, 3.0),
        ..Default::default()
    }).unwrap();

    let bbox = geo.bounding_box();
    assert!((bbox.center() - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-4);
}

#[test]
fn test_all_creation_sops_produce_valid_geometry() {
    // Smoke test: every creation SOP produces non-empty, valid geometry
    let tests: Vec<Box<dyn Fn() -> Geometry>> = vec![
        Box::new(|| BoxSop.execute(&[], &BoxParams::default()).unwrap()),
        Box::new(|| GridSop.execute(&[], &GridParams::default()).unwrap()),
        Box::new(|| LineSop.execute(&[], &LineParams::default()).unwrap()),
        Box::new(|| CircleSop.execute(&[], &CircleParams::default()).unwrap()),
        Box::new(|| SphereSop.execute(&[], &SphereParams::default()).unwrap()),
        Box::new(|| TubeSop.execute(&[], &TubeParams::default()).unwrap()),
        Box::new(|| TorusSop.execute(&[], &TorusParams::default()).unwrap()),
    ];

    for (i, make_geo) in tests.iter().enumerate() {
        let geo = make_geo();
        assert!(geo.num_points() > 0, "SOP {} produced 0 points", i);
        assert!(geo.num_prims() > 0, "SOP {} produced 0 prims", i);
        assert!(geo.num_vertices() > 0, "SOP {} produced 0 vertices", i);

        let bbox = geo.bounding_box();
        assert!(bbox.is_valid(), "SOP {} produced invalid bbox", i);

        // No NaN positions
        for ph in geo.points() {
            let p = geo.point_pos(ph);
            assert!(!p.x.is_nan() && !p.y.is_nan() && !p.z.is_nan(),
                "SOP {} has NaN position at point {}", i, ph.index());
        }
    }
}

#[test]
fn test_attributes_survive_sop_chain() {
    let mut geo = BoxSop.execute(&[], &BoxParams::default()).unwrap();

    // Add a custom attribute
    geo.add_attrib(
        AttribClass::Point,
        "id",
        procgeo_core::AttribDefault::Int(0),
        TypeQualifier::None,
    ).unwrap();

    let id_handle = geo.find_attrib::<i32>(AttribClass::Point, "id").unwrap();
    for ph in geo.points() {
        geo.set_attrib(&id_handle, ph.index(), ph.index() as i32);
    }

    // Transform should preserve attributes
    let geo = TransformSop.execute(&[&geo], &TransformParams {
        translate: Vec3::Y,
        ..Default::default()
    }).unwrap();

    let id_handle = geo.find_attrib::<i32>(AttribClass::Point, "id").unwrap();
    for ph in geo.points() {
        assert_eq!(*geo.get_attrib(&id_handle, ph.index()).unwrap(), ph.index() as i32);
    }
}
```

- [ ] **Step 3: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/procgeo/ tests/
git commit -m "feat: add umbrella crate with prelude and integration tests"
```

---

### Task 16: Add .gitignore and CLAUDE.md

**Files:**
- Create: `.gitignore`
- Create: `CLAUDE.md`

- [ ] **Step 1: Create .gitignore**

```
/target
Cargo.lock
*.obj
*.glb
*.gltf
.DS_Store
```

- [ ] **Step 2: Create CLAUDE.md**

```markdown
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
```

- [ ] **Step 3: Commit**

```bash
git add .gitignore CLAUDE.md
git commit -m "chore: add .gitignore and CLAUDE.md project guide"
```

---

## Summary

| Task | What it builds | Tests |
|------|---------------|-------|
| 1 | Cargo workspace + crate stubs | Compiles |
| 2 | Typed handles (PointHandle, etc.) | 4 tests |
| 3 | SoA point storage | 5 tests |
| 4 | Vertex + primitive storage | 5 tests |
| 5 | Full attribute system | 6 tests |
| 6 | Bitset groups | 7 tests |
| 7 | BBox + math utilities | 8 tests |
| 8 | Geometry struct | 8 tests |
| 9 | Sop trait + error types | Compiles |
| 10 | Box SOP | 5 tests |
| 11 | Grid SOP | 5 tests |
| 12 | Line, Circle, Sphere, Tube, Torus SOPs | 11 tests |
| 13 | Transform, Normal, Merge SOPs | 10 tests |
| 14 | OBJ reader/writer | 3 tests |
| 15 | Umbrella crate + integration tests | 6 tests |
| 16 | .gitignore + CLAUDE.md | — |

**Total: 16 tasks, ~83 tests, ~2,500 lines of Rust**
