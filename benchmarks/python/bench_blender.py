"""Benchmarks for Blender bpy."""

import sys
sys.path.insert(0, ".")
from bench_harness import bench, emit_result, grid_rc, SCALES

try:
    import bpy
    import bmesh
except ImportError:
    print('{"error": "bpy not installed. Run: uv pip install bpy"}', file=sys.stderr)
    sys.exit(1)

FW = "blender_bpy"
LANG = "python"


def clear_scene():
    """Remove all mesh objects from the scene."""
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)


def run():
    for scale in SCALES:
        rc = grid_rc(scale)

        # -- Creation: Grid --
        def create_grid():
            clear_scene()
            bpy.ops.mesh.primitive_grid_add(x_subdivisions=rc, y_subdivisions=rc, size=10.0)

        mean, std, iters = bench(create_grid)
        emit_result(FW, LANG, "creation", "grid", scale, mean, std, iters)

        # -- Creation: Sphere --
        seg = max(4, int((scale ** 0.5) * 1.4))
        rings = max(3, int((scale ** 0.5) * 0.7))

        def create_sphere():
            clear_scene()
            bpy.ops.mesh.primitive_uv_sphere_add(segments=seg, ring_count=rings, radius=1.0)

        mean, std, iters = bench(create_sphere)
        emit_result(FW, LANG, "creation", "sphere", scale, mean, std, iters)

        # -- Creation: Box --
        def create_box():
            clear_scene()
            bpy.ops.mesh.primitive_cube_add(size=1.0)

        mean, std, iters = bench(create_box)
        emit_result(FW, LANG, "creation", "box", scale, mean, std, iters)

        # -- Transform --
        clear_scene()
        bpy.ops.mesh.primitive_grid_add(x_subdivisions=rc, y_subdivisions=rc, size=10.0)
        obj = bpy.context.active_object

        def transform_mesh():
            obj.location = (10.0, 0.0, 0.0)
            obj.scale = (2.0, 2.0, 2.0)
            bpy.context.view_layer.update()

        mean, std, iters = bench(transform_mesh)
        emit_result(FW, LANG, "transform", "translate_scale", scale, mean, std, iters)

        # -- Subdivide (small scales only) --
        if scale <= 10_000:
            def subdivide_mesh():
                clear_scene()
                bpy.ops.mesh.primitive_grid_add(x_subdivisions=rc, y_subdivisions=rc, size=10.0)
                obj = bpy.context.active_object
                bpy.context.view_layer.objects.active = obj
                bpy.ops.object.mode_set(mode="EDIT")
                bpy.ops.mesh.subdivide(number_cuts=1)
                bpy.ops.object.mode_set(mode="OBJECT")

            mean, std, iters = bench(subdivide_mesh)
            emit_result(FW, LANG, "transform", "subdivide", scale, mean, std, iters)

        # -- Smooth --
        def smooth_mesh():
            clear_scene()
            bpy.ops.mesh.primitive_grid_add(x_subdivisions=rc, y_subdivisions=rc, size=10.0)
            obj = bpy.context.active_object
            bpy.context.view_layer.objects.active = obj
            bpy.ops.object.mode_set(mode="EDIT")
            bpy.ops.mesh.vertices_smooth(factor=0.5, repeat=3)
            bpy.ops.object.mode_set(mode="OBJECT")

        mean, std, iters = bench(smooth_mesh)
        emit_result(FW, LANG, "transform", "smooth", scale, mean, std, iters)

        # -- Fuse (remove doubles) --
        def fuse_mesh():
            clear_scene()
            bpy.ops.mesh.primitive_grid_add(x_subdivisions=rc, y_subdivisions=rc, size=10.0)
            obj = bpy.context.active_object
            bpy.context.view_layer.objects.active = obj
            bpy.ops.object.mode_set(mode="EDIT")
            bpy.ops.mesh.select_all(action="SELECT")
            bpy.ops.mesh.remove_doubles(threshold=0.001)
            bpy.ops.object.mode_set(mode="OBJECT")

        mean, std, iters = bench(fuse_mesh)
        emit_result(FW, LANG, "topology", "fuse", scale, mean, std, iters)

        # -- Scatter (sample surface) --
        def scatter_mesh():
            clear_scene()
            bpy.ops.mesh.primitive_grid_add(x_subdivisions=rc, y_subdivisions=rc, size=10.0)
            obj = bpy.context.active_object
            # Use particle system for surface sampling
            bpy.ops.object.particle_system_add()
            ps = obj.particle_systems[0]
            ps.settings.count = scale
            ps.settings.emit_from = "FACE"
            bpy.context.view_layer.update()

        mean, std, iters = bench(scatter_mesh)
        emit_result(FW, LANG, "topology", "scatter", scale, mean, std, iters)

        # -- Full Pipeline --
        def pipeline():
            clear_scene()
            bpy.ops.mesh.primitive_grid_add(x_subdivisions=rc, y_subdivisions=rc, size=10.0)
            obj = bpy.context.active_object
            obj.location = (0.0, 1.0, 0.0)
            obj.scale = (2.0, 2.0, 2.0)
            bpy.context.view_layer.objects.active = obj
            bpy.ops.object.mode_set(mode="EDIT")
            bpy.ops.mesh.vertices_smooth(factor=0.5, repeat=2)
            bpy.ops.mesh.select_all(action="SELECT")
            bpy.ops.mesh.remove_doubles(threshold=0.001)
            bpy.ops.object.mode_set(mode="OBJECT")

        mean, std, iters = bench(pipeline)
        emit_result(FW, LANG, "pipeline", "full_pipeline", scale, mean, std, iters)


if __name__ == "__main__":
    run()
