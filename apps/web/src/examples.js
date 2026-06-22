export const examples = {
    starterScene: `// Starter scene — displaced terrain, an extruded plinth, and a twisted spire
function mergeAll(parts) {
  return parts.slice(1).reduce((acc, part) => pg.merge(acc, part), parts[0]);
}

const footprint = [
  [-1.2, 0, -0.45],
  [-0.45, 0, -1.2],
  [0.45, 0, -1.2],
  [1.2, 0, -0.45],
  [1.2, 0, 0.45],
  [0.45, 0, 1.2],
  [-0.45, 0, 1.2],
  [-1.2, 0, 0.45],
];

let ground = pg.createGrid({ rows: 72, cols: 72, sizeX: 9, sizeY: 9 });
ground = pg.displace(ground, {
  direction: 'y',
  coordinates: 'boundingBox',
  projection: 'xz',
  strength: 0.7,
  midlevel: 0.46,
  noise: {
    noiseType: 'simplex',
    fractal: 'terrain',
    scale: [1.1, 1.1, 1.1],
    octaves: 5,
    lacunarity: 2.0,
    roughness: 0.52,
    seed: 12,
  },
});
ground = pg.displace(ground, {
  direction: 'y',
  coordinates: 'boundingBox',
  projection: 'xz',
  strength: 0.2,
  midlevel: 0.45,
  noise: {
    noiseType: 'worleyF2F1',
    scale: [3.4, 3.4, 3.4],
    octaves: 2,
    seed: 37,
  },
});
ground = pg.displace(ground, {
  direction: 'y',
  coordinates: 'boundingBox',
  projection: 'xz',
  strength: 0.06,
  midlevel: 0.5,
  noise: {
    noiseType: 'perlin',
    fractal: 'standard',
    scale: [7.5, 7.5, 7.5],
    octaves: 3,
    roughness: 0.55,
    seed: 8,
  },
});
ground = pg.smooth(ground, { iterations: 1, strength: 0.12 });
ground = pg.color(pg.computeNormals(ground), { color: [0.23, 0.3, 0.25] });

let plinth = pg.add(null, {
  points: footprint,
  polygons: [[7, 6, 5, 4, 3, 2, 1, 0]],
});
plinth = pg.polyExtrude(plinth, { distance: 0.3, inset: 0.08 });
plinth = pg.transform(plinth, { translate: [0, 0.22, 0] });
plinth = pg.color(pg.computeNormals(plinth), { color: [0.65, 0.61, 0.56] });

let spire = pg.createBox({ size: [0.9, 3.2, 0.9], center: [0, 1.6, 0] });
spire = pg.subdivide(spire, { depth: 2, mode: 'linear' });
spire = pg.bend(spire, {
  twistEnable: true,
  twistAngle: 240,
  captureOrigin: [0, 0, 0],
  captureDirection: [0, 1, 0],
  captureLength: 3.2,
});
spire = pg.color(pg.computeNormals(spire), { color: [0.7, 0.78, 0.88] });

let halo = pg.createTorus({
  radiusOuter: 1.35,
  radiusInner: 0.14,
  rows: 20,
  cols: 36,
  center: [0, 2.6, 0],
});
halo = pg.displace(halo, {
  direction: 'normal',
  strength: 0.08,
  midlevel: 0.5,
  noise: {
    noiseType: 'worley',
    scale: [4.0, 4.0, 4.0],
    octaves: 2,
    seed: 5,
  },
});
halo = pg.color(pg.computeNormals(halo), { color: [0.93, 0.67, 0.25] });

return mergeAll([ground, plinth, spire, halo]);
`,

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

    attribTransfer: `// Attribute transfer from a stretched sphere onto a dense grid
const d = 1000;
let p = pg.createGrid({
  rows: d,
  cols: d,
});
p = pg.color(p, { color: [1, 1, 1] });

let s = pg.createSphere();
s = pg.transform(s, {
  translate: [2, 0, 0],
  scale: [5, 1, 1],
});
s = pg.color(s, {
  color: [1, 0.5, 0],
});

p = pg.attribTransfer(p, s, {
  attribName: 'Cd',
  attribType: 'Vector3',
  distanceThreshold: 0.5,
});

const scene = pg.merge(p, s);
return scene;
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

    textureDisplace: `// Texture displacement from inline RGBA height data
const heights = [
  [0.0, 0.12, 0.28, 0.0],
  [0.18, 0.45, 0.72, 0.2],
  [0.06, 0.35, 1.0, 0.38],
  [0.0, 0.16, 0.4, 0.08],
];

const pixels = [];
for (const row of heights) {
  for (const h of row) {
    pixels.push(h, h, h, 1.0);
  }
}

let geo = pg.createGrid({ rows: 96, cols: 96, sizeX: 5, sizeY: 5 });
geo = pg.displace(geo, {
  texture: {
    width: 4,
    height: 4,
    pixels,
  },
  direction: 'y',
  coordinates: 'boundingBox',
  projection: 'xz',
  sampler: 'bilinear',
  wrap: 'clamp',
  strength: 1.25,
  midlevel: 0.0,
});
geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.36, 0.52, 0.32] });
return geo;
`,

    copDisplaceImage: `// COP-generated heightmap driving displaceImage (async)
