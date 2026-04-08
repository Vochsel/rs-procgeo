"""Export reference geometry from Blender primitives as JSON.

Run via: blender --background --python blender_export.py -- [--output DIR] [--filter PATTERN]

Each test creates geometry using Blender's mesh primitives (the same underlying
operations as Geometry Nodes), extracts mesh data, and writes a JSON file.
Coordinates are converted from Blender Z-up to procgeo Y-up (Houdini convention)
by swapping the Y and Z components.
"""

import bpy
import json
import math
import os
import sys


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def clear_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()


def apply_transforms(obj):
    """Apply all object-level transforms so mesh data reflects world coords."""
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)


def extract_mesh(obj):
    """Extract geometry data from a Blender mesh object.

    Coordinates are converted from Z-up (Blender) to Y-up (procgeo):
        Blender (X, Y, Z) -> procgeo (X, Z, Y)
    """
    depsgraph = bpy.context.evaluated_depsgraph_get()
    obj_eval = obj.evaluated_get(depsgraph)
    mesh = obj_eval.data

    # Positions — swap Y <-> Z
    positions = []
    for v in mesh.vertices:
        positions.append([
            round(float(v.co.x), 6),
            round(float(v.co.z), 6),  # Blender Z -> procgeo Y
            round(float(v.co.y), 6),  # Blender Y -> procgeo Z
        ])

    # Faces (polygon -> list of vertex indices)
    faces = [list(p.vertices) for p in mesh.polygons]

    # Face vertex counts (for topology comparison)
    face_vertex_counts = sorted(len(f) for f in faces)

    # Vertex normals — swap Y <-> Z
    normals = []
    for v in mesh.vertices:
        normals.append([
            round(float(v.normal.x), 6),
            round(float(v.normal.z), 6),
            round(float(v.normal.y), 6),
        ])

    # Bounding box — swap Y <-> Z
    if positions:
        xs = [p[0] for p in positions]
        ys = [p[1] for p in positions]
        zs = [p[2] for p in positions]
        bbox_min = [min(xs), min(ys), min(zs)]
        bbox_max = [max(xs), max(ys), max(zs)]
    else:
        bbox_min = [0, 0, 0]
        bbox_max = [0, 0, 0]

    return {
        "num_points": len(mesh.vertices),
        "num_faces": len(mesh.polygons),
        "positions": positions,
        "faces": faces,
        "face_vertex_counts": face_vertex_counts,
        "normals": normals,
        "bbox_min": bbox_min,
        "bbox_max": bbox_max,
    }


# ---------------------------------------------------------------------------
# Test definitions — Creation SOPs
# ---------------------------------------------------------------------------

def test_box_default():
    """Default unit box: size 1x1x1, centered at origin."""
    clear_scene()
    # Blender size=1 -> cube from -0.5 to 0.5 (matches procgeo default)
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=(0, 0, 0))
    return extract_mesh(bpy.context.active_object)


def test_box_scaled():
    """Non-uniform box: procgeo size=(2, 3, 4)."""
    clear_scene()
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=(0, 0, 0))
    obj = bpy.context.active_object
    # procgeo size=(sx, sy, sz) -> Blender scale=(sx, sz, sy) due to Y<->Z swap
    obj.scale = (2.0, 4.0, 3.0)
    apply_transforms(obj)
    return extract_mesh(obj)


def test_box_offset():
    """Box offset from origin: procgeo center=(1, 2, 3)."""
    clear_scene()
    # procgeo center=(cx, cy, cz) -> Blender location=(cx, cz, cy)
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=(1.0, 3.0, 2.0))
    obj = bpy.context.active_object
    apply_transforms(obj)
    return extract_mesh(obj)


def test_grid_default():
    """Default 10x10 grid, size 10x10.

    Blender grid lies on XY plane (Z-up); after Y<->Z swap it maps to
    procgeo's XZ plane (Y-up, default GridOrientation::XZ).

    Parameter mapping:
        procgeo rows=10, cols=10  ->  Blender x_subdivisions=10, y_subdivisions=10
        procgeo size=[10, 10]     ->  Blender size=10 (total extent)
    """
    clear_scene()
    bpy.ops.mesh.primitive_grid_add(
        x_subdivisions=10, y_subdivisions=10, size=5.0,
    )
    return extract_mesh(bpy.context.active_object)


def test_sphere_default():
    """Default UV sphere: radius=0.5, rows=12, cols=24.

    Parameter mapping:
        procgeo cols=24   ->  Blender segments=24
        procgeo rows=12   ->  Blender ring_count=11  (ring_count = rows - 1)

    procgeo points: 2 + (rows-1)*cols = 2 + 11*24 = 266
    Blender points: 2 + ring_count*segments = 2 + 11*24 = 266
    """
    clear_scene()
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=24, ring_count=11, radius=0.5,
    )
    return extract_mesh(bpy.context.active_object)


