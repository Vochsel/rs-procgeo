// Reversed normals comparison
// Create two grids — one normal, one reversed
import type { ProcGeo, Geometry } from "procgeo";
declare const pg: ProcGeo;

let grid = pg.createGrid({ rows: 8, cols: 8, sizeX: 2, sizeY: 2 });
grid = pg.computeNormals(grid);
grid = pg.color(grid, { color: [0.2, 0.7, 0.9] });

let reversed = pg.createGrid({ rows: 8, cols: 8, sizeX: 2, sizeY: 2 });
reversed = pg.transform(reversed, { translate: [3, 0, 0] });
reversed = pg.reverse(reversed);
reversed = pg.computeNormals(reversed);
reversed = pg.color(reversed, { color: [0.9, 0.3, 0.2] });

return grid;
