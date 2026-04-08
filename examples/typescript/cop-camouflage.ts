// Military camo — organic blobs in earthy tones
import type { ProcGeo } from "procgeo";
declare const pg: ProcGeo;

const size = 512;

// Layer 1: base tan
const base = pg.copConstant({
  color: [0.55, 0.5, 0.35, 1.0],
  width: size,
  height: size,
});

// Layer 2: large dark green blobs
const blobsGreen = pg.copNoise({
  noiseType: "simplex",
  frequency: 2.5,
  octaves: 3,
  lacunarity: 1.8,
  gain: 0.6,
  amplitude: 1.0,
  seed: 20,
  width: size,
  height: size,
});
const green = pg.copConstant({
  color: [0.22, 0.32, 0.15, 1.0],
  width: size,
  height: size,
});
const greenMasked = pg.copComposite(green, blobsGreen, {
  operation: "multiply",
  mix: 1.0,
});
let result = pg.copComposite(base, greenMasked, {
  operation: "screen",
  mix: 0.8,
});

// Layer 3: medium brown patches
const blobsBrown = pg.copNoise({
  noiseType: "simplex",
  frequency: 3.5,
  octaves: 3,
  lacunarity: 2.0,
  gain: 0.5,
  amplitude: 1.0,
  seed: 44,
  width: size,
  height: size,
});
const brown = pg.copConstant({
  color: [0.35, 0.22, 0.1, 1.0],
  width: size,
  height: size,
});
const brownMasked = pg.copComposite(brown, blobsBrown, {
  operation: "multiply",
  mix: 1.0,
});
result = pg.copComposite(result, brownMasked, {
  operation: "screen",
  mix: 0.6,
});

// Layer 4: small dark splotches
const splotches = pg.copNoise({
  noiseType: "simplex",
  frequency: 5.0,
  octaves: 2,
  amplitude: 0.8,
  seed: 66,
  width: size,
  height: size,
});
const dark = pg.copConstant({
  color: [0.1, 0.1, 0.08, 1.0],
  width: size,
  height: size,
});
const darkMasked = pg.copComposite(dark, splotches, {
  operation: "multiply",
  mix: 1.0,
});
result = pg.copComposite(result, darkMasked, {
  operation: "screen",
  mix: 0.4,
});

// Soften edges for organic look
result = pg.copBlur(result, { radiusX: 3.0, radiusY: 3.0 });
return result;
