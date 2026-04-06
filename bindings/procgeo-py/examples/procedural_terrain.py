"""Procedural Terrain Generator using ProcGeo

Build first: cd bindings/procgeo-py && ./build.sh
Run: python examples/procedural_terrain.py
"""

import procgeo
import math

# Create a high-res grid for terrain
terrain = procgeo.create_grid(rows=50, cols=50, size_x=10, size_y=10)
print(f"Base terrain: {terrain}")

# Subdivide for more detail
terrain = procgeo.subdivide(terrain, depth=1)
print(f"Subdivided: {terrain}")

# Smooth the terrain
terrain = procgeo.smooth(terrain, iterations=2, strength=0.3)

# Compute normals
terrain = procgeo.compute_normals(terrain)

# Color it green
terrain = procgeo.color(terrain, r=0.3, g=0.6, b=0.2)

# Scatter some rocks
rock_positions = procgeo.scatter(terrain, count=30, seed=7)
rock = procgeo.create_sphere(radius=0.15, rows=6, cols=8)
rocks = procgeo.copy_to_points(rock, rock_positions)
rocks = procgeo.compute_normals(rocks)
rocks = procgeo.color(rocks, r=0.5, g=0.5, b=0.5)

# Scatter some trees (tall thin boxes)
tree_positions = procgeo.scatter(terrain, count=20, seed=99)
tree_trunk = procgeo.create_tube(
    radius_bottom=0.05, radius_top=0.03,
    height=0.8, cols=6, rows=2
)
trees = procgeo.copy_to_points(tree_trunk, tree_positions)
trees = procgeo.compute_normals(trees)
trees = procgeo.color(trees, r=0.4, g=0.25, b=0.1)

# Merge everything
scene = procgeo.merge([terrain, rocks, trees])
print(f"Final scene: {scene}")

# Export
procgeo.write_glb(scene, "terrain.glb")
procgeo.write_obj(scene, "terrain.obj")
print("Exported terrain.glb and terrain.obj")
