---
name: procgeo-rust
description: Use when writing Rust code that creates, transforms, or manipulates procedural geometry using the procgeo library. Triggers on geometry generation, SOP chaining, attribute/group workflows, mesh construction, or any Houdini-style procedural modeling task.
---

# Procedural Geometry in Rust with ProcGeo

## Overview

ProcGeo is a Houdini-inspired procedural geometry library. Think in **networks**: geometry flows through a chain of stateless **SOPs** (Surface Operators), each reading inputs and producing new geometry. Variables are attributes living on elements. Logic is expressed by composing SOPs, not by writing imperative mesh code.

**Mental model:** You are wiring nodes in a Houdini network, not writing a mesh builder.

## Core Concepts

| Houdini | ProcGeo Rust | Notes |
|---------|-------------|-------|
| SOP node | `struct` implementing `Sop` trait | Stateless, params via `Params` struct |
| Wire between nodes | `.apply()` chain | Consumes geometry, returns `Result<Geometry>` |
| Generator node | `generate(&sop, &params)` | Creates geometry from nothing (0 inputs) |
| Multi-input node | `sop.execute(&[&geo1, &geo2], &params)` | Direct `execute()` call for 2+ inputs |
| Point/prim/vertex attrib | `AttribHandle<T>` + get/set | Type-safe, auto-resizes with geometry |
| Group | `ElementGroup` (bitset) | Named, boolean ops, used by Blast/GroupCreate |
| Detail attrib | `AttribClass::Detail` | Single value on the geometry itself |

## The Procedural Mindset

### 1. Generate, Don't Construct

```rust
use procgeo::prelude::*;
use glam::Vec3;

// YES: generate from a SOP
let box_geo = generate(&BoxSop, &BoxParams {
    size: Vec3::splat(2.0),
    ..Default::default()
}).unwrap();

// AVOID: manually adding points/faces for standard shapes
// (Reserve manual construction for custom topology only)
```

### 2. Chain SOPs with `.apply()`

Every SOP is a pure function: geometry in, geometry out. Chain them:

```rust
let geo = generate(&BoxSop, &BoxParams::default()).unwrap()
    .apply(&SubdivideSop, &SubdivideParams {
        depth: 2,
        mode: SubdivideMode::CatmullClark,
        ..Default::default()
    }).unwrap()
    .apply(&TransformSop, &TransformParams {
        translate: Vec3::new(0.0, 1.0, 0.0),
        scale: Vec3::splat(0.5),
        ..Default::default()
    }).unwrap()
    .apply(&NormalSop, &NormalParams).unwrap()
    .apply(&ColorSop, &ColorParams {
        color: [0.2, 0.6, 1.0],
    }).unwrap();
```

For multi-input SOPs (Merge, CopyToPoints), call `.execute()` directly:

```rust
// Merge any number of geometries
let merged = MergeSop.execute(&[&box_geo, &sphere_geo, &grid_geo], &MergeParams).unwrap();

// Copy source onto each point of target
let scattered = CopyToPointsSop.execute(
    &[&source_geo, &target_points],
    &CopyToPointsParams,
).unwrap();
```

### 3. Use Variables (Attributes), Not Local State

Attributes travel with the geometry through the chain. Use them instead of side-channel variables:

```rust
// Create a float attribute on points
geo.add_attrib(
    AttribClass::Point, "pscale",
    AttribDefault::Float(1.0),
    TypeQualifier::None,
).unwrap();

// Get a typed handle (compile-time type safety)
let pscale: AttribHandle<f32> = geo.find_attrib(AttribClass::Point, "pscale").unwrap();

// Read/write per element
for i in 0..geo.num_points() {
    let y = geo.point_pos(PointHandle::from_index(i)).y;
    geo.set_attrib(&pscale, i, y.abs()).unwrap();
}
```

**Common attribute types:**

| Type | Rust type | `AttribDefault` | Typical use |
|------|-----------|-----------------|-------------|
| `Float` | `f32` | `AttribDefault::Float(0.0)` | pscale, weight, density |
| `Int` | `i32` | `AttribDefault::Int(0)` | id, piece, copynum |
| `Vector3` | `[f32; 3]` | `AttribDefault::Vector3([0.0; 3])` | Cd, N, v |
| `String` | `String` | `AttribDefault::String("".into())` | name, path |

**Type qualifiers** give semantic meaning: `TypeQualifier::Color` for Cd, `TypeQualifier::Normal` for N, `TypeQualifier::None` for generic.

