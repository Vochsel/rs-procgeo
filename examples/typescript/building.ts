// Multi-color building with merge
import type { ProcGeo, Geometry } from "procgeo";
declare const pg: ProcGeo;

function box(
  size: [number, number, number],
  center: [number, number, number],
  col: [number, number, number]
): Geometry {
  let g = pg.createBox({ size, center });
  g = pg.color(g, { color: col });
  return g;
}

const parts: Geometry[] = [];

// Main body
parts.push(box([4, 6, 3], [0, 3, 0], [0.88, 0.9, 0.92]));

// Window bands (teal glass)
for (let i = 0; i < 5; i++) {
  const y = 1.2 + i * 1.2;
  parts.push(box([3.8, 0.6, 0.2], [0, y, 1.55], [0.15, 0.38, 0.5]));
  parts.push(box([3.8, 0.6, 0.2], [0, y, -1.55], [0.15, 0.38, 0.5]));
  parts.push(box([0.2, 0.6, 2.8], [-2.05, y, 0], [0.15, 0.38, 0.5]));
  parts.push(box([0.2, 0.6, 2.8], [2.05, y, 0], [0.15, 0.38, 0.5]));
}

// Dark ground floor
parts.push(box([4, 0.8, 3], [0, 0.4, 0], [0.3, 0.35, 0.4]));

// Roof slab
parts.push(box([4.4, 0.2, 3.4], [0, 6.1, 0], [0.2, 0.24, 0.3]));

// Penthouse
parts.push(box([3, 1.0, 2], [0, 6.6, 0], [0.5, 0.53, 0.56]));

// Ground plane
parts.push(box([12, 0.05, 8], [0, -0.025, 0], [0.5, 0.52, 0.54]));

let scene = parts.reduce((a, b) => pg.merge(a, b));
scene = pg.computeNormals(scene);
return scene;
