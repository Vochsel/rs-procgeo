// Procedural City Generator using ProcGeo
// Build first: cd bindings/procgeo-node && ./build.sh
// Run: node examples/procedural_city.js

const procgeo = require('../procgeo.node');

// Create a ground plane
const ground = procgeo.createGrid({ rows: 2, cols: 2, size_x: 20, size_y: 20 });

// Scatter building positions
const positions = procgeo.scatter(ground, { count: 15, seed: 123 });

// Create a building template (extruded box)
const building = procgeo.createBox({ size: [1, 3, 1] });
const extrudedBuilding = procgeo.polyExtrude(building, { distance: 0.2, inset: 0.15 });

// Copy buildings to scattered points
const city = procgeo.copyToPoints(extrudedBuilding, positions);

// Add normals and color
const withNormals = procgeo.computeNormals(city);
const colored = procgeo.color(withNormals, { color: [0.7, 0.7, 0.8] });

// Merge with ground
const groundColored = procgeo.color(ground, { color: [0.3, 0.5, 0.2] });
const scene = procgeo.merge([colored, groundColored]);

console.log(`City: ${scene.numPoints} points, ${scene.numPrims} prims`);

// Export
procgeo.writeGlb(scene, 'city.glb');
procgeo.writeObj(scene, 'city.obj');
console.log('Exported city.glb and city.obj');