const size = 256;

const hills = pg.copNoise({
  noiseType: 'simplex',
  frequency: 2.2,
  octaves: 4,
  lacunarity: 2.0,
  gain: 0.5,
  amplitude: 1.0,
  seed: 3,
  width: size,
  height: size,
});

const ridges = pg.copNoise({
  noiseType: 'worley',
  frequency: 7.0,
  octaves: 2,
  amplitude: 0.28,
  seed: 17,
  width: size,
  height: size,
});

const heightmap = pg.copComposite(hills, ridges, {
  operation: 'screen',
  mix: 0.6,
});

let geo = pg.createGrid({ rows: 120, cols: 120, sizeX: 6, sizeY: 6 });
geo = await pg.displaceImage(geo, heightmap, {
  direction: 'y',
  coordinates: 'boundingBox',
  projection: 'xz',
  sampleChannel: 'luminance',
  sampler: 'bilinear',
  wrap: 'clamp',
  strength: 1.1,
  midlevel: 0.42,
});
geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.34, 0.47, 0.31] });
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
let pillar = pg.createTube({ radiusBottom: 0.05, radiusTop: 0.03, height: 0.4, cols: 6, rows: 2 });
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

    // ── COP (Image Compositing) Examples ──────────────────

    copTerrainHeightmap: `// Terrain heightmap — layered fBm noise
// Broad hills + medium ridges + fine Worley detail
const size = 256;

const hills = pg.copNoise({
  noiseType: 'simplex', frequency: 2.0, octaves: 4,
  lacunarity: 2.0, gain: 0.5, amplitude: 1.0,
  seed: 0, width: size, height: size,
});

const ridges = pg.copNoise({
  noiseType: 'perlin', frequency: 6.0, octaves: 6,
  lacunarity: 2.2, gain: 0.45, amplitude: 0.4,
  seed: 42, width: size, height: size,
});

const detail = pg.copNoise({
  noiseType: 'worley', frequency: 12.0, octaves: 2,
  amplitude: 0.15, seed: 99, width: size, height: size,
});

let terrain = pg.copComposite(hills, ridges, {
  operation: 'add', mix: 0.6,
});
terrain = pg.copComposite(terrain, detail, {
  operation: 'screen', mix: 0.4,
});
terrain = pg.copBlur(terrain, { radiusX: 2.0, radiusY: 2.0 });
return terrain;
`,

    copNeonGrid: `// Neon grid — checkerboard + radial glow + swirl
const size = 256;

const checker = pg.copCheckerboard({
  colorA: [0.9, 0.1, 0.9, 1.0],
  colorB: [0.1, 0.9, 0.9, 1.0],
  frequency: [12.0, 12.0],
  width: size, height: size,
});

const glow = pg.copRamp({
  rampType: 'radial',
  stops: [
    { position: 0.0, color: [1.0, 1.0, 1.0, 1.0] },
    { position: 0.6, color: [0.4, 0.2, 0.6, 1.0] },
    { position: 1.0, color: [0.02, 0.01, 0.05, 1.0] },
  ],
  width: size, height: size,
});

let result = pg.copComposite(checker, glow, {
  operation: 'multiply', mix: 1.0,
});
result = pg.copSwirl(result, {
  center: [0.5, 0.5], angle: 120.0, radius: 0.6,
});
return result;
`,

    copMarbleTexture: `// Marble texture — Worley veins over tinted Perlin base
const size = 256;

const base = pg.copNoise({
  noiseType: 'perlin', frequency: 3.0, octaves: 4,
  amplitude: 1.0, seed: 7, width: size, height: size,
});

const veins = pg.copNoise({
  noiseType: 'worley', frequency: 5.0, octaves: 3,
  lacunarity: 2.5, gain: 0.6, amplitude: 1.0,
  seed: 33, width: size, height: size,
});

const colorRamp = pg.copRamp({
  rampType: 'diagonal',
  stops: [
    { position: 0.0, color: [0.92, 0.88, 0.82, 1.0] },
    { position: 0.35, color: [0.85, 0.78, 0.70, 1.0] },
    { position: 0.65, color: [0.70, 0.62, 0.55, 1.0] },
    { position: 1.0, color: [0.55, 0.48, 0.42, 1.0] },
  ],
  width: size, height: size,
});

let marble = pg.copComposite(colorRamp, base, {
  operation: 'multiply', mix: 0.7,
});
marble = pg.copComposite(marble, veins, {
  operation: 'screen', mix: 0.3,
});
marble = pg.copBlur(marble, { radiusX: 1.5, radiusY: 1.5 });
return marble;
`,

    copPlasmaShader: `// Plasma shader — custom WGSL interference pattern
const size = 256;
const source = \`
@group(0) @binding(0) var output: texture_storage_2d<rgba32float, write>;

fn psin(x: f32) -> f32 {
    return sin(fract(x / 6.2831853) * 6.2831853);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let dims = textureDimensions(output);
    if gid.x >= dims.x || gid.y >= dims.y { return; }
    let uv = vec2f(f32(gid.x), f32(gid.y)) / vec2f(f32(dims.x), f32(dims.y));
    let p = uv * 8.0;

    var v = 0.0;
    v += psin(p.x + psin(p.y * 1.3 + 1.7));
    v += psin(p.y * 0.9 + psin(p.x * 1.1 + 2.3));
    v += psin(length(p - vec2f(4.0, 4.0)) * 1.5);
    v += psin(length(p - vec2f(2.0, 6.0)) * 2.0 + 0.5);
    v = v * 0.25 + 0.5;

    let r = psin(v * 6.2831853) * 0.5 + 0.5;
    let g = psin(v * 6.2831853 + 2.094) * 0.5 + 0.5;
    let b = psin(v * 6.2831853 + 4.189) * 0.5 + 0.5;
    textureStore(output, vec2i(gid.xy), vec4f(r, g, b, 1.0));
}
\`;
return pg.copCustomShader(null, null, {
  source, language: 'wgsl', width: size, height: size,
});
`,

    copGlowEffect: `// Bloom / glow — dual-layer blur composited over source
const size = 256;

const checker = pg.copCheckerboard({
  colorA: [0.0, 0.0, 0.0, 1.0],
  colorB: [1.0, 0.7, 0.2, 1.0],
  frequency: [6.0, 6.0],
  width: size, height: size,
});

const noise = pg.copNoise({
  noiseType: 'simplex', frequency: 8.0, octaves: 3,
  amplitude: 1.0, seed: 5, width: size, height: size,
});

let source = pg.copComposite(checker, noise, {
  operation: 'multiply', mix: 0.4,
});

const bloomWide = pg.copBlur(source, { radiusX: 20, radiusY: 20 });
const bloomTight = pg.copBlur(source, { radiusX: 8, radiusY: 8 });

let bloom = pg.copComposite(bloomWide, bloomTight, {
  operation: 'add', mix: 0.5,
});

let glowed = pg.copComposite(source, bloom, {
  operation: 'add', mix: 0.6,
});

const vignette = pg.copRamp({
  rampType: 'radial',
  stops: [
    { position: 0.0, color: [1.0, 1.0, 1.0, 1.0] },
    { position: 0.5, color: [0.9, 0.9, 0.9, 1.0] },
    { position: 1.0, color: [0.15, 0.1, 0.05, 1.0] },
  ],
  width: size, height: size,
});

return pg.copComposite(glowed, vignette, {
  operation: 'multiply', mix: 1.0,
});
`,

    copRustyMetal: `// Rusty metal — corroded steel with patina and pitting
const size = 512;

// Base steel: dark, mostly uniform gray
const steel = pg.copNoise({
  noiseType: 'perlin', frequency: 1.5, octaves: 2,
  amplitude: 0.15, seed: 10, width: size, height: size,
});
const steelColor = pg.copRamp({
  rampType: 'linear',
  stops: [
    { position: 0.0, color: [0.22, 0.22, 0.24, 1.0] },
    { position: 1.0, color: [0.35, 0.34, 0.33, 1.0] },
  ],
  width: size, height: size,
});
let base = pg.copComposite(steelColor, steel, {
  operation: 'multiply', mix: 1.0,
});

// Rust patches: warm orange-brown Simplex noise at medium scale
const rustMask = pg.copNoise({
  noiseType: 'simplex', frequency: 3.0, octaves: 5,
  lacunarity: 2.0, gain: 0.55, amplitude: 1.0,
  seed: 77, width: size, height: size,
});
const rustColor = pg.copRamp({
  rampType: 'linear',
  stops: [
    { position: 0.0, color: [0.45, 0.18, 0.05, 1.0] },
    { position: 0.4, color: [0.62, 0.28, 0.08, 1.0] },
    { position: 0.7, color: [0.50, 0.22, 0.06, 1.0] },
    { position: 1.0, color: [0.35, 0.12, 0.04, 1.0] },
  ],
  width: size, height: size,
});
const rust = pg.copComposite(rustColor, rustMask, {
  operation: 'multiply', mix: 1.0,
});

// Blend rust onto steel using screen for natural layering
let result = pg.copComposite(base, rust, {
  operation: 'screen', mix: 0.7,
});

// Pitting: fine Worley craters darkening the surface
const pits = pg.copNoise({
  noiseType: 'worley', frequency: 20.0, octaves: 1,
  amplitude: 0.6, seed: 55, width: size, height: size,
});
result = pg.copComposite(result, pits, {
  operation: 'multiply', mix: 0.25,
});

// Subtle blur for realism
result = pg.copBlur(result, { radiusX: 0.8, radiusY: 0.8 });
return result;
`,

    copWoodGrain: `// Wood grain — rings with knots and color variation
const size = 512;

// Ring structure: stretched Perlin creates directional grain
const rings = pg.copNoise({
  noiseType: 'perlin', frequency: 2.0, octaves: 6,
  lacunarity: 2.0, gain: 0.5, amplitude: 1.0,
  offset: [0.0, 0.0], seed: 3, width: size, height: size,
});

// Fine grain detail: high-frequency noise for wood fiber
const grain = pg.copNoise({
  noiseType: 'perlin', frequency: 40.0, octaves: 2,
  amplitude: 0.3, seed: 19, width: size, height: size,
});

// Knot disturbance: low-frequency Worley for organic knot shapes
const knots = pg.copNoise({
  noiseType: 'worley', frequency: 1.5, octaves: 2,
  amplitude: 0.5, seed: 61, width: size, height: size,
});

// Warm wood color ramp
const woodColor = pg.copRamp({
  rampType: 'linear',
  stops: [
    { position: 0.0, color: [0.35, 0.20, 0.10, 1.0] },
    { position: 0.25, color: [0.55, 0.35, 0.18, 1.0] },
    { position: 0.5, color: [0.65, 0.42, 0.22, 1.0] },
    { position: 0.75, color: [0.50, 0.30, 0.15, 1.0] },
    { position: 1.0, color: [0.40, 0.22, 0.10, 1.0] },
  ],
  width: size, height: size,
});

// Build the base: color modulated by ring pattern
let wood = pg.copComposite(woodColor, rings, {
  operation: 'multiply', mix: 0.8,
});

// Add fine grain texture
wood = pg.copComposite(wood, grain, {
  operation: 'multiply', mix: 0.15,
});

// Screen in knot regions for lighter, organic marks
wood = pg.copComposite(wood, knots, {
  operation: 'screen', mix: 0.2,
});

// Gentle horizontal blur to emphasize grain direction
wood = pg.copBlur(wood, { radiusX: 2.0, radiusY: 0.5 });
return wood;
`,

    copLavaFlow: `// Lava flow — incandescent magma with cooling crust
const size = 512;

// Hot magma base: bright orange-yellow Simplex turbulence
const magma = pg.copNoise({
  noiseType: 'simplex', frequency: 3.0, octaves: 6,
  lacunarity: 2.2, gain: 0.5, amplitude: 1.0,
  seed: 88, width: size, height: size,
});
const heatRamp = pg.copRamp({
  rampType: 'linear',
  stops: [
    { position: 0.0, color: [1.0, 0.95, 0.4, 1.0] },
    { position: 0.3, color: [1.0, 0.6, 0.05, 1.0] },
    { position: 0.6, color: [0.9, 0.25, 0.0, 1.0] },
    { position: 1.0, color: [0.4, 0.05, 0.0, 1.0] },
  ],
  width: size, height: size,
});
let lava = pg.copComposite(heatRamp, magma, {
  operation: 'multiply', mix: 1.0,
});

// Cooling crust: Worley cell boundaries form dark rock plates
const crust = pg.copNoise({
  noiseType: 'worley', frequency: 6.0, octaves: 3,
  lacunarity: 2.0, gain: 0.5, amplitude: 1.0,
  seed: 14, width: size, height: size,
});
const crustColor = pg.copRamp({
  rampType: 'linear',
  stops: [
    { position: 0.0, color: [0.08, 0.04, 0.02, 1.0] },
    { position: 0.5, color: [0.15, 0.08, 0.04, 1.0] },
    { position: 1.0, color: [0.25, 0.12, 0.06, 1.0] },
  ],
  width: size, height: size,
});
const darkCrust = pg.copComposite(crustColor, crust, {
  operation: 'multiply', mix: 1.0,
});

// Composite: crust darkens everything, then hot cracks show through
let result = pg.copComposite(darkCrust, lava, {
  operation: 'screen', mix: 0.8,
});

// Emissive bloom: blur and add back for glow in cracks
const glow = pg.copBlur(lava, { radiusX: 12.0, radiusY: 12.0 });
result = pg.copComposite(result, glow, {
  operation: 'add', mix: 0.3,
});
return result;
`,

    copCamouflage: `// Military camo — organic blobs in earthy tones
const size = 512;

// Layer 1: base tan
const base = pg.copConstant({
  color: [0.55, 0.50, 0.35, 1.0],
  width: size, height: size,
});

// Layer 2: large dark green blobs
const blobsGreen = pg.copNoise({
  noiseType: 'simplex', frequency: 2.5, octaves: 3,
  lacunarity: 1.8, gain: 0.6, amplitude: 1.0,
  seed: 20, width: size, height: size,
});
const green = pg.copConstant({
  color: [0.22, 0.32, 0.15, 1.0],
  width: size, height: size,
});
const greenMasked = pg.copComposite(green, blobsGreen, {
  operation: 'multiply', mix: 1.0,
});
let result = pg.copComposite(base, greenMasked, {
  operation: 'screen', mix: 0.8,
});

// Layer 3: medium brown patches
const blobsBrown = pg.copNoise({
  noiseType: 'simplex', frequency: 3.5, octaves: 3,
  lacunarity: 2.0, gain: 0.5, amplitude: 1.0,
  seed: 44, width: size, height: size,
});
const brown = pg.copConstant({
  color: [0.35, 0.22, 0.10, 1.0],
  width: size, height: size,
});
const brownMasked = pg.copComposite(brown, blobsBrown, {
  operation: 'multiply', mix: 1.0,
});
result = pg.copComposite(result, brownMasked, {
  operation: 'screen', mix: 0.6,
});

// Layer 4: small dark splotches
const splotches = pg.copNoise({
  noiseType: 'simplex', frequency: 5.0, octaves: 2,
  amplitude: 0.8, seed: 66, width: size, height: size,
});
const dark = pg.copConstant({
  color: [0.10, 0.10, 0.08, 1.0],
  width: size, height: size,
});
const darkMasked = pg.copComposite(dark, splotches, {
  operation: 'multiply', mix: 1.0,
});
result = pg.copComposite(result, darkMasked, {
  operation: 'screen', mix: 0.4,
});

// Soften edges for organic look
result = pg.copBlur(result, { radiusX: 3.0, radiusY: 3.0 });
return result;
`,

    copOceanCaustics: `// Ocean caustics — shimmering underwater light patterns
const size = 512;

// Dual-layer Worley noise creates caustic interference
const caustics1 = pg.copNoise({
  noiseType: 'worley', frequency: 8.0, octaves: 3,
  lacunarity: 2.0, gain: 0.6, amplitude: 1.0,
  seed: 7, width: size, height: size,
});
const caustics2 = pg.copNoise({
  noiseType: 'worley', frequency: 10.0, octaves: 3,
  lacunarity: 2.2, gain: 0.55, amplitude: 1.0,
  seed: 31, width: size, height: size,
});

// Combine two Worley layers with min to get sharp bright edges
let pattern = pg.copComposite(caustics1, caustics2, {
  operation: 'min', mix: 1.0,
});

// Add subtle undulation with swirl
pattern = pg.copSwirl(pattern, {
  center: [0.45, 0.55], angle: 25.0, radius: 0.8,
});

// Color map: deep blue to bright aqua highlights
const waterColor = pg.copRamp({
  rampType: 'linear',
  stops: [
    { position: 0.0, color: [0.02, 0.06, 0.18, 1.0] },
    { position: 0.3, color: [0.05, 0.15, 0.35, 1.0] },
    { position: 0.6, color: [0.10, 0.35, 0.50, 1.0] },
    { position: 0.85, color: [0.30, 0.70, 0.80, 1.0] },
    { position: 1.0, color: [0.60, 0.95, 1.0, 1.0] },
  ],
  width: size, height: size,
});

let result = pg.copComposite(waterColor, pattern, {
  operation: 'multiply', mix: 1.0,
});

// Soft bloom for the bright caustic lines
const bloom = pg.copBlur(result, { radiusX: 6.0, radiusY: 6.0 });
result = pg.copComposite(result, bloom, {
  operation: 'add', mix: 0.25,
});
return result;
`,

    copTiledMosaic: `// Tiled mosaic — geometric tiles with grout lines and color variation
const size = 512;

// High-frequency checkerboard for tile grid
const tiles = pg.copCheckerboard({
  colorA: [0.85, 0.82, 0.75, 1.0],
  colorB: [0.65, 0.60, 0.55, 1.0],
  frequency: [16.0, 16.0],
  width: size, height: size,
});

// Color variation per tile region using low-freq noise
const tint = pg.copNoise({
  noiseType: 'simplex', frequency: 4.0, octaves: 2,
  amplitude: 0.6, seed: 5, width: size, height: size,
});
const colorPalette = pg.copRamp({
  rampType: 'linear',
  stops: [
    { position: 0.0, color: [0.15, 0.30, 0.55, 1.0] },
    { position: 0.25, color: [0.20, 0.50, 0.45, 1.0] },
    { position: 0.5, color: [0.55, 0.35, 0.20, 1.0] },
    { position: 0.75, color: [0.50, 0.20, 0.25, 1.0] },
    { position: 1.0, color: [0.25, 0.25, 0.50, 1.0] },
  ],
  width: size, height: size,
});
const tileColor = pg.copComposite(colorPalette, tint, {
  operation: 'multiply', mix: 1.0,
});

// Combine tile structure with color
let result = pg.copComposite(tiles, tileColor, {
  operation: 'multiply', mix: 0.8,
});

// Grout lines: fine Worley edges create the gap between tiles
const grout = pg.copNoise({
  noiseType: 'worley', frequency: 16.0, octaves: 1,
  amplitude: 1.0, seed: 12, width: size, height: size,
});
const groutColor = pg.copConstant({
  color: [0.30, 0.28, 0.25, 1.0],
  width: size, height: size,
});
const groutLines = pg.copComposite(groutColor, grout, {
  operation: 'screen', mix: 0.5,
});
result = pg.copComposite(result, groutLines, {
  operation: 'multiply', mix: 0.6,
});

// Surface wear: subtle noise overlay
const wear = pg.copNoise({
  noiseType: 'perlin', frequency: 12.0, octaves: 3,
  amplitude: 0.2, seed: 40, width: size, height: size,
});
result = pg.copComposite(result, wear, {
  operation: 'multiply', mix: 0.15,
});
return result;
`,

    // ── Deform SOP Examples ──────────────────────────────────

    bendTube: `// Bent tube — 90° L-shaped pipe
let geo = pg.createTube({ radiusBottom: 0.15, radiusTop: 0.15, height: 3, cols: 24, rows: 16 });
geo = pg.bend(geo, {
  bendEnable: true,
  bendAngle: 90,
  captureOrigin: [0, -1.5, 0],
  captureDirection: [0, 1, 0],
  captureLength: 3.0,
  upVector: [0, 0, 1],
});
geo = pg.color(geo, { color: [0.8, 0.45, 0.2] });
geo = pg.computeNormals(geo);
return geo;
`,

    twistColumn: `// Twisted column — 360° twist along full height
let geo = pg.createBox({ size: [0.6, 4, 0.6] });
geo = pg.subdivide(geo, { depth: 3, mode: 'linear' });
geo = pg.bend(geo, {
  twistEnable: true,
  twistAngle: 360,
  captureOrigin: [0, -2, 0],
  captureDirection: [0, 1, 0],
  captureLength: 4.0,
});
geo = pg.color(geo, { color: [0.55, 0.6, 0.75] });
geo = pg.computeNormals(geo);
return geo;
`,

    squashStretch: `// Squash & stretch — volume-preserving length scale on a sphere
let geo = pg.createSphere({ radius: 1.0, rows: 16, cols: 32 });
geo = pg.subdivide(geo, { depth: 1, mode: 'linear' });
geo = pg.bend(geo, {
  lengthScaleEnable: true,
  lengthScale: 0.5,
  preserveVolume: true,
  captureOrigin: [0, -1, 0],
  captureDirection: [0, 1, 0],
  captureLength: 2.0,
});
geo = pg.color(geo, { color: [0.9, 0.55, 0.3] });
geo = pg.computeNormals(geo);
return geo;
`,

    taperCone: `// Tapered cone — cylinder tapered to a point
let geo = pg.createTube({ radiusBottom: 0.5, radiusTop: 0.5, height: 2, cols: 24, rows: 12 });
geo = pg.subdivide(geo, { depth: 1, mode: 'linear' });
geo = pg.bend(geo, {
  taperEnable: true,
  taperValue: 0,
  captureOrigin: [0, -1, 0],
  captureDirection: [0, 1, 0],
  captureLength: 2.0,
});
geo = pg.color(geo, { color: [0.4, 0.7, 0.5] });
geo = pg.computeNormals(geo);
return geo;
`,

    pointDeformWave: `// Point deform wave — grid deformed by a displaced lattice
// Create the mesh to deform
let grid = pg.createGrid({ rows: 40, cols: 40, sizeX: 4, sizeY: 4 });

// Build a rest lattice: a line of points along X
let restLattice = pg.createGrid({ rows: 1, cols: 10, sizeX: 4, sizeY: 0.01 });

// Build a deformed lattice: same points with sine-wave Y displacement
let deformedLattice = pg.createGrid({ rows: 1, cols: 10, sizeX: 4, sizeY: 0.01 });
deformedLattice = pg.attribNoise(deformedLattice, {
  attribName: 'P', dimensions: 3,
  noiseType: 'simplex', fractal: 'standard',
  octaves: 2, elementSize: 1.5, amplitude: 0.6, seed: 7,
});

// Deform the grid using the lattice pair
let geo = pg.pointDeform(grid, restLattice, deformedLattice, { radius: 2.0 });
geo = pg.color(geo, { color: [0.3, 0.6, 0.9] });
geo = pg.computeNormals(geo);
return geo;
`,

    // ── Boolean CSG Examples ─────────────────────────────────

    booleanUnion: `// Boolean union — box + sphere merged
let box = pg.createBox({ size: [1.2, 1.2, 1.2] });
let sphere = pg.createSphere({ radius: 0.8, rows: 16, cols: 32 });
sphere = pg.transform(sphere, { translate: [0.5, 0.5, 0] });
let geo = pg.booleanOp(box, sphere, { operation: 'union' });
geo = pg.color(geo, { color: [0.3, 0.75, 0.5] });
geo = pg.computeNormals(geo);
return geo;
`,

    booleanSubtract: `// Boolean subtract — spherical hole punched through a box
let box = pg.createBox({ size: [1.5, 1.5, 1.5] });
let sphere = pg.createSphere({ radius: 1.0, rows: 16, cols: 32 });
sphere = pg.transform(sphere, { translate: [0.4, 0.4, 0.4] });
let geo = pg.booleanOp(box, sphere, { operation: 'subtract' });
geo = pg.color(geo, { color: [0.85, 0.5, 0.3] });
geo = pg.computeNormals(geo);
return geo;
`,

    quadWild: `// QuadWild — field-aligned quad remeshing
// Converts a triangle mesh into a quad-dominant mesh
let geo = pg.createSphere({ radius: 1.0, rows: 8, cols: 16 });
geo = pg.subdivide(geo, { depth: 1, mode: 'linear' });
geo = pg.quadWild(geo, {
  sharpAngle: 35,
  scaleFactor: 1.5,
  curvatureWeight: 0.3,
  smoothIterations: 15,
  postSmoothIterations: 20,
});
geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.3, 0.7, 0.9] });
return geo;
`,

    quadWildBox: `// QuadWild on a subdivided box
// Sharp features at 90° edges are preserved
let geo = pg.createBox({ size: [1.5, 1.5, 1.5] });
geo = pg.subdivide(geo, { depth: 2, mode: 'linear' });
geo = pg.quadWild(geo, {
  sharpAngle: 45,
  scaleFactor: 1.0,
});
geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.9, 0.6, 0.3] });
return geo;
`,

    booleanIntersect: `// Boolean intersect — only the overlapping region
let box = pg.createBox({ size: [1.5, 1.5, 1.5] });
let sphere = pg.createSphere({ radius: 1.0, rows: 16, cols: 32 });
sphere = pg.transform(sphere, { translate: [0.4, 0.4, 0] });
let geo = pg.booleanOp(box, sphere, { operation: 'intersect' });
geo = pg.color(geo, { color: [0.5, 0.4, 0.9] });
geo = pg.computeNormals(geo);
return geo;
`,
};
