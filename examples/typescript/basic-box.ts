// Basic box with normals
import type { ProcGeo, Geometry } from "procgeo";
declare const pg: ProcGeo;

const box = pg.createBox({ size: [1, 1, 1] });
const withNormals = pg.computeNormals(box);
return withNormals;
