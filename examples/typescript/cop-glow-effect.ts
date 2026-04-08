// Bloom / glow — dual-layer blur composited over source
import type { ProcGeo } from "procgeo";
declare const pg: ProcGeo;

const size = 256;

const checker = pg.copCheckerboard({
  colorA: [0.0, 0.0, 0.0, 1.0],
  colorB: [1.0, 0.7, 0.2, 1.0],
  frequency: [6.0, 6.0],
  width: size,
  height: size,
});

const noise = pg.copNoise({
  noiseType: "simplex",
  frequency: 8.0,
  octaves: 3,
  amplitude: 1.0,
  seed: 5,
  width: size,
  height: size,
});

let source = pg.copComposite(checker, noise, {
  operation: "multiply",
  mix: 0.4,
});

const bloomWide = pg.copBlur(source, { radiusX: 20, radiusY: 20 });
const bloomTight = pg.copBlur(source, { radiusX: 8, radiusY: 8 });

let bloom = pg.copComposite(bloomWide, bloomTight, {
  operation: "add",
  mix: 0.5,
});

let glowed = pg.copComposite(source, bloom, {
  operation: "add",
  mix: 0.6,
});

const vignette = pg.copRamp({
  rampType: "radial",
  stops: [
    { position: 0.0, color: [1.0, 1.0, 1.0, 1.0] },
    { position: 0.5, color: [0.9, 0.9, 0.9, 1.0] },
    { position: 1.0, color: [0.15, 0.1, 0.05, 1.0] },
  ],
  width: size,
  height: size,
});

return pg.copComposite(glowed, vignette, {
  operation: "multiply",
  mix: 1.0,
});
