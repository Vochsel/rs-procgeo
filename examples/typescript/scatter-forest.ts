// Scattered trees on terrain
import type { ProcGeo, Geometry } from "procgeo";
declare const pg: ProcGeo;

let terrain = pg.createGrid({ rows: 20, cols: 20, sizeX: 6, sizeY: 6 });
terrain = pg.attribNoise(terrain, {
  attribName: "P",
  dimensions: 3,
  noiseType: "simplex",
  fractal: "standard",
  octaves: 4,
  elementSize: 2.0,
  amplitude: 0.3,
});
terrain = pg.computeNormals(terrain);
terrain = pg.color(terrain, { color: [0.3, 0.5, 0.2] });

let treePositions = pg.scatter(terrain, { count: 40, seed: 13 });
let trunk = pg.createTube({
  radiusBottom: 0.04,
  radiusTop: 0.02,
  height: 0.4,
  cols: 6,
  rows: 2,
});
let trees = pg.copyToPoints(trunk, treePositions);
trees = pg.computeNormals(trees);
trees = pg.color(trees, { color: [0.45, 0.3, 0.15] });

return terrain;
