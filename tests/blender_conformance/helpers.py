from __future__ import annotations

from dataclasses import dataclass
from typing import Callable, Iterable, Sequence


EPSILON_DIGITS = 6
UNIT_QUAD_POINTS = (
    (-0.5, 0.0, -0.5),
    (0.5, 0.0, -0.5),
    (0.5, 0.0, 0.5),
    (-0.5, 0.0, 0.5),
)
UNIT_QUAD_FACES = ((0, 1, 2, 3),)


@dataclass(frozen=True)
class GeometrySnapshot:
    point_positions: tuple[tuple[float, float, float], ...]
    prim_point_indices: tuple[tuple[int, ...], ...]
    vertex_points: tuple[int, ...]


@dataclass(frozen=True)
class CanonicalGeometry:
    point_positions: tuple[tuple[float, float, float], ...]
    prim_point_indices: tuple[tuple[int, ...], ...]
    vertex_points: tuple[int, ...]


def rounded_vec3(values: Iterable[float]) -> tuple[float, float, float]:
    x, y, z = values
    return (
        round(float(x), EPSILON_DIGITS),
        round(float(y), EPSILON_DIGITS),
        round(float(z), EPSILON_DIGITS),
    )


def make_procgeo_quad(procgeo_module):
    geo = procgeo_module.Geometry()
    for position in UNIT_QUAD_POINTS:
        geo.add_point(*position)
    geo.add_face(list(UNIT_QUAD_FACES[0]))
    return geo


def make_blender_mesh(
    point_positions: Sequence[Sequence[float]],
    prim_point_indices: Sequence[Sequence[int]],
    *,
    name: str,
):
    import bpy

    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata([tuple(position) for position in point_positions], [], [tuple(prim) for prim in prim_point_indices])
    mesh.update()
    return mesh


def make_blender_quad_mesh(*, name: str):
    return make_blender_mesh(UNIT_QUAD_POINTS, UNIT_QUAD_FACES, name=name)


def geometry_snapshot(
    point_positions: Sequence[Sequence[float]],
    prim_point_indices: Sequence[Sequence[int]],
) -> GeometrySnapshot:
    rounded_positions = tuple(rounded_vec3(position) for position in point_positions)
    normalized_prims = tuple(tuple(int(point_index) for point_index in prim) for prim in prim_point_indices)
    return GeometrySnapshot(
        point_positions=rounded_positions,
        prim_point_indices=normalized_prims,
        vertex_points=tuple(point_index for prim in normalized_prims for point_index in prim),
    )


def snapshot_procgeo_geometry(geo) -> GeometrySnapshot:
    return GeometrySnapshot(
        point_positions=tuple(rounded_vec3(geo.point_pos(i)) for i in range(geo.num_points)),
        prim_point_indices=tuple(
            tuple(int(index) for index in geo.prim_point_indices(prim_index))
            for prim_index in range(geo.num_prims)
        ),
        vertex_points=tuple(int(geo.vertex_point(vertex_index)) for vertex_index in range(geo.num_vertices)),
    )


def snapshot_blender_mesh(mesh) -> GeometrySnapshot:
    return GeometrySnapshot(
        point_positions=tuple(rounded_vec3(vertex.co[:]) for vertex in mesh.vertices),
        prim_point_indices=tuple(
            tuple(int(index) for index in polygon.vertices)
            for polygon in mesh.polygons
        ),
        vertex_points=tuple(int(loop.vertex_index) for loop in mesh.loops),
    )


def evaluate_geometry_nodes_snapshot(
    build_nodes: Callable,
    *,
    base_mesh=None,
    object_name: str = "ConformanceCarrier",
    tree_name: str = "ConformanceTree",
) -> GeometrySnapshot:
    import bpy

    mesh = base_mesh or bpy.data.meshes.new(f"{object_name}Mesh")
    obj = bpy.data.objects.new(object_name, mesh)
    bpy.context.scene.collection.objects.link(obj)

    modifier = obj.modifiers.new(name="ProcGeoConformance", type="NODES")
    node_group = bpy.data.node_groups.new(tree_name, "GeometryNodeTree")
    modifier.node_group = node_group
    node_group.interface.new_socket(
        name="Geometry",
        in_out="INPUT",
        socket_type="NodeSocketGeometry",
    )
    node_group.interface.new_socket(
        name="Geometry",
        in_out="OUTPUT",
        socket_type="NodeSocketGeometry",
    )

    nodes = node_group.nodes
    links = node_group.links
    nodes.clear()

    input_node = nodes.new("NodeGroupInput")
    output_node = nodes.new("NodeGroupOutput")
    output_node.is_active_output = True

    geometry_socket = build_nodes(node_group, input_node)
    links.new(geometry_socket, output_node.inputs["Geometry"])

    bpy.context.view_layer.update()
    evaluated = obj.evaluated_get(bpy.context.evaluated_depsgraph_get())
    evaluated_mesh = bpy.data.meshes.new_from_object(evaluated)
    try:
        return snapshot_blender_mesh(evaluated_mesh)
    finally:
        bpy.data.meshes.remove(evaluated_mesh)


