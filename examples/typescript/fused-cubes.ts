// Fused overlapping cubes
import type { ProcGeo, Geometry } from "procgeo";
declare const pg: ProcGeo;

let a = pg.createBox({ size: [1, 1, 1] });
let b = pg.createBox({ size: [1, 1, 1] });
b = pg.transform(b, { translate: [0.5, 0.5, 0.5] });
let c = pg.createBox({ size: [1, 1, 1] });
c = pg.transform(c, { translate: [-0.5, 0.3, 0.2] });

let geo = pg.fuse(a, { distance: 0.001 });
geo = pg.subdivide(geo, { depth: 1, mode: "catmullClark" });
geo = pg.smooth(geo, { iterations: 2, strength: 0.5 });
geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.5, 0.7, 0.9] });
return geo;
