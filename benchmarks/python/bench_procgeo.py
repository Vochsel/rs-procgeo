"""Benchmarks for procgeo Python bindings (PyO3)."""

import sys
sys.path.insert(0, ".")
from bench_harness import bench, emit_result, grid_rc, SCALES

try:
    import procgeo as pg
except ImportError:
    print('{"error": "procgeo not installed. Build wheel and install it."}', file=sys.stderr)
    sys.exit(1)

FW = "procgeo"
LANG = "python"


def run():
    for scale in SCALES:
        rc = grid_rc(scale)

        # -- Creation: Grid --
        mean, std, iters = bench(lambda: pg.create_grid(rows=rc, cols=rc))
        emit_result(FW, LANG, "creation", "grid", scale, mean, std, iters)

        # -- Creation: Sphere --
        sr = max(3, int(rc * 0.7))
        sc = max(4, int(rc * 1.4))
        mean, std, iters = bench(lambda sr=sr, sc=sc: pg.create_sphere(rows=sr, cols=sc))
        emit_result(FW, LANG, "creation", "sphere", scale, mean, std, iters)

        # -- Creation: Box --
        mean, std, iters = bench(lambda: pg.create_box())
        emit_result(FW, LANG, "creation", "box", scale, mean, std, iters)

        # -- Transform --
        grid = pg.create_grid(rows=rc, cols=rc)
        mean, std, iters = bench(
            lambda: pg.transform(grid, translate_x=10.0, scale_x=2.0, scale_y=2.0, scale_z=2.0)
        )
        emit_result(FW, LANG, "transform", "translate_scale", scale, mean, std, iters)

        # -- Subdivide (small scales only) --
        if scale <= 10_000:
            mean, std, iters = bench(lambda: pg.subdivide(grid, depth=1))
            emit_result(FW, LANG, "transform", "subdivide", scale, mean, std, iters)

        # -- Smooth --
        mean, std, iters = bench(lambda: pg.smooth(grid, iterations=3, strength=0.5))
        emit_result(FW, LANG, "transform", "smooth", scale, mean, std, iters)

        # -- Fuse --
        mean, std, iters = bench(lambda: pg.fuse(grid, distance=0.001))
        emit_result(FW, LANG, "topology", "fuse", scale, mean, std, iters)

        # -- Scatter --
        mean, std, iters = bench(lambda: pg.scatter(grid, count=scale, seed=42))
        emit_result(FW, LANG, "topology", "scatter", scale, mean, std, iters)

        # -- Softbody (XPBD), 10 simulated frames --
        mean, std, iters = bench(lambda: pg.softbody(grid, frame=10))
        emit_result(FW, LANG, "simulation", "softbody", scale, mean, std, iters)

        # -- Full Pipeline --
        def pipeline():
            g = pg.create_grid(rows=rc, cols=rc)
            g = pg.transform(g, translate_y=1.0, scale_x=2.0, scale_y=2.0, scale_z=2.0)
            g = pg.smooth(g, iterations=2, strength=0.5)
            g = pg.fuse(g, distance=0.001)
            return g

        mean, std, iters = bench(pipeline)
        emit_result(FW, LANG, "pipeline", "full_pipeline", scale, mean, std, iters)


if __name__ == "__main__":
    run()
