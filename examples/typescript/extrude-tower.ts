// Recursive extrude tower
import type { ProcGeo, Geometry } from "procgeo";
declare const pg: ProcGeo;

let geo = pg.createBox({ size: [1.5, 0.3, 1.5] });
geo = pg.polyExtrude(geo, { distance: 0.8, inset: 0.15 });
geo = pg.polyExtrude(geo, { distance: 0.6, inset: 0.1 });
geo = pg.polyExtrude(geo, { distance: 0.4, inset: 0.08 });
geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.7, 0.65, 0.6] });
return geo;
