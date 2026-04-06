---
name: procgeo-ts
description: Use when writing TypeScript or JavaScript code that uses procgeo bindings (Node.js napi-rs or WASM) to create, transform, or render procedural geometry. Triggers on procgeo imports, Three.js geometry bridges, SOP function calls (createBox, scatter, subdivide), WASM init, or any Houdini-style procedural modeling in JS/TS.
---

# Procedural Geometry in TypeScript/JavaScript with ProcGeo

## Overview

ProcGeo exposes its Rust geometry engine to JavaScript via two bindings:

| Binding | Package | Runtime | Use case |
|---------|---------|---------|----------|
| **Node** (napi-rs) | `@procgeo/core` | Node.js | CLI tools, build pipelines, server-side generation |
| **WASM** | `procgeo-wasm` | Browser / Node | In-browser editors, Three.js scenes, interactive apps |

Both share the same API shape: stateless **SOP functions** that take a `Geometry` + optional params object and return a new `Geometry`. Think in Houdini networks, not imperative mesh code.

**Key difference:** Node params use **snake_case** keys (matching Rust serde). WASM params use **camelCase** keys (JS convention).

## Imports

```ts
// Node.js (napi-rs)
const pg = require('@procgeo/core');
// or: import * as pg from '@procgeo/core';

// WASM (browser)
import init, * as pg from 'procgeo-wasm';
await init(); // REQUIRED before any calls

// Three.js bridge (works with WASM Geometry)
import { toMesh, toBufferGeometry, toEdges, toPointCloud, toWireframe, createScene } from '@procgeo/three';
```

## Geometry Class

Both bindings expose the same `Geometry` class:

```ts
const geo = pg.createBox();

// Properties (readonly getters)
geo.numPoints   // number
geo.numPrims    // number
geo.numVertices // number

// Methods (both bindings)
geo.pointPos(index: number)          // [x, y, z] (Node: number[], WASM: Float32Array)
geo.boundingBox()                    // { min: [x,y,z], max: [x,y,z] }
geo.attribNames(class: string)       // string[] — class: "point"|"vertex"|"primitive"|"detail"
geo.attribType(class, name)          // string|undefined — "Float", "Int", "Vector3", etc.
geo.attribSize(class, name)          // number|undefined — component count (1=float, 3=vec3)
geo.attribData(class, name)          // number[]|undefined — flat interleaved numeric data
geo.attribDataString(class, name)    // string[]|undefined — string attribute values
geo.primPointIndices(primIndex)      // number[] — point indices for a primitive
geo.primVertexCount(primIndex)       // number
geo.vertexPoint(vertexIndex)         // number — which point a vertex maps to

// WASM-only (for WebGL/Three.js)
geo.getPositions()        // Float32Array — flat [x0,y0,z0, x1,y1,z1, ...]
geo.getTriangleIndices()  // Uint32Array — fan-triangulated index buffer
geo.getNormals()          // Float32Array|undefined — if "N" attrib exists
geo.getColors()           // Float32Array|undefined — if "Cd" attrib exists
geo.toObj()               // string — OBJ format
geo.toGlb()               // Uint8Array — GLB binary
```

## SOP Functions — Quick Reference

### Generators (create geometry from nothing)

| Function | Key Params | Output |
|----------|-----------|--------|
| `createBox(params?)` | `size: [w,h,d]`, `center: [x,y,z]` | 8 pts, 6 quads |
| `createGrid(params?)` | `rows`, `cols`, `sizeX`/`sizeY` (WASM) or `size_x`/`size_y` (Node) | Grid mesh |
| `createSphere(params?)` | `radius`, `rows`, `cols`, `center` | UV sphere |
| `createTube(params?)` | `radiusBottom`/`radiusTop` (WASM) or `radius_bottom`/`radius_top` (Node), `height`, `rows`, `cols` | Cylinder |
| `createTorus(params?)` | `radiusOuter`/`radiusInner` (WASM) or `radius_outer`/`radius_inner` (Node), `rows`, `cols` | Torus |
| `createCircle(params?)` | `radius`, `divisions`, `center` | Closed polygon |
| `createLine(params?)` | `origin`, `direction`, `length`, `points` | Open polyline |
| `createMetaball(params?)` | `balls: [{center, radius, weight}]`, `threshold`, `kernel`, `resolution` | *Node only* |

