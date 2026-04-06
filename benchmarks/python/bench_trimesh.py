"""Benchmarks for trimesh (Python)."""

import sys
sys.path.insert(0, ".")
from bench_harness import bench, emit_result, grid_rc, SCALES

try:
    import trimesh
    import numpy as np
except ImportError:
    print('{"error": "trimesh not installed. Run: uv pip install trimesh numpy"}', file=sys.stderr)
    sys.exit(1)

FW = "trimesh"
LANG = "python"


def make_grid_mesh(rc):
    """Create a grid mesh with rc*rc vertices."""
    x = np.linspace(0, 10, rc)
    z = np.linspace(0, 10, rc)
    xx, zz = np.meshgrid(x, z)
    vertices = np.column_stack([xx.ravel(), np.zeros(rc * rc), zz.ravel()])

    faces = []
    for j in range(rc - 1):
        for i in range(rc - 1):
            idx = j * rc + i
            faces.append([idx, idx + 1, idx + rc])
            faces.append([idx + 1, idx + rc + 1, idx + rc])

    return trimesh.Trimesh(vertices=vertices, faces=np.array(faces))


def run():
    for scale in SCALES:
        rc = grid_rc(scale)

        # -- Creation: Grid --
        mean, std, iters = bench(lambda: make_grid_mesh(rc))
        emit_result(FW, LANG, "creation", "grid", scale, mean, std, iters)

        # -- Creation: Sphere (icosphere) --
        def create_sphere():
            subdiv = max(1, min(5, int(np.log2(scale / 12))))
            return trimesh.creation.icosphere(subdivisions=subdiv, radius=1.0)

        mean, std, iters = bench(create_sphere)
        emit_result(FW, LANG, "creation", "sphere", scale, mean, std, iters)

        # -- Creation: Box --
        mean, std, iters = bench(lambda: trimesh.creation.box(extents=[1.0, 1.0, 1.0]))
        emit_result(FW, LANG, "creation", "box", scale, mean, std, iters)

        # -- Transform --
        grid = make_grid_mesh(rc)
        transform_matrix = np.eye(4)
        transform_matrix[:3, 3] = [10.0, 0.0, 0.0]
        transform_matrix[:3, :3] *= 2.0

        def transform_mesh():
            m = grid.copy()
            m.apply_transform(transform_matrix)
            return m

        mean, std, iters = bench(transform_mesh)
        emit_result(FW, LANG, "transform", "translate_scale", scale, mean, std, iters)

        # -- Subdivide (small scales only) --
        if scale <= 10_000:
            def subdivide_mesh():
                m = grid.copy()
                v, f = trimesh.remesh.subdivide(m.vertices, m.faces)
                return trimesh.Trimesh(vertices=v, faces=f)

            mean, std, iters = bench(subdivide_mesh)
            emit_result(FW, LANG, "transform", "subdivide", scale, mean, std, iters)

        # -- Smooth --
        def smooth_mesh():
            m = grid.copy()
            trimesh.smoothing.filter_laplacian(m, iterations=3)
            return m

        mean, std, iters = bench(smooth_mesh)
        emit_result(FW, LANG, "transform", "smooth", scale, mean, std, iters)

        # -- Fuse (merge vertices) --
        def fuse_mesh():
            m = grid.copy()
            m.merge_vertices()
            return m

        mean, std, iters = bench(fuse_mesh)
        emit_result(FW, LANG, "topology", "fuse", scale, mean, std, iters)

        # -- Scatter (sample surface) --
        def scatter_mesh():
            return grid.sample(scale)

        mean, std, iters = bench(scatter_mesh)
        emit_result(FW, LANG, "topology", "scatter", scale, mean, std, iters)

        # -- Full Pipeline --
        def pipeline():
            m = make_grid_mesh(rc)
            m.apply_transform(transform_matrix)
            trimesh.smoothing.filter_laplacian(m, iterations=2)
            m.merge_vertices()
            return m

        mean, std, iters = bench(pipeline)
        emit_result(FW, LANG, "pipeline", "full_pipeline", scale, mean, std, iters)


if __name__ == "__main__":
    run()
