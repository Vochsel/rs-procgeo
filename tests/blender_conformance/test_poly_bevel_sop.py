from __future__ import annotations

import pytest

procgeo = pytest.importorskip("procgeo")

from .helpers import (
    UNIT_QUAD_FACES,
    UNIT_QUAD_POINTS,
    assert_same_geometry,
    build_vertex_bevel_reference_snapshot,
    make_procgeo_quad,
    snapshot_procgeo_geometry,
)


def test_poly_bevel_matches_blender_vertex_bevel_cut_positions():
    procgeo_snapshot = snapshot_procgeo_geometry(
        procgeo.poly_bevel(make_procgeo_quad(procgeo), offset=0.1, divisions=1)
    )

    blender_snapshot = build_vertex_bevel_reference_snapshot(
        point_positions=UNIT_QUAD_POINTS,
        prim_point_indices=UNIT_QUAD_FACES,
        offset=0.1,
        segments=1,
    )

    assert_same_geometry(procgeo_snapshot, blender_snapshot)
