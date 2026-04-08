// Terrain heightmap — layered fBm noise
// Broad hills + medium ridges + fine Worley detail
import type { ProcGeo } from "procgeo";
declare const pg: ProcGeo;

const size = 256;

const hills = pg.copNoise({
  noiseType: "simplex",
  frequency: 2.0,
  octaves: 4,
  lacunarity: 2.0,
  gain: 0.5,
  amplitude: 1.0,
  seed: 0,
  width: size,
  height: size,
});

const ridges = pg.copNoise({
  noiseType: "perlin",
  frequency: 6.0,
  octaves: 6,
  lacunarity: 2.2,
  gain: 0.45,
  amplitude: 0.4,
  seed: 42,
  width: size,
  height: size,
});

const detail = pg.copNoise({
  noiseType: "worley",
  frequency: 12.0,
  octaves: 2,
  amplitude: 0.15,
  seed: 99,
  width: size,
  height: size,
});

let terrain = pg.copComposite(hills, ridges, {
  operation: "add",
  mix: 0.6,
});
terrain = pg.copComposite(terrain, detail, {
  operation: "screen",
  mix: 0.4,
});
terrain = pg.copBlur(terrain, { radiusX: 2.0, radiusY: 2.0 });
return terrain;
