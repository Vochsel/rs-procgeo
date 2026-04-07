export const examples = {
    basic: `// Basic box with normals
const box = pg.createBox({ size: [1, 1, 1] });
const withNormals = pg.computeNormals(box);
return withNormals;
`,

    subdiv: `// Catmull-Clark subdivided sphere
let geo = pg.createSphere({ radius: 0.8, rows: 4, cols: 8 });
geo = pg.subdivide(geo, { depth: 2, mode: 'catmullClark' });
geo = pg.computeNormals(geo);
return geo;
`,

    scatter: `// Scatter boxes on a grid
const grid = pg.createGrid({ rows: 5, cols: 5, sizeX: 4, sizeY: 4 });
const points = pg.scatter(grid, { count: 30, seed: 42 });
const box = pg.createBox({ size: [0.15, 0.15, 0.15] });
let instances = pg.copyToPoints(box, points);
instances = pg.computeNormals(instances);
instances = pg.color(instances, { color: [0.9, 0.5, 0.2] });
return instances;
`,

    extrude: `// Extruded city blocks
const grid = pg.createGrid({ rows: 2, cols: 2, sizeX: 8, sizeY: 8 });
const points = pg.scatter(grid, { count: 12, seed: 7 });
let building = pg.createBox({ size: [0.8, 2, 0.8] });
building = pg.polyExtrude(building, { distance: 0.3, inset: 0.15 });
let city = pg.copyToPoints(building, points);
city = pg.computeNormals(city);
city = pg.color(city, { color: [0.6, 0.65, 0.75] });

const ground = pg.createGrid({ rows: 2, cols: 2, sizeX: 10, sizeY: 10 });
const groundN = pg.computeNormals(ground);
const groundC = pg.color(groundN, { color: [0.25, 0.3, 0.2] });

return pg.fuse(city, { distance: 0.001 });
`,

    fracture: `// Voronoi fractured box
let box = pg.createBox({ size: [1.5, 1.5, 1.5] });
let points = pg.scatter(box, { count: 6, seed: 42 });
let fractured = pg.voronoiFracture(box, points, { cutPlaneOffset: 0.1, createInsideFaces: true });
fractured = pg.computeNormals(fractured);
return fractured;
`,

    noiseDisplace: `// Noise-displaced terrain
// attribNoise defaults to operation: 'add'
let geo = pg.createGrid({ rows: 30, cols: 30, sizeX: 4, sizeY: 4 });
geo = pg.subdivide(geo, { depth: 1, mode: 'linear' });

// Layer 1: large hills (simplex fBm)
geo = pg.attribNoise(geo, {
  attribName: 'P', dimensions: 3,
  noiseType: 'simplex', fractal: 'standard',
  octaves: 4, elementSize: 2.0, amplitude: 0.5,
});

// Layer 2: fine detail added on top (operation defaults to 'add')
geo = pg.attribNoise(geo, {
  attribName: 'P', dimensions: 3,
  noiseType: 'perlin', fractal: 'standard',
  octaves: 6, elementSize: 0.5, amplitude: 0.08, seed: 99,
});

geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.35, 0.55, 0.25] });
return geo;
`,

    smoothSphere: `// Smoothed low-poly sphere
let geo = pg.createSphere({ radius: 1.0, rows: 4, cols: 6 });
geo = pg.subdivide(geo, { depth: 1, mode: 'linear' });
geo = pg.smooth(geo, { iterations: 5, strength: 0.8 });
geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.9, 0.4, 0.3] });
return geo;
`,

    clippedTorus: `// Clipped torus
let geo = pg.createTorus({ radiusOuter: 1.0, radiusInner: 0.35, rows: 16, cols: 32 });
geo = pg.clip(geo, { origin: [0, 0.1, 0], normal: [0, 1, 0], keepAbove: true });
geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.3, 0.5, 0.9] });
return geo;
`,

    allPrimitives: `// All primitive shapes
let box = pg.transform(pg.createBox(), { translate: [-3, 0, 0] });
box = pg.color(pg.computeNormals(box), { color: [0.9, 0.3, 0.3] });

let sphere = pg.transform(pg.createSphere({ rows: 8, cols: 16 }), { translate: [-1.5, 0, 0] });
sphere = pg.color(pg.computeNormals(sphere), { color: [0.3, 0.9, 0.3] });

let torus = pg.createTorus();
torus = pg.color(pg.computeNormals(torus), { color: [0.3, 0.3, 0.9] });

let tube = pg.transform(pg.createTube({ rows: 4 }), { translate: [1.5, 0, 0] });
tube = pg.color(pg.computeNormals(tube), { color: [0.9, 0.9, 0.3] });

let circle = pg.transform(pg.createCircle({ divisions: 24 }), { translate: [3, 0, 0] });
circle = pg.color(pg.computeNormals(circle), { color: [0.9, 0.3, 0.9] });

return box;
`,

    extrudeTower: `// Recursive extrude tower
let geo = pg.createBox({ size: [1.5, 0.3, 1.5] });
geo = pg.polyExtrude(geo, { distance: 0.8, inset: 0.15 });
geo = pg.polyExtrude(geo, { distance: 0.6, inset: 0.1 });
geo = pg.polyExtrude(geo, { distance: 0.4, inset: 0.08 });
geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.7, 0.65, 0.6] });
return geo;
`,

    reversedGrid: `// Reversed normals comparison
// Create two grids — one normal, one reversed
let grid = pg.createGrid({ rows: 8, cols: 8, sizeX: 2, sizeY: 2 });
grid = pg.computeNormals(grid);
grid = pg.color(grid, { color: [0.2, 0.7, 0.9] });

let reversed = pg.createGrid({ rows: 8, cols: 8, sizeX: 2, sizeY: 2 });
reversed = pg.transform(reversed, { translate: [3, 0, 0] });
reversed = pg.reverse(reversed);
reversed = pg.computeNormals(reversed);
reversed = pg.color(reversed, { color: [0.9, 0.3, 0.2] });

return grid;
`,

    noisyTorus: `// Noise-deformed torus with Worley cellular pattern
let geo = pg.createTorus({ radiusOuter: 1.0, radiusInner: 0.3, rows: 24, cols: 48 });

// Perlin displacement
geo = pg.attribNoise(geo, {
  attribName: 'P', dimensions: 3,
  noiseType: 'perlin', fractal: 'standard',
  octaves: 3, elementSize: 0.6, amplitude: 0.12,
});

// Worley cellular bumps (operation: 'add' stacks on top)
geo = pg.attribNoise(geo, {
  attribName: 'P', dimensions: 3,
  noiseType: 'worley',
  elementSize: 0.4, amplitude: 0.05, seed: 7,
});

geo = pg.smooth(geo, { iterations: 1, strength: 0.3 });
geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.95, 0.7, 0.3] });
return geo;
`,

    scatterForest: `// Scattered trees on terrain
let terrain = pg.createGrid({ rows: 20, cols: 20, sizeX: 6, sizeY: 6 });
terrain = pg.attribNoise(terrain, {
  attribName: 'P',
  dimensions: 3,
  noiseType: 'simplex',
  fractal: 'standard',
  octaves: 4,
  elementSize: 2.0,
  amplitude: 0.3,
});
terrain = pg.computeNormals(terrain);
terrain = pg.color(terrain, { color: [0.3, 0.5, 0.2] });

let treePositions = pg.scatter(terrain, { count: 40, seed: 13 });
let trunk = pg.createTube({ radiusBottom: 0.04, radiusTop: 0.02, height: 0.4, cols: 6, rows: 2 });
let trees = pg.copyToPoints(trunk, treePositions);
trees = pg.computeNormals(trees);
trees = pg.color(trees, { color: [0.45, 0.3, 0.15] });

return terrain;
`,

    building: `// Multi-color building with merge
function box(size, center, col) {
  let g = pg.createBox({ size, center });
  g = pg.color(g, { color: col });
  return g;
}

const parts = [];

// Main body
parts.push(box([4, 6, 3], [0, 3, 0], [0.88, 0.90, 0.92]));

// Window bands (teal glass)
for (let i = 0; i < 5; i++) {
  const y = 1.2 + i * 1.2;
  parts.push(box([3.8, 0.6, 0.2], [0, y, 1.55], [0.15, 0.38, 0.50]));
  parts.push(box([3.8, 0.6, 0.2], [0, y, -1.55], [0.15, 0.38, 0.50]));
  parts.push(box([0.2, 0.6, 2.8], [-2.05, y, 0], [0.15, 0.38, 0.50]));
  parts.push(box([0.2, 0.6, 2.8], [2.05, y, 0], [0.15, 0.38, 0.50]));
}

// Dark ground floor
parts.push(box([4, 0.8, 3], [0, 0.4, 0], [0.3, 0.35, 0.4]));

// Roof slab
parts.push(box([4.4, 0.2, 3.4], [0, 6.1, 0], [0.2, 0.24, 0.3]));

// Penthouse
parts.push(box([3, 1.0, 2], [0, 6.6, 0], [0.5, 0.53, 0.56]));

// Ground plane
parts.push(box([12, 0.05, 8], [0, -0.025, 0], [0.5, 0.52, 0.54]));

let scene = parts.reduce((a, b) => pg.merge(a, b));
scene = pg.computeNormals(scene);
return scene;
`,

    groupBooleans: `// Group boolean operations: union, intersect, subtract
// Each sphere shows a different boolean of "upper half" vs "front half"

function boolOp(operation: string, offset: [number, number, number], col: [number, number, number]) {
  let geo = pg.createSphere({ radius: 0.8, rows: 16, cols: 32 });

  // Group A: primitives in the upper half (y > 0)
  geo = pg.groupCreate(geo, {
    name: 'upper', groupType: 'primitives',
    mode: 'boundingBox', bboxMin: [-2, 0, -2], bboxMax: [2, 2, 2],
  });

  // Group B: primitives in the front half (z > 0)
  geo = pg.groupCreate(geo, {
    name: 'front', groupType: 'primitives',
    mode: 'boundingBox', bboxMin: [-2, -2, 0], bboxMax: [2, 2, 2],
  });

  // Boolean combine the two groups
  geo = pg.groupCombine(geo, {
    nameA: 'upper', nameB: 'front', result: 'result',
    operation, groupType: 'primitives',
  });

  // Keep only the result group (negate = delete everything NOT in group)
  geo = pg.blast(geo, { groupName: 'result', entity: 'primitives', negate: true });
  geo = pg.transform(geo, { translate: offset });
  geo = pg.computeNormals(geo);
  geo = pg.color(geo, { color: col });
  return geo;
}

const union     = boolOp('union',     [-2, 0, 0], [0.2, 0.8, 0.3]);  // top OR front
const intersect = boolOp('intersect', [ 0, 0, 0], [0.9, 0.6, 0.1]);  // top AND front
const subtract  = boolOp('subtract',  [ 2, 0, 0], [0.3, 0.5, 0.9]);  // top AND NOT front

let scene = pg.merge(pg.merge(union, intersect), subtract);
return scene;
`,

    metaballs: `// Metaballs — implicit boolean union via field blending
// Overlapping balls smoothly merge together
let geo = pg.createMetaball({
  balls: [
    { center: [ 0,    0.5, 0], radius: 0.6, weight: 1.0 },
    { center: [ 0.5,  0,   0], radius: 0.5, weight: 1.0 },
    { center: [-0.5,  0,   0], radius: 0.5, weight: 1.0 },
    { center: [ 0,   -0.4, 0.4], radius: 0.4, weight: 1.0 },
    { center: [ 0.3,  0.8, 0.2], radius: 0.3, weight: 0.8 },
  ],
  kernel: 'wyvill',
  threshold: 0.5,
  resolution: 64,
  padding: 0.3,
});
geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.85, 0.35, 0.5] });
return geo;
`,

    stressTest: `// Stress test — high-poly terrain + scattered instances
// Pushes subdivision, noise layering, scatter, and copyToPoints

// 1. Dense terrain grid with layered noise
let terrain = pg.createGrid({ rows: 100, cols: 100, sizeX: 12, sizeY: 12 });

terrain = pg.attribNoise(terrain, {
  attribName: 'P', dimensions: 3,
  noiseType: 'simplex', fractal: 'terrain',
  octaves: 6, elementSize: 3.0, amplitude: 1.5,
});
terrain = pg.attribNoise(terrain, {
  attribName: 'P', dimensions: 3,
  noiseType: 'perlin', fractal: 'standard',
  octaves: 4, elementSize: 0.6, amplitude: 0.1, seed: 42,
});
terrain = pg.computeNormals(terrain);
terrain = pg.color(terrain, { color: [0.35, 0.5, 0.25] });

// 2. Scatter subdivided rocks
const pts = pg.scatter(terrain, { count: 800, seed: 13 });
let rock = pg.createBox({ size: [0.06, 0.05, 0.06] });
rock = pg.subdivide(rock, { depth: 1, mode: 'catmullClark' });
let rocks = pg.copyToPoints(rock, pts);
rocks = pg.computeNormals(rocks);
rocks = pg.color(rocks, { color: [0.5, 0.48, 0.42] });

// 3. Taller scattered pillars
const pts2 = pg.scatter(terrain, { count: 120, seed: 77 });
let pillar = pg.createTube({ radiusBottom: 0.05, radiusTop: 0.03, height: 0.4, cols: 6, rows: 1 });
let pillars = pg.copyToPoints(pillar, pts2);
pillars = pg.computeNormals(pillars);
pillars = pg.color(pillars, { color: [0.45, 0.3, 0.15] });

let scene = pg.merge(pg.merge(terrain, rocks), pillars);
return scene;
`,

    fusedCubes: `// Fused overlapping cubes
let a = pg.createBox({ size: [1, 1, 1] });
let b = pg.createBox({ size: [1, 1, 1] });
b = pg.transform(b, { translate: [0.5, 0.5, 0.5] });
let c = pg.createBox({ size: [1, 1, 1] });
c = pg.transform(c, { translate: [-0.5, 0.3, 0.2] });

let geo = pg.fuse(a, { distance: 0.001 });
geo = pg.subdivide(geo, { depth: 1, mode: 'catmullClark' });
geo = pg.smooth(geo, { iterations: 2, strength: 0.5 });
geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.5, 0.7, 0.9] });
return geo;
`,
};
