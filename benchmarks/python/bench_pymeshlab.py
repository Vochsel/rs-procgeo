"""Benchmarks for PyMeshLab."""

import sys
sys.path.insert(0, ".")
from bench_harness import bench, emit_result, grid_rc, SCALES

try:
    import pymeshlab
    import numpy as np
except ImportError:
    print('{"error": "pymeshlab not installed. Run: uv pip install pymeshlab numpy"}', file=sys.stderr)
    sys.exit(1)

FW = "pymeshlab"
LANG = "python"


def make_grid_ms(rc):
    """Create a MeshSet with a grid mesh."""
    x = np.linspace(0, 10, rc, dtype=np.float64)
    z = np.linspace(0, 10, rc, dtype=np.float64)
    xx, zz = np.meshgrid(x, z)
    vertices = np.column_stack([xx.ravel(), np.zeros(rc * rc, dtype=np.float64), zz.ravel()])

    faces = []
    for j in range(rc - 1):
        for i in range(rc - 1):
            idx = j * rc + i
            faces.append([idx, idx + 1, idx + rc])
            faces.append([idx + 1, idx + rc + 1, idx + rc])
    faces = np.array(faces, dtype=np.int32)

    ms = pymeshlab.MeshSet()
    m = pymeshlab.Mesh(vertices, faces)
    ms.add_mesh(m)
    return ms


def run():
    for scale in SCALES:
        rc = grid_rc(scale)

        # -- Creation: Grid --
        mean, std, iters = bench(lambda: make_grid_ms(rc))
        emit_result(FW, LANG, "creation", "grid", scale, mean, std, iters)

        # -- Creation: Sphere --
        def create_sphere():
            ms = pymeshlab.MeshSet()
            ms.create_sphere(radius=1.0)
            return ms

        mean, std, iters = bench(create_sphere)
        emit_result(FW, LANG, "creation", "sphere", scale, mean, std, iters)

        # -- Creation: Box --
        def create_box():
            ms = pymeshlab.MeshSet()
            ms.create_cube()
            return ms

        mean, std, iters = bench(create_box)
        emit_result(FW, LANG, "creation", "box", scale, mean, std, iters)

        # -- Transform --
        def transform_mesh():
            ms = make_grid_ms(rc)
            ms.apply_filter("compute_matrix_from_translation", traslmethod="XYZ translation",
                            axisx=10.0, axisy=0.0, axisz=0.0)
            ms.apply_filter("compute_matrix_from_scaling_or_normalization",
                            scalecenter="origin", axisx=2.0, axisy=2.0, axisz=2.0)
            return ms

        mean, std, iters = bench(transform_mesh)
        emit_result(FW, LANG, "transform", "translate_scale", scale, mean, std, iters)

        # -- Subdivide (small scales only) --
        if scale <= 10_000:
            def subdivide_mesh():
                ms = make_grid_ms(rc)
                ms.apply_filter("meshing_surface_subdivision_midpoint", iterations=1)
                return ms

            mean, std, iters = bench(subdivide_mesh)
            emit_result(FW, LANG, "transform", "subdivide", scale, mean, std, iters)

        # -- Smooth --
        def smooth_mesh():
            ms = make_grid_ms(rc)
            ms.apply_filter("apply_coord_laplacian_smoothing", stepsmoothnum=3)
            return ms

        mean, std, iters = bench(smooth_mesh)
        emit_result(FW, LANG, "transform", "smooth", scale, mean, std, iters)

        # -- Fuse (merge close vertices) --
        def fuse_mesh():
            ms = make_grid_ms(rc)
            ms.apply_filter("meshing_merge_close_vertices", threshold=pymeshlab.PercentageValue(0.01))
            return ms

        mean, std, iters = bench(fuse_mesh)
        emit_result(FW, LANG, "topology", "fuse", scale, mean, std, iters)

        # -- Scatter (Poisson disk sampling) --
        def scatter_mesh():
            ms = make_grid_ms(rc)
            ms.apply_filter("generate_sampling_poisson_disk", samplenum=scale)
            return ms

        mean, std, iters = bench(scatter_mesh)
        emit_result(FW, LANG, "topology", "scatter", scale, mean, std, iters)

        # -- Full Pipeline --
        def pipeline():
            ms = make_grid_ms(rc)
            ms.apply_filter("compute_matrix_from_translation", traslmethod="XYZ translation",
                            axisx=0.0, axisy=1.0, axisz=0.0)
            ms.apply_filter("compute_matrix_from_scaling_or_normalization",
                            scalecenter="origin", axisx=2.0, axisy=2.0, axisz=2.0)
            ms.apply_filter("apply_coord_laplacian_smoothing", stepsmoothnum=2)
            ms.apply_filter("meshing_merge_close_vertices", threshold=pymeshlab.PercentageValue(0.01))
            return ms

        mean, std, iters = bench(pipeline)
        emit_result(FW, LANG, "pipeline", "full_pipeline", scale, mean, std, iters)


if __name__ == "__main__":
    run()
