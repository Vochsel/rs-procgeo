// Extruded city blocks
import type { ProcGeo, Geometry } from "procgeo";
declare const pg: ProcGeo;

const grid = pg.createGrid({ rows: 2, cols: 2, sizeX: 8, sizeY: 8 });
const points = pg.scatter(grid, { count: 12, seed: 7 });
let building = pg.createBox({ size: [0.8, 2, 0.8] });
building = pg.polyExtrude(building, { distance: 0.3, inset: 0.15 });
let city = pg.copyToPoints(building, points);
city = pg.computeNormals(city);
city = pg.color(city, { color: [0.6, 0.65, 0.75] });

const ground = pg.createGrid({ rows: 2, cols: 2, sizeX: 10, sizeY: 10 });
const groundN = pg.computeNormals(ground);
const groundC = pg.color(groundN, { color: [0.25, 0.3, 0.2] });

return pg.fuse(city, { distance: 0.001 });
