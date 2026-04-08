// Noise-deformed torus with Worley cellular pattern
import type { ProcGeo, Geometry } from "procgeo";
declare const pg: ProcGeo;

let geo = pg.createTorus({
  radiusOuter: 1.0,
  radiusInner: 0.3,
  rows: 24,
  cols: 48,
});

// Perlin displacement
geo = pg.attribNoise(geo, {
  attribName: "P",
  dimensions: 3,
  noiseType: "perlin",
  fractal: "standard",
  octaves: 3,
  elementSize: 0.6,
  amplitude: 0.12,
});

// Worley cellular bumps (operation: 'add' stacks on top)
geo = pg.attribNoise(geo, {
  attribName: "P",
  dimensions: 3,
  noiseType: "worley",
  elementSize: 0.4,
  amplitude: 0.05,
  seed: 7,
});

geo = pg.smooth(geo, { iterations: 1, strength: 0.3 });
geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.95, 0.7, 0.3] });
return geo;
