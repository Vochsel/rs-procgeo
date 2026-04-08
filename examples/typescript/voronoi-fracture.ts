// Voronoi fractured box
import type { ProcGeo, Geometry } from "procgeo";
declare const pg: ProcGeo;

let box = pg.createBox({ size: [1.5, 1.5, 1.5] });
let points = pg.scatter(box, { count: 6, seed: 42 });
let fractured = pg.voronoiFracture(box, points, {
  cutPlaneOffset: 0.1,
  createInsideFaces: true,
});
fractured = pg.computeNormals(fractured);
return fractured;
