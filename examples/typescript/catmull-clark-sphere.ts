// Catmull-Clark subdivided sphere
import type { ProcGeo, Geometry } from "procgeo";
declare const pg: ProcGeo;

let geo = pg.createSphere({ radius: 0.8, rows: 4, cols: 8 });
geo = pg.subdivide(geo, { depth: 2, mode: "catmullClark" });
geo = pg.computeNormals(geo);
return geo;