def test_circle_default():
    """Default circle: radius=1.0, 40 divisions.

    Blender circle with fill_type='NGON' creates a single N-gon face.
    procgeo circle creates a closed polygon.
    """
    clear_scene()
    bpy.ops.mesh.primitive_circle_add(
        vertices=40, radius=1.0, fill_type="NGON",
    )
    return extract_mesh(bpy.context.active_object)


def test_tube_default():
    """Default tube/cylinder: radius=0.5, height=1.0, 24 cols, no caps.

    procgeo TubeParams default: radius_bottom=0.5, radius_top=0.5,
    height=1.0, cols=24, rows=2, cap=None

    Blender cylinder with end_fill_type='NOTHING' matches no-cap tube.
    """
    clear_scene()
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=24, radius=0.5, depth=1.0,
        end_fill_type="NOTHING",
    )
    return extract_mesh(bpy.context.active_object)


def test_torus_default():
    """Default torus: outer_r=1.0, inner_r=0.3, 24 major segs, 12 minor segs.

    Parameter mapping:
        procgeo radius_outer=1.0  ->  Blender major_radius=1.0
        procgeo radius_inner=0.3  ->  Blender minor_radius=0.3
        procgeo cols=24           ->  Blender major_segments=24
        procgeo rows=12           ->  Blender minor_segments=12
    """
    clear_scene()
    bpy.ops.mesh.primitive_torus_add(
        major_segments=24, minor_segments=12,
        major_radius=1.0, minor_radius=0.3,
    )
    return extract_mesh(bpy.context.active_object)


# ---------------------------------------------------------------------------
# Test definitions — Transform SOP
# ---------------------------------------------------------------------------

def test_transform_translate():
    """Box translated by (1, 2, 3) in procgeo coords."""
    clear_scene()
    # procgeo translate=(1,2,3) -> Blender location=(1,3,2)
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=(1.0, 3.0, 2.0))
    obj = bpy.context.active_object
    apply_transforms(obj)
    return extract_mesh(obj)


def test_transform_uniform_scale():
    """Box with uniform scale (2, 2, 2)."""
    clear_scene()
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=(0, 0, 0))
    obj = bpy.context.active_object
    obj.scale = (2.0, 2.0, 2.0)
    apply_transforms(obj)
    return extract_mesh(obj)


def test_transform_rotate_z():
    """Box rotated 45 degrees around Y axis (procgeo Y = Blender Z)."""
    clear_scene()
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=(0, 0, 0))
    obj = bpy.context.active_object
    # procgeo rotate=(0, 45, 0) means 45° around Y-up axis
    # Blender equivalent: 45° around Z axis (since Blender Z = procgeo Y)
    obj.rotation_euler = (0, 0, math.radians(45))
    apply_transforms(obj)
    return extract_mesh(obj)


# ---------------------------------------------------------------------------
# Test definitions — Normals
# ---------------------------------------------------------------------------

def test_normals_box():
    """Box with vertex normals computed by Blender."""
    clear_scene()
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=(0, 0, 0))
    obj = bpy.context.active_object
    # Blender auto-computes vertex normals
    return extract_mesh(obj)


# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------

TESTS = {
    # Creation SOPs
    "box_default": test_box_default,
    "box_scaled": test_box_scaled,
    "box_offset": test_box_offset,
    "grid_default": test_grid_default,
    "sphere_default": test_sphere_default,
    "circle_default": test_circle_default,
    "tube_default": test_tube_default,
    "torus_default": test_torus_default,
    # Transform SOP
    "transform_translate": test_transform_translate,
    "transform_uniform_scale": test_transform_uniform_scale,
    "transform_rotate_z": test_transform_rotate_z,
    # Normals
    "normals_box": test_normals_box,
}


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    # Parse args after '--' separator
    argv = sys.argv
    try:
        idx = argv.index("--")
        args = argv[idx + 1:]
    except ValueError:
        args = []

    output_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "reference")
    filter_pat = None

    i = 0
    while i < len(args):
        if args[i] == "--output" and i + 1 < len(args):
            output_dir = args[i + 1]
            i += 2
        elif args[i] == "--filter" and i + 1 < len(args):
            filter_pat = args[i + 1]
            i += 2
        else:
            i += 1

    os.makedirs(output_dir, exist_ok=True)

    exported = 0
    for name in sorted(TESTS):
        if filter_pat and filter_pat not in name:
            continue

        fn = TESTS[name]
        data = fn()
        data["name"] = name
        data["description"] = (fn.__doc__ or "").strip().split("\n")[0]
        data["coordinate_system"] = "y_up"

        path = os.path.join(output_dir, f"{name}.json")
        with open(path, "w") as f:
            json.dump(data, f, indent=2)

        print(f"  exported {name:30s}  ({data['num_points']:>4d} pts, {data['num_faces']:>4d} faces)")
        exported += 1

    print(f"\n  {exported} reference file(s) written to {output_dir}/")


if __name__ == "__main__":
    main()
