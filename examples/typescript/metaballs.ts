// Metaballs — implicit boolean union via field blending
// Overlapping balls smoothly merge together
import type { ProcGeo, Geometry } from "procgeo";
declare const pg: ProcGeo;

let geo = pg.createMetaball({
  balls: [
    { center: [0, 0.5, 0], radius: 0.6, weight: 1.0 },
    { center: [0.5, 0, 0], radius: 0.5, weight: 1.0 },
    { center: [-0.5, 0, 0], radius: 0.5, weight: 1.0 },
    { center: [0, -0.4, 0.4], radius: 0.4, weight: 1.0 },
    { center: [0.3, 0.8, 0.2], radius: 0.3, weight: 0.8 },
  ],
  kernel: "wyvill",
  threshold: 0.5,
  resolution: 64,
  padding: 0.3,
});
geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.85, 0.35, 0.5] });
return geo;
