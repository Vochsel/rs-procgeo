// Noise-displaced terrain
// attribNoise defaults to operation: 'add'
import type { ProcGeo, Geometry } from "procgeo";
declare const pg: ProcGeo;

let geo = pg.createGrid({ rows: 30, cols: 30, sizeX: 4, sizeY: 4 });
geo = pg.subdivide(geo, { depth: 1, mode: "linear" });

// Layer 1: large hills (simplex fBm)
geo = pg.attribNoise(geo, {
  attribName: "P",
  dimensions: 3,
  noiseType: "simplex",
  fractal: "standard",
  octaves: 4,
  elementSize: 2.0,
  amplitude: 0.5,
});

// Layer 2: fine detail added on top (operation defaults to 'add')
geo = pg.attribNoise(geo, {
  attribName: "P",
  dimensions: 3,
  noiseType: "perlin",
  fractal: "standard",
  octaves: 6,
  elementSize: 0.5,
  amplitude: 0.08,
  seed: 99,
});

geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.35, 0.55, 0.25] });
return geo;
