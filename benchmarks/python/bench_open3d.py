"""Benchmarks for Open3D."""

import sys
sys.path.insert(0, ".")
from bench_harness import bench, emit_result, grid_rc, SCALES

try:
    import open3d as o3d
    import numpy as np
except ImportError:
    print('{"error": "open3d not installed. Run: uv pip install open3d numpy"}', file=sys.stderr)
    sys.exit(1)

FW = "open3d"
LANG = "python"


def make_grid_mesh(rc):
    """Create an Open3D TriangleMesh grid."""
    x = np.linspace(0, 10, rc, dtype=np.float64)
    z = np.linspace(0, 10, rc, dtype=np.float64)
    xx, zz = np.meshgrid(x, z)
    vertices = np.column_stack([xx.ravel(), np.zeros(rc * rc, dtype=np.float64), zz.ravel()])

    triangles = []
    for j in range(rc - 1):
        for i in range(rc - 1):
            idx = j * rc + i
            triangles.append([idx, idx + 1, idx + rc])
            triangles.append([idx + 1, idx + rc + 1, idx + rc])

    mesh = o3d.geometry.TriangleMesh()
    mesh.vertices = o3d.utility.Vector3dVector(vertices)
    mesh.triangles = o3d.utility.Vector3iVector(np.array(triangles, dtype=np.int32))
    return mesh


def run():
    for scale in SCALES:
        rc = grid_rc(scale)

        # -- Creation: Grid --
        mean, std, iters = bench(lambda: make_grid_mesh(rc))
        emit_result(FW, LANG, "creation", "grid", scale, mean, std, iters)

        # -- Creation: Sphere --
        def create_sphere():
            return o3d.geometry.TriangleMesh.create_sphere(radius=1.0, resolution=max(3, rc // 3))

        mean, std, iters = bench(create_sphere)
        emit_result(FW, LANG, "creation", "sphere", scale, mean, std, iters)

        # -- Creation: Box --
        mean, std, iters = bench(lambda: o3d.geometry.TriangleMesh.create_box(1.0, 1.0, 1.0))
        emit_result(FW, LANG, "creation", "box", scale, mean, std, iters)

        # -- Transform --
        grid = make_grid_mesh(rc)
        transform_matrix = np.eye(4)
        transform_matrix[:3, 3] = [10.0, 0.0, 0.0]
        transform_matrix[:3, :3] *= 2.0

        def transform_mesh():
            m = o3d.geometry.TriangleMesh(grid)
            m.transform(transform_matrix)
            return m

        mean, std, iters = bench(transform_mesh)
        emit_result(FW, LANG, "transform", "translate_scale", scale, mean, std, iters)

        # -- Subdivide (small scales only) --
        if scale <= 10_000:
            def subdivide_mesh():
                m = o3d.geometry.TriangleMesh(grid)
                return m.subdivide_midpoint(number_of_iterations=1)

            mean, std, iters = bench(subdivide_mesh)
            emit_result(FW, LANG, "transform", "subdivide", scale, mean, std, iters)

        # -- Smooth --
        def smooth_mesh():
            m = o3d.geometry.TriangleMesh(grid)
            return m.filter_smooth_simple(number_of_iterations=3)

        mean, std, iters = bench(smooth_mesh)
        emit_result(FW, LANG, "transform", "smooth", scale, mean, std, iters)

        # -- Fuse (merge close vertices not directly available, use cluster) --
        # Open3D doesn't have a direct merge_vertices, skip or approximate
        emit_result(FW, LANG, "topology", "fuse", scale, float("nan"), 0.0, 0)

        # -- Scatter (sample surface) --
        def scatter_mesh():
            return grid.sample_points_uniformly(number_of_points=scale)

        mean, std, iters = bench(scatter_mesh)
        emit_result(FW, LANG, "topology", "scatter", scale, mean, std, iters)

        # -- Full Pipeline --
        def pipeline():
            m = make_grid_mesh(rc)
            m.transform(transform_matrix)
            m = m.filter_smooth_simple(number_of_iterations=2)
            m.compute_vertex_normals()
            return m

        mean, std, iters = bench(pipeline)
        emit_result(FW, LANG, "pipeline", "full_pipeline", scale, mean, std, iters)


if __name__ == "__main__":
    run()
