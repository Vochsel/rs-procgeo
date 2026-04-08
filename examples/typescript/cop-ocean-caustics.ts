// Ocean caustics — shimmering underwater light patterns
import type { ProcGeo } from "procgeo";
declare const pg: ProcGeo;

const size = 512;

// Dual-layer Worley noise creates caustic interference
const caustics1 = pg.copNoise({
  noiseType: "worley",
  frequency: 8.0,
  octaves: 3,
  lacunarity: 2.0,
  gain: 0.6,
  amplitude: 1.0,
  seed: 7,
  width: size,
  height: size,
});
const caustics2 = pg.copNoise({
  noiseType: "worley",
  frequency: 10.0,
  octaves: 3,
  lacunarity: 2.2,
  gain: 0.55,
  amplitude: 1.0,
  seed: 31,
  width: size,
  height: size,
});

// Combine two Worley layers with min to get sharp bright edges
let pattern = pg.copComposite(caustics1, caustics2, {
  operation: "min",
  mix: 1.0,
});

// Add subtle undulation with swirl
pattern = pg.copSwirl(pattern, {
  center: [0.45, 0.55],
  angle: 25.0,
  radius: 0.8,
});

// Color map: deep blue to bright aqua highlights
const waterColor = pg.copRamp({
  rampType: "linear",
  stops: [
    { position: 0.0, color: [0.02, 0.06, 0.18, 1.0] },
    { position: 0.3, color: [0.05, 0.15, 0.35, 1.0] },
    { position: 0.6, color: [0.1, 0.35, 0.5, 1.0] },
    { position: 0.85, color: [0.3, 0.7, 0.8, 1.0] },
    { position: 1.0, color: [0.6, 0.95, 1.0, 1.0] },
  ],
  width: size,
  height: size,
});

let result = pg.copComposite(waterColor, pattern, {
  operation: "multiply",
  mix: 1.0,
});

// Soft bloom for the bright caustic lines
const bloom = pg.copBlur(result, { radiusX: 6.0, radiusY: 6.0 });
result = pg.copComposite(result, bloom, {
  operation: "add",
  mix: 0.25,
});
return result;
