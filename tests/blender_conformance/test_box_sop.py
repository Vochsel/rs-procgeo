from __future__ import annotations

import pytest

procgeo = pytest.importorskip("procgeo")

from .helpers import assert_same_geometry, evaluate_geometry_nodes_snapshot, snapshot_procgeo_geometry


def test_box_sop_matches_geometry_nodes_mesh_cube():
    procgeo_snapshot = snapshot_procgeo_geometry(procgeo.create_box())

    blender_snapshot = evaluate_geometry_nodes_snapshot(
        lambda node_group, _input_node: node_group.nodes.new("GeometryNodeMeshCube").outputs["Mesh"]
    )

    assert_same_geometry(procgeo_snapshot, blender_snapshot)
