// Clipped torus
import type { ProcGeo, Geometry } from "procgeo";
declare const pg: ProcGeo;

let geo = pg.createTorus({
  radiusOuter: 1.0,
  radiusInner: 0.35,
  rows: 16,
  cols: 32,
});
geo = pg.clip(geo, { origin: [0, 0.1, 0], normal: [0, 1, 0], keepAbove: true });
geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.3, 0.5, 0.9] });
return geo;
