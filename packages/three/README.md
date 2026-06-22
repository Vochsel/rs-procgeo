# @procgeo/three

Three.js bridge for [ProcGeo](https://www.npmjs.com/package/@procgeo/lib) procedural geometry.

Converts ProcGeo Geometry objects into Three.js `BufferGeometry`, `Mesh`, wireframe `LineSegments`, and `Points`.

## Install

```bash
npm install @procgeo/three @procgeo/lib three
```

## Usage

```js
import init, { createTorus, subdivide, computeNormals } from '@procgeo/lib';
import { toMesh, toWireframe, toPointCloud, createScene } from '@procgeo/three';

await init();

const geo = computeNormals(subdivide(createTorus(), { depth: 2, mode: 'catmullClark' }));

const { scene, animate } = createScene(document.getElementById('canvas'));
scene.add(toMesh(geo));
scene.add(toWireframe(geo, { color: 0x88aaff }));
animate();
```

## API

| Function | Returns | Description |
|---|---|---|
| `toBufferGeometry(geo, opts?)` | `THREE.BufferGeometry` | Raw buffer geometry with positions, indices, normals, colors |
| `toMesh(geo, opts?)` | `THREE.Mesh` | Mesh with auto-detected vertex colors and configurable material |
| `toWireframe(geo, opts?)` | `THREE.LineSegments` | True polygon-edge wireframe (not triangulated diagonals) |
| `toPointCloud(geo, opts?)` | `THREE.Points` | Point cloud with optional vertex colors |
| `toEdges(geo, opts?)` | `THREE.LineSegments` | Edge outline using Three.js EdgesGeometry |
| `createScene(container, opts?)` | `SceneResult` | Quick scene setup with camera, lights, and animation loop |
