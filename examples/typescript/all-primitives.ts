// All primitive shapes
import type { ProcGeo, Geometry } from "procgeo";
declare const pg: ProcGeo;

let box = pg.transform(pg.createBox(), { translate: [-3, 0, 0] });
box = pg.color(pg.computeNormals(box), { color: [0.9, 0.3, 0.3] });

let sphere = pg.transform(pg.createSphere({ rows: 8, cols: 16 }), {
  translate: [-1.5, 0, 0],
});
sphere = pg.color(pg.computeNormals(sphere), { color: [0.3, 0.9, 0.3] });

let torus = pg.createTorus();
torus = pg.color(pg.computeNormals(torus), { color: [0.3, 0.3, 0.9] });

let tube = pg.transform(pg.createTube({ rows: 4 }), { translate: [1.5, 0, 0] });
tube = pg.color(pg.computeNormals(tube), { color: [0.9, 0.9, 0.3] });

let circle = pg.transform(pg.createCircle({ divisions: 24 }), {
  translate: [3, 0, 0],
});
circle = pg.color(pg.computeNormals(circle), { color: [0.9, 0.3, 0.9] });

return box;