### Modifiers (geometry in, geometry out)

| Function | Key Params | Effect |
|----------|-----------|--------|
| `transform(geo, params?)` | `translate`, `rotate`, `scale`, `pivot` (all `[x,y,z]`) | Move/rotate/scale |
| `subdivide(geo, params?)` | `depth`, `mode: "linear"\|"catmullClark"` | Refine topology |
| `smooth(geo, params?)` | `iterations`, `strength` | Laplacian smoothing |
| `polyExtrude(geo, params?)` | `distance`, `inset`, `outputFront`/`outputSide` | Extrude faces |
| `clip(geo, params?)` | `origin`, `normal`, `keepAbove` (WASM) / `keep_above` (Node) | Cut with plane |
| `computeNormals(geo)` | *(none)* | Compute "N" vertex attrib |
| `color(geo, params?)` | `color: [r, g, b]` (0-1) | Set uniform "Cd" attrib |
| `scatter(geo, params?)` | `count`, `seed` | Random points on surface |
| `reverse(geo)` | *(none)* | Flip winding order |
| `fuse(geo, params?)` | `distance` | Merge coincident points |
| `polyBevel(geo, params?)` | `offset`, `divisions` | *Node only* |
| `polyWire(geo, params?)` | `radius`, `divisions` | *Node only* |
| `polyReduce(geo, params?)` | `target_percent`, `preserve_boundaries` | *Node only* |
| `polyFill(geo, params?)` | `mode: "single"\|"fan"`, `smooth` | *Node only* |
| `resample(geo, params?)` | `length`, `max_segments` | *Node only* |
| `sort(geo, params?)` | `seed` | *Node only* |
| `connectivity(geo, params?)` | `attrib_name` | *Node only* |
| `revolve(geo, params?)` | `origin`, `axis`, `divisions`, `start_angle`, `end_angle` | *Node only* |

### Multi-input

| Function | Signature | Effect |
|----------|-----------|--------|
| `merge(geometries)` | `merge([geo1, geo2, ...])` (Node) | Concatenate geometries |
| `copyToPoints(source, target)` | Both bindings | Instance source at each target point |
| `voronoiFracture(geo, points, params?)` | WASM only — `cutPlaneOffset`, `createInsideFaces` | Fracture mesh |

### Attribute SOPs

| Function | Key Params | Effect |
|----------|-----------|--------|
| `attribNoise(geo, params?)` | `attribName`, `noiseType`, `elementSize`, `amplitude`, `fractal`, `octaves` | Procedural noise |
| `attribRandomize(geo, params?)` | `attribName`, `class`, `attribType`, `distribution`, `seed` | Random values |
| `attribTransfer(dest, source, params?)` | `attribName`, `class`, `attribType`, `maxSamples` | Transfer by proximity |
| `attribCopy(dest, source?, params?)` | `attribName`, `class`, `newName` | Copy/rename attrib |
| `attribSort(geo, params?)` | `attribName`, `order: "Ascending"\|"Descending"` | Sort elements by attrib |
| `attribBlur(geo, params?)` | `attribName`, `iterations`, `stepSize` | Smooth attrib values |
| `attribFill(geo, params?)` | `attribName`, `boundaryGroup`, `iterations` | Fill missing values |
| `enumerateAttrib(geo, params?)` | `name`, `start` | Sequential index attrib (*Node only*) |
| `measure(geo, params?)` | `attrib_name` | Compute prim measurements (*Node only*) |

### I/O

```ts
// Node.js — write to file
pg.writeObj(geo, 'output.obj');
pg.writeGlb(geo, 'output.glb');

// WASM — get data in memory
const objString = geo.toObj();
const glbBytes  = geo.toGlb();  // Uint8Array
```

