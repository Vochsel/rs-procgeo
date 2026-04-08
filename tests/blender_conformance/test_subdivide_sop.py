from __future__ import annotations

import pytest

procgeo = pytest.importorskip("procgeo")

from .helpers import (
    assert_same_geometry,
    evaluate_geometry_nodes_snapshot,
    make_blender_quad_mesh,
    make_procgeo_quad,
    snapshot_procgeo_geometry,
)


def test_subdivide_matches_geometry_nodes_subdivide_mesh():
    procgeo_snapshot = snapshot_procgeo_geometry(
        procgeo.subdivide(make_procgeo_quad(procgeo), depth=1)
    )

    blender_snapshot = evaluate_geometry_nodes_snapshot(
        _build_subdivide_mesh,
        base_mesh=make_blender_quad_mesh(name="ConformanceSubdivideQuad"),
        object_name="ConformanceSubdivideCarrier",
        tree_name="ConformanceSubdivideTree",
    )

    assert_same_geometry(procgeo_snapshot, blender_snapshot)


def _build_subdivide_mesh(node_group, input_node):
    subdivide = node_group.nodes.new("GeometryNodeSubdivideMesh")
    node_group.links.new(input_node.outputs["Geometry"], subdivide.inputs["Mesh"])
    subdivide.inputs["Level"].default_value = 1
    return subdivide.outputs["Mesh"]
