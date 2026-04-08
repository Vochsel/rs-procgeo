// Tiled mosaic — geometric tiles with grout lines and color variation
import type { ProcGeo } from "procgeo";
declare const pg: ProcGeo;

const size = 512;

// High-frequency checkerboard for tile grid
const tiles = pg.copCheckerboard({
  colorA: [0.85, 0.82, 0.75, 1.0],
  colorB: [0.65, 0.6, 0.55, 1.0],
  frequency: [16.0, 16.0],
  width: size,
  height: size,
});

// Color variation per tile region using low-freq noise
const tint = pg.copNoise({
  noiseType: "simplex",
  frequency: 4.0,
  octaves: 2,
  amplitude: 0.6,
  seed: 5,
  width: size,
  height: size,
});
const colorPalette = pg.copRamp({
  rampType: "linear",
  stops: [
    { position: 0.0, color: [0.15, 0.3, 0.55, 1.0] },
    { position: 0.25, color: [0.2, 0.5, 0.45, 1.0] },
    { position: 0.5, color: [0.55, 0.35, 0.2, 1.0] },
    { position: 0.75, color: [0.5, 0.2, 0.25, 1.0] },
    { position: 1.0, color: [0.25, 0.25, 0.5, 1.0] },
  ],
  width: size,
  height: size,
});
const tileColor = pg.copComposite(colorPalette, tint, {
  operation: "multiply",
  mix: 1.0,
});

// Combine tile structure with color
let result = pg.copComposite(tiles, tileColor, {
  operation: "multiply",
  mix: 0.8,
});

// Grout lines: fine Worley edges create the gap between tiles
const grout = pg.copNoise({
  noiseType: "worley",
  frequency: 16.0,
  octaves: 1,
  amplitude: 1.0,
  seed: 12,
  width: size,
  height: size,
});
const groutColor = pg.copConstant({
  color: [0.3, 0.28, 0.25, 1.0],
  width: size,
  height: size,
});
const groutLines = pg.copComposite(groutColor, grout, {
  operation: "screen",
  mix: 0.5,
});
result = pg.copComposite(result, groutLines, {
  operation: "multiply",
  mix: 0.6,
});

// Surface wear: subtle noise overlay
const wear = pg.copNoise({
  noiseType: "perlin",
  frequency: 12.0,
  octaves: 3,
  amplitude: 0.2,
  seed: 40,
  width: size,
  height: size,
});
result = pg.copComposite(result, wear, {
  operation: "multiply",
  mix: 0.15,
});
return result;
