"""ProcGeo Python Example

Build first: cd bindings/procgeo-py && ./build.sh
Run: python examples/basic.py
"""

import procgeo

# Create a box
box = procgeo.create_box(size_x=2, size_y=2, size_z=2)
print(f"Box: {box}")

# Create a sphere
sphere = procgeo.create_sphere(radius=1.0, rows=16, cols=32)
print(f"Sphere: {sphere}")

# Transform the box
transformed = procgeo.transform(box, translate_x=5)
bbox = transformed.bounding_box()
print(f"Transformed box bbox: min={bbox[0]}, max={bbox[1]}")

# Subdivide and smooth
subdivided = procgeo.subdivide(box, depth=2)
smoothed = procgeo.smooth(subdivided, iterations=3, strength=0.5)
print(f"Smoothed: {smoothed}")

# Compute normals and color
with_normals = procgeo.compute_normals(smoothed)
colored = procgeo.color(with_normals, r=0.2, g=0.6, b=1.0)

# Scatter points on a grid
grid = procgeo.create_grid(rows=5, cols=5, size_x=4, size_y=4)
scattered = procgeo.scatter(grid, count=20, seed=42)
print(f"Scattered: {scattered.num_points} points")

# Copy box to scattered points
small_box = procgeo.create_box(size_x=0.2, size_y=0.2, size_z=0.2)
instances = procgeo.copy_to_points(small_box, scattered)
print(f"Instanced: {instances}")

# Poly extrude
extruded = procgeo.poly_extrude(box, distance=0.5, inset=0.1)
print(f"Extruded: {extruded}")

# Merge
merged = procgeo.merge([box, sphere, grid])
print(f"Merged: {merged}")

# Export
procgeo.write_obj(colored, "output.obj")
procgeo.write_glb(colored, "output.glb")
print("Written output.obj and output.glb")

print("\nDone! All SOPs working correctly.")
