// Neon grid — checkerboard + radial glow + swirl
import type { ProcGeo } from "procgeo";
declare const pg: ProcGeo;

const size = 256;

const checker = pg.copCheckerboard({
  colorA: [0.9, 0.1, 0.9, 1.0],
  colorB: [0.1, 0.9, 0.9, 1.0],
  frequency: [12.0, 12.0],
  width: size,
  height: size,
});

const glow = pg.copRamp({
  rampType: "radial",
  stops: [
    { position: 0.0, color: [1.0, 1.0, 1.0, 1.0] },
    { position: 0.6, color: [0.4, 0.2, 0.6, 1.0] },
    { position: 1.0, color: [0.02, 0.01, 0.05, 1.0] },
  ],
  width: size,
  height: size,
});

let result = pg.copComposite(checker, glow, {
  operation: "multiply",
  mix: 1.0,
});
result = pg.copSwirl(result, {
  center: [0.5, 0.5],
  angle: 120.0,
  radius: 0.6,
});
return result;
