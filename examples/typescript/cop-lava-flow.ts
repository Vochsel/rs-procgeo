// Lava flow — incandescent magma with cooling crust
import type { ProcGeo } from "procgeo";
declare const pg: ProcGeo;

const size = 512;

// Hot magma base: bright orange-yellow Simplex turbulence
const magma = pg.copNoise({
  noiseType: "simplex",
  frequency: 3.0,
  octaves: 6,
  lacunarity: 2.2,
  gain: 0.5,
  amplitude: 1.0,
  seed: 88,
  width: size,
  height: size,
});
const heatRamp = pg.copRamp({
  rampType: "linear",
  stops: [
    { position: 0.0, color: [1.0, 0.95, 0.4, 1.0] },
    { position: 0.3, color: [1.0, 0.6, 0.05, 1.0] },
    { position: 0.6, color: [0.9, 0.25, 0.0, 1.0] },
    { position: 1.0, color: [0.4, 0.05, 0.0, 1.0] },
  ],
  width: size,
  height: size,
});
let lava = pg.copComposite(heatRamp, magma, {
  operation: "multiply",
  mix: 1.0,
});

// Cooling crust: Worley cell boundaries form dark rock plates
const crust = pg.copNoise({
  noiseType: "worley",
  frequency: 6.0,
  octaves: 3,
  lacunarity: 2.0,
  gain: 0.5,
  amplitude: 1.0,
  seed: 14,
  width: size,
  height: size,
});
const crustColor = pg.copRamp({
  rampType: "linear",
  stops: [
    { position: 0.0, color: [0.08, 0.04, 0.02, 1.0] },
    { position: 0.5, color: [0.15, 0.08, 0.04, 1.0] },
    { position: 1.0, color: [0.25, 0.12, 0.06, 1.0] },
  ],
  width: size,
  height: size,
});
const darkCrust = pg.copComposite(crustColor, crust, {
  operation: "multiply",
  mix: 1.0,
});

// Composite: crust darkens everything, then hot cracks show through
let result = pg.copComposite(darkCrust, lava, {
  operation: "screen",
  mix: 0.8,
});

// Emissive bloom: blur and add back for glow in cracks
const glow = pg.copBlur(lava, { radiusX: 12.0, radiusY: 12.0 });
result = pg.copComposite(result, glow, {
  operation: "add",
  mix: 0.3,
});
return result;
