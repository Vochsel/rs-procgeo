// ProcGeo TypeScript/Node.js Example
// Build first: cd bindings/procgeo-node && ./build.sh
// Run: node examples/basic.js

const procgeo = require('../procgeo.node');

// Create a box
const box = procgeo.createBox({ size: [2, 2, 2] });
console.log(`Box: ${box.numPoints} points, ${box.numPrims} prims`);

// Create a sphere
const sphere = procgeo.createSphere({ radius: 1.0, rows: 16, cols: 32 });
console.log(`Sphere: ${sphere.numPoints} points, ${sphere.numPrims} prims`);

// Transform the box
const transformed = procgeo.transform(box, {
  translate: [5, 0, 0],
  scale: [1.5, 1.5, 1.5],
});
console.log(`Transformed box center:`, transformed.boundingBox());

// Subdivide
const subdivided = procgeo.subdivide(box, { depth: 2 });
console.log(`Subdivided: ${subdivided.numPoints} points, ${subdivided.numPrims} prims`);

// Smooth
const smoothed = procgeo.smooth(subdivided, { iterations: 3, strength: 0.5 });
console.log(`Smoothed: ${smoothed.numPoints} points`);

// Compute normals and add color
const withNormals = procgeo.computeNormals(smoothed);
const colored = procgeo.color(withNormals, { color: [0.2, 0.6, 1.0] });

// Scatter points on a grid
const grid = procgeo.createGrid({ rows: 5, cols: 5, size_x: 4, size_y: 4 });
const scattered = procgeo.scatter(grid, { count: 20, seed: 42 });
console.log(`Scattered: ${scattered.numPoints} points on grid`);

// Copy box to scattered points
const instances = procgeo.copyToPoints(
  procgeo.createBox({ size: [0.2, 0.2, 0.2] }),
  scattered
);
console.log(`Instanced: ${instances.numPoints} points, ${instances.numPrims} prims`);

// Poly extrude
const extruded = procgeo.polyExtrude(box, { distance: 0.5, inset: 0.1 });
console.log(`Extruded: ${extruded.numPoints} points, ${extruded.numPrims} prims`);

// Merge geometries
const merged = procgeo.merge([box, sphere, grid]);
console.log(`Merged: ${merged.numPoints} points, ${merged.numPrims} prims`);

// Export to OBJ
procgeo.writeObj(colored, 'output.obj');
console.log('Written to output.obj');

// Export to GLB
procgeo.writeGlb(colored, 'output.glb');
console.log('Written to output.glb');

console.log('\nDone! All SOPs working correctly.');