### SOP Registry (dynamic dispatch)

```ts
// Execute any registered SOP by name (uses snake_case param keys)
const geo = pg.executeSop("transform", inputGeo, { translate: [0, 1, 0] });
const box = pg.executeSopCreate("box", { size: [2, 2, 2] });
const names = pg.listSops(); // string[]
```

## The Procedural Mindset (JS Edition)

### 1. Chain SOPs Functionally

Every SOP is pure: geometry in, geometry out. Chain them:

```ts
let geo = pg.createBox({ size: [2, 2, 2] });
geo = pg.subdivide(geo, { depth: 2, mode: 'catmullClark' });
geo = pg.smooth(geo, { iterations: 3, strength: 0.5 });
geo = pg.transform(geo, { translate: [0, 1, 0], scale: [0.5, 0.5, 0.5] });
geo = pg.computeNormals(geo);
geo = pg.color(geo, { color: [0.2, 0.6, 1.0] });
```

For multi-input SOPs, call directly:

```ts
// Merge any number of geometries
const merged = pg.merge([boxGeo, sphereGeo, gridGeo]);

// Copy source onto each point of target
const instances = pg.copyToPoints(sourceGeo, targetPoints);
```

### 2. Build Reusable Pipelines

```ts
function roundedBox(size, smoothing = 3) {
  let geo = pg.createBox({ size });
  geo = pg.subdivide(geo, { depth: 2, mode: 'catmullClark' });
  geo = pg.smooth(geo, { iterations: smoothing, strength: 0.5 });
  return pg.computeNormals(geo);
}

function scatterInstances(shape, surface, count, seed = 0) {
  const points = pg.scatter(surface, { count, seed });
  return pg.copyToPoints(shape, points);
}
```

### 3. Use Attributes for Data

```ts
// Inspect attributes on geometry
const names = geo.attribNames('point');     // ["P", "N", "Cd", ...]
const type  = geo.attribType('point', 'N'); // "Vector3"
const data  = geo.attribData('point', 'Cd');// flat Float64Array [r0,g0,b0, r1,g1,b1, ...]

// Create/modify with SOPs
geo = pg.attribNoise(geo, {
  attribName: 'height',
  noiseType: 'simplex',
  elementSize: 2.0,
  amplitude: 0.5,
  fractal: 'standard',
  octaves: 4,
});
```

## Three.js Bridge (`@procgeo/three`)

Works with WASM `Geometry` objects. Converts to Three.js primitives:

```ts
import init, * as pg from 'procgeo-wasm';
import { toMesh, toEdges, toPointCloud, toWireframe, toBufferGeometry, createScene } from '@procgeo/three';

await init();

// Convert to Three.js objects
const mesh      = toMesh(geo, { color: 0x4488cc, flat: false, wireframe: false });
const edges     = toEdges(geo, { thresholdAngle: 20, color: 0x223355 });
const wireframe = toWireframe(geo, { color: 0x88aaff });
const points    = toPointCloud(geo, { color: 0xffcc44, size: 0.05 });

// Or get raw BufferGeometry for custom materials
const bufGeo = toBufferGeometry(geo, { computeNormals: true });

// Quick scene setup
const { scene, camera, renderer, animate } = createScene(container, {
  background: 0x1a1a2e,
  antialias: true,
});
scene.add(mesh);
animate();
```

**Vertex colors:** If the geometry has a "Cd" attribute, `toMesh` auto-enables `vertexColors` and sets base color to white.

**Normals:** If no "N" attribute exists, pass `computeNormals: true` to let Three.js compute them.

## Common Recipes

**Interactive shape editor (WASM + Three.js):**
```ts
await init();

function rebuild(shape, subdivDepth, smoothIter) {
  let geo;
  switch (shape) {
    case 'box':    geo = pg.createBox(); break;
    case 'sphere': geo = pg.createSphere({ rows: 8, cols: 16 }); break;
    case 'torus':  geo = pg.createTorus(); break;
  }
  if (subdivDepth > 0) geo = pg.subdivide(geo, { depth: subdivDepth, mode: 'catmullClark' });
  if (smoothIter > 0)  geo = pg.smooth(geo, { iterations: smoothIter, strength: 0.5 });
  geo = pg.computeNormals(geo);
  return geo;
}
```

