// Rusty metal — corroded steel with patina and pitting
import type { ProcGeo } from "procgeo";
declare const pg: ProcGeo;

const size = 512;

// Base steel: dark, mostly uniform gray
const steel = pg.copNoise({
  noiseType: "perlin",
  frequency: 1.5,
  octaves: 2,
  amplitude: 0.15,
  seed: 10,
  width: size,
  height: size,
});
const steelColor = pg.copRamp({
  rampType: "linear",
  stops: [
    { position: 0.0, color: [0.22, 0.22, 0.24, 1.0] },
    { position: 1.0, color: [0.35, 0.34, 0.33, 1.0] },
  ],
  width: size,
  height: size,
});
let base = pg.copComposite(steelColor, steel, {
  operation: "multiply",
  mix: 1.0,
});

// Rust patches: warm orange-brown Simplex noise at medium scale
const rustMask = pg.copNoise({
  noiseType: "simplex",
  frequency: 3.0,
  octaves: 5,
  lacunarity: 2.0,
  gain: 0.55,
  amplitude: 1.0,
  seed: 77,
  width: size,
  height: size,
});
const rustColor = pg.copRamp({
  rampType: "linear",
  stops: [
    { position: 0.0, color: [0.45, 0.18, 0.05, 1.0] },
    { position: 0.4, color: [0.62, 0.28, 0.08, 1.0] },
    { position: 0.7, color: [0.5, 0.22, 0.06, 1.0] },
    { position: 1.0, color: [0.35, 0.12, 0.04, 1.0] },
  ],
  width: size,
  height: size,
});
const rust = pg.copComposite(rustColor, rustMask, {
  operation: "multiply",
  mix: 1.0,
});

// Blend rust onto steel using screen for natural layering
let result = pg.copComposite(base, rust, {
  operation: "screen",
  mix: 0.7,
});

// Pitting: fine Worley craters darkening the surface
const pits = pg.copNoise({
  noiseType: "worley",
  frequency: 20.0,
  octaves: 1,
  amplitude: 0.6,
  seed: 55,
  width: size,
  height: size,
});
result = pg.copComposite(result, pits, {
  operation: "multiply",
  mix: 0.25,
});

// Subtle blur for realism
result = pg.copBlur(result, { radiusX: 0.8, radiusY: 0.8 });
return result;