### 4. Select with Groups, Delete with Blast

Groups are named bitsets. Create them procedurally, then feed them to SOPs:

```rust
// Create group by bounding box
let geo = geo.apply(&GroupCreateSop, &GroupCreateParams {
    name: "bottom".into(),
    group_type: GroupType::Points,
    mode: GroupCreateMode::BoundingBox,
    bbox_min: Vec3::new(-10.0, -10.0, -10.0),
    bbox_max: Vec3::new(10.0, 0.0, 10.0),
    ..Default::default()
}).unwrap();

// Delete the group members
let geo = geo.apply(&BlastSop, &BlastParams {
    group_name: "bottom".into(),
    entity: BlastEntity::Points,
    negate: false, // true = keep only group members
}).unwrap();
```

Manual group manipulation for custom logic:

```rust
geo.create_point_group("border");
let n = 100; // grid side length
let grp = geo.groups_mut().point_group_mut("border").unwrap();
for i in 0..geo.num_points() {
    let x = i % n;
    let z = i / n;
    if x == 0 || x == n - 1 || z == 0 || z == n - 1 {
        grp.add(i);
    }
}

// Boolean ops between groups
let grp_a = geo.groups().point_group("A").unwrap().clone();
geo.groups_mut().point_group_mut("B").unwrap().union(&grp_a);
```

### 5. Build Procedural Functions as Reusable Pipelines

Compose SOPs into higher-level functions — this is how you build a procedural toolkit:

```rust
/// Create a rounded box: box -> subdivide -> smooth -> normals
fn rounded_box(size: Vec3, smoothing: u32) -> Result<Geometry, SopError> {
    generate(&BoxSop, &BoxParams { size, ..Default::default() })?
        .apply(&SubdivideSop, &SubdivideParams {
            depth: 2,
            mode: SubdivideMode::CatmullClark,
            ..Default::default()
        })?
        .apply(&SmoothSop, &SmoothParams {
            iterations: smoothing,
            strength: 0.5,
        })?
        .apply(&NormalSop, &NormalParams)
}

/// Scatter points on a surface with color
fn colored_scatter(surface: &Geometry, count: u32, color: [f32; 3]) -> Result<Geometry, SopError> {
    let points = surface.clone()
        .apply(&ScatterSop, &ScatterParams { count, seed: 42 })?
        .apply(&ColorSop, &ColorParams { color })?;
    Ok(points)
}

/// Instance a shape onto scattered points
fn instance_on_surface(
    shape: &Geometry,
    surface: &Geometry,
    count: u32,
) -> Result<Geometry, SopError> {
    let points = surface.clone()
        .apply(&ScatterSop, &ScatterParams { count, seed: 0 })?;
    CopyToPointsSop.execute(&[shape, &points], &CopyToPointsParams)
}
```

### 6. Custom Geometry When SOPs Aren't Enough

For procedural patterns that don't map to existing SOPs, build geometry manually:

```rust
/// Parametric spiral curve
fn spiral(turns: f32, points_per_turn: u32, radius: f32, height: f32) -> Geometry {
    let total_points = (turns * points_per_turn as f32) as usize;
    let mut geo = Geometry::with_capacity(total_points, 1);

    let mut handles = Vec::with_capacity(total_points);
    for i in 0..total_points {
        let t = i as f32 / points_per_turn as f32;
        let angle = t * std::f32::consts::TAU;
        let pos = Vec3::new(
            angle.cos() * radius,
            t / turns * height,
            angle.sin() * radius,
        );
        handles.push(geo.add_point(pos));
    }
    geo.add_polyline(&handles);
    geo
}
```

Then feed it back into the SOP chain:

```rust
let geo = spiral(3.0, 64, 1.0, 5.0)
    .apply(&TransformSop, &TransformParams {
        rotate: Vec3::new(0.0, 45.0, 0.0),
        ..Default::default()
    }).unwrap();
```

## SOP Quick Reference

### Generators (0 inputs — use `generate()`)

| SOP | Key Params | Output |
|-----|-----------|--------|
| `BoxSop` | `size: Vec3, center: Vec3` | 8 pts, 6 quads |
| `GridSop` | `rows, cols, size: [f32;2], orientation` | rows*cols pts, (r-1)*(c-1) quads |
| `SphereSop` | `radius: Vec3, rows, cols` | Parametric UV sphere |
| `TubeSop` | `radius, height, segments, caps` | Cylinder with optional caps |
| `TorusSop` | `major_radius, minor_radius, rows, cols` | Parametric torus |
| `CircleSop` | `radius, point_count` | Closed polygon |
| `LineSop` | `point_count, length` | Open polyline |

