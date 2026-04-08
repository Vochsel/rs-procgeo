// Group boolean operations: union, intersect, subtract
// Each sphere shows a different boolean of "upper half" vs "front half"
import type { ProcGeo, Geometry } from "procgeo";
declare const pg: ProcGeo;

function boolOp(
  operation: string,
  offset: [number, number, number],
  col: [number, number, number]
): Geometry {
  let geo = pg.createSphere({ radius: 0.8, rows: 16, cols: 32 });

  // Group A: primitives in the upper half (y > 0)
  geo = pg.groupCreate(geo, {
    name: "upper",
    groupType: "primitives",
    mode: "boundingBox",
    bboxMin: [-2, 0, -2],
    bboxMax: [2, 2, 2],
  });

  // Group B: primitives in the front half (z > 0)
  geo = pg.groupCreate(geo, {
    name: "front",
    groupType: "primitives",
    mode: "boundingBox",
    bboxMin: [-2, -2, 0],
    bboxMax: [2, 2, 2],
  });

  // Boolean combine the two groups
  geo = pg.groupCombine(geo, {
    nameA: "upper",
    nameB: "front",
    result: "result",
    operation,
    groupType: "primitives",
  });

  // Keep only the result group (negate = delete everything NOT in group)
  geo = pg.blast(geo, {
    groupName: "result",
    entity: "primitives",
    negate: true,
  });
  geo = pg.transform(geo, { translate: offset });
  geo = pg.computeNormals(geo);
  geo = pg.color(geo, { color: col });
  return geo;
}

const union = boolOp("union", [-2, 0, 0], [0.2, 0.8, 0.3]); // top OR front
const intersect = boolOp("intersect", [0, 0, 0], [0.9, 0.6, 0.1]); // top AND front
const subtract = boolOp("subtract", [2, 0, 0], [0.3, 0.5, 0.9]); // top AND NOT front

let scene = pg.merge(pg.merge(union, intersect), subtract);
return scene;
