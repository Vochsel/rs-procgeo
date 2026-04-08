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


def test_poly_extrude_matches_geometry_nodes_extrude_mesh():
    procgeo_snapshot = snapshot_procgeo_geometry(
        procgeo.poly_extrude(
            make_procgeo_quad(procgeo),
            distance=1.0,
            inset=0.0,
            output_front=True,
            output_side=True,
        )
    )

    blender_snapshot = evaluate_geometry_nodes_snapshot(
        _build_extrude_mesh,
        base_mesh=make_blender_quad_mesh(name="ConformanceExtrudeQuad"),
        object_name="ConformanceExtrudeCarrier",
        tree_name="ConformanceExtrudeTree",
    )

    assert_same_geometry(procgeo_snapshot, blender_snapshot)


def _build_extrude_mesh(node_group, input_node):
    extrude = node_group.nodes.new("GeometryNodeExtrudeMesh")
    extrude.mode = "FACES"
    node_group.links.new(input_node.outputs["Geometry"], extrude.inputs["Mesh"])
    extrude.inputs["Selection"].default_value = True
    # The shared quad uses winding [0, 1, 2, 3], whose face normal points along -Y.
    extrude.inputs["Offset"].default_value = (0.0, -1.0, 0.0)
    extrude.inputs["Offset Scale"].default_value = 1.0
    extrude.inputs["Individual"].default_value = False
    return extrude.outputs["Mesh"]