### Modifiers (1 input — use `.apply()`)

| SOP | Key Params | Effect |
|-----|-----------|--------|
| `TransformSop` | `translate, rotate, scale, pivot` | Move/rotate/scale all points |
| `SubdivideSop` | `depth, mode (Linear/CatmullClark)` | Refine mesh topology |
| `PolyExtrudeSop` | `distance, inset` | Extrude faces along normals |
| `SmoothSop` | `iterations, strength` | Laplacian smoothing |
| `ClipSop` | `origin, normal, keep_above` | Cut with plane |
| `NormalSop` | *(none)* | Compute vertex normals → "N" attrib |
| `ColorSop` | `color: [f32;3]` | Set uniform "Cd" attrib |
| `ScatterSop` | `count, seed` | Random points on surface |
| `BlastSop` | `group_name, entity, negate` | Delete by group |
| `GroupCreateSop` | `name, group_type, mode` | Create named group |
| `AttribCreateSop` | `name, class, attrib_type, value_*` | Create/set attribute |
| `EnumerateSop` | `name, class, start` | Sequential index attrib |
| `MeasureSop` | `measure_type (Area/Perimeter)` | Compute prim measurements |
| `SortSop` | `entity, mode, axis, reverse` | Reorder elements |
| `FuseSop` | *(tolerance)* | Merge coincident points |
| `ReverseSop` | *(none)* | Flip winding order |
| `VoronoiFractureSop` | `num_points, seed` | Fracture into pieces |

### Multi-input (use `.execute()`)

| SOP | Inputs | Effect |
|-----|--------|--------|
| `MergeSop` | Any number | Concatenate geometries |
| `CopyToPointsSop` | [source, target] | Instance source at each target point |

## Params Pattern

All params implement `Default` with Houdini-like defaults. Use struct update syntax:

```rust
// Override only what you need
let params = TransformParams {
    translate: Vec3::new(0.0, 5.0, 0.0),
    ..Default::default()  // rotate=0, scale=1, pivot=0
};
```

## Common Recipes

**Fractured debris:**
```rust
let geo = generate(&BoxSop, &BoxParams::default())?
    .apply(&VoronoiFractureSop, &VoronoiFractureParams {
        num_points: 20, seed: 42, create_inside_faces: true,
    })?
    .apply(&NormalSop, &NormalParams)?;
// "piece" prim attrib tracks fragments
```

**Terrain with scattered trees:**
```rust
let terrain = generate(&GridSop, &GridParams {
    rows: 50, cols: 50,
    size: [100.0, 100.0],
    ..Default::default()
})?;
let tree = generate(&BoxSop, &BoxParams {
    size: Vec3::new(0.2, 1.0, 0.2), ..Default::default()
})?;
let forest = CopyToPointsSop.execute(
    &[&tree, &terrain.clone().apply(&ScatterSop, &ScatterParams { count: 500, seed: 7 })?],
    &CopyToPointsParams,
)?;
let scene = MergeSop.execute(&[&terrain, &forest], &MergeParams)?;
```

**Smooth sculpted shape:**
```rust
let geo = generate(&SphereSop, &SphereParams {
    rows: 24, cols: 48, ..Default::default()
})?
.apply(&SubdivideSop, &SubdivideParams { depth: 1, mode: SubdivideMode::CatmullClark, ..Default::default() })?
.apply(&SmoothSop, &SmoothParams { iterations: 5, strength: 0.3 })?
.apply(&NormalSop, &NormalParams)?;
```

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Calling `.apply()` for a 0-input SOP | Use `generate(&sop, &params)` for generators |
| Calling `.apply()` for a 2-input SOP | Use `sop.execute(&[&geo1, &geo2], &params)` |
| Forgetting `.unwrap()` or `?` on Results | Every SOP returns `Result<Geometry, SopError>` |
| Indexing attributes by `PointHandle` directly | Use `handle.index()` to get `usize` |
| Creating attrib after points but not seeing values | Attribs auto-resize but new points get default value; set values explicitly |
| Mutating geometry inside a `.apply()` chain | SOPs are pure — do mutations before or after the chain |
| Not enabling feature gates | SOPs are gated: `features = ["creation", "transform", ...]` in Cargo.toml |
