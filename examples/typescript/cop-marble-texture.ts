// Marble texture — Worley veins over tinted Perlin base
import type { ProcGeo } from "procgeo";
declare const pg: ProcGeo;

const size = 256;

const base = pg.copNoise({
  noiseType: "perlin",
  frequency: 3.0,
  octaves: 4,
  amplitude: 1.0,
  seed: 7,
  width: size,
  height: size,
});

const veins = pg.copNoise({
  noiseType: "worley",
  frequency: 5.0,
  octaves: 3,
  lacunarity: 2.5,
  gain: 0.6,
  amplitude: 1.0,
  seed: 33,
  width: size,
  height: size,
});

const colorRamp = pg.copRamp({
  rampType: "diagonal",
  stops: [
    { position: 0.0, color: [0.92, 0.88, 0.82, 1.0] },
    { position: 0.35, color: [0.85, 0.78, 0.7, 1.0] },
    { position: 0.65, color: [0.7, 0.62, 0.55, 1.0] },
    { position: 1.0, color: [0.55, 0.48, 0.42, 1.0] },
  ],
  width: size,
  height: size,
});

let marble = pg.copComposite(colorRamp, base, {
  operation: "multiply",
  mix: 0.7,
});
marble = pg.copComposite(marble, veins, {
  operation: "screen",
  mix: 0.3,
});
marble = pg.copBlur(marble, { radiusX: 1.5, radiusY: 1.5 });
return marble;
