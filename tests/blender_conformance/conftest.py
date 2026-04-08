from __future__ import annotations

import pytest

bpy = pytest.importorskip("bpy")


@pytest.fixture(autouse=True)
def reset_blender_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    bpy.context.view_layer.update()

    yield

    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    bpy.context.view_layer.update()

    for mesh in list(bpy.data.meshes):
        if mesh.users == 0:
            bpy.data.meshes.remove(mesh)

    for node_group in list(bpy.data.node_groups):
        if node_group.users == 0:
            bpy.data.node_groups.remove(node_group)