def build_vertex_bevel_reference_snapshot(
    *,
    point_positions: Sequence[Sequence[float]],
    prim_point_indices: Sequence[Sequence[int]],
    offset: float,
    segments: int = 1,
) -> GeometrySnapshot:
    import bmesh
    import bpy
    from mathutils import Vector

    if len(prim_point_indices) != 1:
        raise AssertionError("Vertex bevel reference currently expects a single polygon input.")
    if segments != 1:
        raise AssertionError("Vertex bevel reference currently supports a single segment.")

    source_mesh = make_blender_mesh(point_positions, prim_point_indices, name="ConformanceBevelSource")
    bm = bmesh.new()
    bm.from_mesh(source_mesh)
    bmesh.ops.bevel(
        bm,
        geom=list(bm.verts),
        offset=offset,
        offset_type="OFFSET",
        segments=segments,
        affect="VERTICES",
        profile=0.5,
    )

    beveled_mesh = bpy.data.meshes.new("ConformanceBevelEval")
    bm.to_mesh(beveled_mesh)
    beveled_mesh.update()
    bm.free()

    try:
        if len(beveled_mesh.polygons) != 1:
            raise AssertionError("Expected Blender vertex bevel to produce a single inset polygon.")

        cut_boundary = tuple(int(vertex_index) for vertex_index in beveled_mesh.polygons[0].vertices)
        cut_positions = tuple(
            rounded_vec3(beveled_mesh.vertices[vertex_index].co[:])
            for vertex_index in cut_boundary
        )
        original_positions = tuple(rounded_vec3(position) for position in point_positions)
        original_vectors = [Vector(position) for position in original_positions]
        face_normal = (original_vectors[1] - original_vectors[0]).cross(original_vectors[2] - original_vectors[0])

        corner_to_cut_indices: dict[int, list[int]] = {corner_index: [] for corner_index in range(len(original_positions))}
        for cut_index, cut_position in enumerate(cut_positions):
            cut_vector = Vector(cut_position)
            nearest_corner = min(
                range(len(original_positions)),
                key=lambda corner_index: (cut_vector - original_vectors[corner_index]).length,
            )
            corner_to_cut_indices[nearest_corner].append(cut_index)

        prims: list[tuple[int, ...]] = [tuple(range(len(cut_positions)))]
        for corner_index in range(len(original_positions)):
            cut_indices = corner_to_cut_indices[corner_index]
            if len(cut_indices) != 2:
                raise AssertionError("Expected exactly two bevel cut points per original corner.")

            # ProcGeo's current bevel keeps the original corner vertices as triangles.
            # Blender's vertex bevel gives us the cut positions, then we rebuild that
            # ProcGeo-specific face layout on top of Blender's bevel distances.
            triangle = [cut_indices[0], len(cut_positions) + corner_index, cut_indices[1]]
            cut_a = Vector(cut_positions[triangle[0]])
            original_corner = Vector(original_positions[corner_index])
            cut_b = Vector(cut_positions[triangle[2]])
            triangle_normal = (original_corner - cut_a).cross(cut_b - cut_a)
            if triangle_normal.dot(face_normal) < 0:
                triangle = [triangle[2], triangle[1], triangle[0]]
            prims.append(tuple(triangle))

        return geometry_snapshot(cut_positions + original_positions, prims)
    finally:
        bpy.data.meshes.remove(beveled_mesh)
        bpy.data.meshes.remove(source_mesh)


def canonicalize_geometry(snapshot: GeometrySnapshot) -> CanonicalGeometry:
    # Blender and ProcGeo do not guarantee identical point/primitive numbering,
    # so comparisons use a deterministic renumbering while preserving winding.
    point_order = sorted(
        range(len(snapshot.point_positions)),
        key=lambda point_index: (*snapshot.point_positions[point_index], point_index),
    )
    point_remap = {old_index: new_index for new_index, old_index in enumerate(point_order)}

    canonical_points = tuple(snapshot.point_positions[point_index] for point_index in point_order)
    canonical_prims = tuple(
        sorted(
            normalize_cycle(tuple(point_remap[point_index] for point_index in prim))
            for prim in snapshot.prim_point_indices
        )
    )
    canonical_vertex_points = tuple(
        point_index
        for prim in canonical_prims
        for point_index in prim
    )

    return CanonicalGeometry(
        point_positions=canonical_points,
        prim_point_indices=canonical_prims,
        vertex_points=canonical_vertex_points,
    )


def normalize_cycle(indices: tuple[int, ...]) -> tuple[int, ...]:
    if not indices:
        return indices

    rotations = (
        indices[offset:] + indices[:offset]
        for offset in range(len(indices))
    )
    return min(rotations)


def assert_same_geometry(procgeo_snapshot: GeometrySnapshot, blender_snapshot: GeometrySnapshot) -> None:
    assert len(procgeo_snapshot.point_positions) == len(blender_snapshot.point_positions)
    assert len(procgeo_snapshot.prim_point_indices) == len(blender_snapshot.prim_point_indices)
    assert len(procgeo_snapshot.vertex_points) == len(blender_snapshot.vertex_points)

    procgeo_canonical = canonicalize_geometry(procgeo_snapshot)
    blender_canonical = canonicalize_geometry(blender_snapshot)

    assert procgeo_canonical.point_positions == blender_canonical.point_positions
    assert procgeo_canonical.prim_point_indices == blender_canonical.prim_point_indices
    assert procgeo_canonical.vertex_points == blender_canonical.vertex_points
