// Stress test — high-poly terrain + scattered instances
// Pushes subdivision, noise layering, scatter, and copyToPoints
import type { ProcGeo, Geometry } from "procgeo";
declare const pg: ProcGeo;

// 1. Dense terrain grid with layered noise
let terrain = pg.createGrid({ rows: 100, cols: 100, sizeX: 12, sizeY: 12 });

terrain = pg.attribNoise(terrain, {
  attribName: "P",
  dimensions: 3,
  noiseType: "simplex",
  fractal: "terrain",
  octaves: 6,
  elementSize: 3.0,
  amplitude: 1.5,
});
terrain = pg.attribNoise(terrain, {
  attribName: "P",
  dimensions: 3,
  noiseType: "perlin",
  fractal: "standard",
  octaves: 4,
  elementSize: 0.6,
  amplitude: 0.1,
  seed: 42,
});
terrain = pg.computeNormals(terrain);
terrain = pg.color(terrain, { color: [0.35, 0.5, 0.25] });

// 2. Scatter subdivided rocks
const pts = pg.scatter(terrain, { count: 800, seed: 13 });
let rock = pg.createBox({ size: [0.06, 0.05, 0.06] });
rock = pg.subdivide(rock, { depth: 1, mode: "catmullClark" });
let rocks = pg.copyToPoints(rock, pts);
rocks = pg.computeNormals(rocks);
rocks = pg.color(rocks, { color: [0.5, 0.48, 0.42] });

// 3. Taller scattered pillars
const pts2 = pg.scatter(terrain, { count: 120, seed: 77 });
let pillar = pg.createTube({
  radiusBottom: 0.05,
  radiusTop: 0.03,
  height: 0.4,
  cols: 6,
  rows: 2,
});
let pillars = pg.copyToPoints(pillar, pts2);
pillars = pg.computeNormals(pillars);
pillars = pg.color(pillars, { color: [0.45, 0.3, 0.15] });

let scene = pg.merge(pg.merge(terrain, rocks), pillars);
return scene;