**Procedural city (Node.js):**
```js
const ground = pg.createGrid({ rows: 2, cols: 2, size_x: 20, size_y: 20 });
const positions = pg.scatter(ground, { count: 15, seed: 123 });
const building = pg.polyExtrude(pg.createBox({ size: [1, 3, 1] }), { distance: 0.2, inset: 0.15 });
const city = pg.copyToPoints(building, positions);
const scene = pg.merge([
  pg.color(pg.computeNormals(city), { color: [0.7, 0.7, 0.8] }),
  pg.color(ground, { color: [0.3, 0.5, 0.2] }),
]);
pg.writeGlb(scene, 'city.glb');
```

**Layered noise terrain (WASM):**
```ts
let terrain = pg.createGrid({ rows: 50, cols: 50, sizeX: 10, sizeY: 10 });
terrain = pg.attribNoise(terrain, {
  attribName: 'height',
  noiseType: 'simplex',
  elementSize: 3.0,
  amplitude: 1.0,
  fractal: 'terrain',
  octaves: 6,
});
terrain = pg.attribNoise(terrain, {
  attribName: 'height',
  noiseType: 'worley',
  elementSize: 1.5,
  amplitude: 0.3,
  operation: 'add',
});
terrain = pg.computeNormals(terrain);
```

**Export from browser:**
```ts
// OBJ download
const blob = new Blob([geo.toObj()], { type: 'text/plain' });
const a = document.createElement('a');
a.href = URL.createObjectURL(blob);
a.download = 'model.obj';
a.click();

// GLB download
const glbBlob = new Blob([geo.toGlb()], { type: 'model/gltf-binary' });
```

## Param Key Casing Reference

| Concept | Node (snake_case) | WASM (camelCase) |
|---------|-------------------|------------------|
| Grid size | `size_x`, `size_y` | `sizeX`, `sizeY` |
| Tube radii | `radius_bottom`, `radius_top` | `radiusBottom`, `radiusTop` |
| Torus radii | `radius_outer`, `radius_inner` | `radiusOuter`, `radiusInner` |
| Clip keep | `keep_above` | `keepAbove` |
| Extrude output | `output_front`, `output_side` | `outputFront`, `outputSide` |
| Attrib params | `attrib_name`, `attrib_type` | `attribName`, `attribType` |
| Noise params | `element_size`, `min_value`, `max_value` | `elementSize`, `minValue`, `maxValue` |
| Randomize | `min_value`, `max_value`, `global_scale` | `minValue`, `maxValue`, `globalScale` |
| Transfer | `max_samples`, `distance_threshold` | `maxSamples`, `distanceThreshold` |
| Blur | `step_size` | `stepSize` |
| Fill | `boundary_group`, `step_size` | `boundaryGroup`, `stepSize` |

**Registry (`executeSop`/`executeSopCreate`) always uses snake_case** (both bindings) since it serializes to Rust serde JSON.

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Calling WASM functions before `await init()` | Always `await init()` first in browser |
| Using camelCase params with Node binding | Node uses snake_case: `size_x` not `sizeX` |
| Using snake_case params with WASM binding | WASM uses camelCase: `sizeX` not `size_x` |
| Forgetting `computeNormals()` before render | Always call before Three.js display |
| Using `merge()` in WASM | WASM has no merge — use Node or merge Three.js meshes |
| Passing `params` to `computeNormals` | It takes no params, just `computeNormals(geo)` |
| Expecting `toMesh()` to work with Node geo | Three.js bridge needs WASM `Geometry` (with `getPositions()`) |
| Not freeing WASM Geometry | Call `geo.free()` or use `using geo = ...` (Symbol.dispose) |
| Using `executeSop` with camelCase params | Registry always uses snake_case, even in WASM |
