// Wood grain — rings with knots and color variation
import type { ProcGeo } from "procgeo";
declare const pg: ProcGeo;

const size = 512;

// Ring structure: stretched Perlin creates directional grain
const rings = pg.copNoise({
  noiseType: "perlin",
  frequency: 2.0,
  octaves: 6,
  lacunarity: 2.0,
  gain: 0.5,
  amplitude: 1.0,
  offset: [0.0, 0.0],
  seed: 3,
  width: size,
  height: size,
});

// Fine grain detail: high-frequency noise for wood fiber
const grain = pg.copNoise({
  noiseType: "perlin",
  frequency: 40.0,
  octaves: 2,
  amplitude: 0.3,
  seed: 19,
  width: size,
  height: size,
});

// Knot disturbance: low-frequency Worley for organic knot shapes
const knots = pg.copNoise({
  noiseType: "worley",
  frequency: 1.5,
  octaves: 2,
  amplitude: 0.5,
  seed: 61,
  width: size,
  height: size,
});

// Warm wood color ramp
const woodColor = pg.copRamp({
  rampType: "linear",
  stops: [
    { position: 0.0, color: [0.35, 0.2, 0.1, 1.0] },
    { position: 0.25, color: [0.55, 0.35, 0.18, 1.0] },
    { position: 0.5, color: [0.65, 0.42, 0.22, 1.0] },
    { position: 0.75, color: [0.5, 0.3, 0.15, 1.0] },
    { position: 1.0, color: [0.4, 0.22, 0.1, 1.0] },
  ],
  width: size,
  height: size,
});

// Build the base: color modulated by ring pattern
let wood = pg.copComposite(woodColor, rings, {
  operation: "multiply",
  mix: 0.8,
});

// Add fine grain texture
wood = pg.copComposite(wood, grain, {
  operation: "multiply",
  mix: 0.15,
});

// Screen in knot regions for lighter, organic marks
wood = pg.copComposite(wood, knots, {
  operation: "screen",
  mix: 0.2,
});

// Gentle horizontal blur to emphasize grain direction
wood = pg.copBlur(wood, { radiusX: 2.0, radiusY: 0.5 });
return wood;
