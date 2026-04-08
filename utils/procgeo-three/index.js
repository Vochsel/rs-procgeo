/**
 * @procgeo/three — Three.js bridge for ProcGeo geometry.
 *
 * Converts ProcGeo Geometry objects (from WASM or napi-rs bindings)
 * into Three.js BufferGeometry / Mesh objects.
 *
 * Works with any object that implements the ProcGeo Geometry interface:
 *   - getPositions(): Float32Array
 *   - getTriangleIndices(): Uint32Array
 *   - getNormals?(): Float32Array | undefined
 *   - getColors?(): Float32Array | undefined
 *   - numPoints: number
 *   - numPrims: number
 *
 * Usage:
 *   import { toBufferGeometry, toMesh, toWireframe, toPointCloud } from '@procgeo/three';
 *   import * as pg from 'procgeo-wasm';
 *
 *   const geo = pg.createBox({ size: [1, 1, 1] });
 *   const mesh = toMesh(geo);
 *   scene.add(mesh);
 */

import * as THREE from 'three';

/**
 * Convert a ProcGeo Geometry to a Three.js BufferGeometry.
 *
 * @param {object} geo - ProcGeo Geometry instance
 * @param {object} [options]
 * @param {boolean} [options.computeNormals=false] - Compute normals via Three.js if ProcGeo normals are missing
 * @returns {THREE.BufferGeometry}
 */
export function toBufferGeometry(geo, options = {}) {
  const bufGeo = new THREE.BufferGeometry();

  // Positions (required)
  const positions = geo.getPositions();
  bufGeo.setAttribute('position', new THREE.BufferAttribute(positions, 3));

  // Indices (required for indexed geometry)
  const indices = geo.getTriangleIndices();
  bufGeo.setIndex(new THREE.BufferAttribute(indices, 1));

  // Normals (optional)
  const normals = geo.getNormals?.();
  if (normals) {
    bufGeo.setAttribute('normal', new THREE.BufferAttribute(normals, 3));
  } else if (options.computeNormals) {
    bufGeo.computeVertexNormals();
  }

  // Colors (optional)
  const colors = geo.getColors?.();
  if (colors) {
    bufGeo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
  }

  bufGeo.computeBoundingSphere();
  return bufGeo;
}

/**
 * Convert a ProcGeo Geometry to a Three.js Mesh.
 *
 * @param {object} geo - ProcGeo Geometry instance
 * @param {object} [options]
 * @param {THREE.Material} [options.material] - Custom material (auto-detected if omitted)
 * @param {boolean} [options.wireframe=false] - Use wireframe material
 * @param {boolean} [options.flat=false] - Use flat shading
 * @param {number|string} [options.color=0x4488cc] - Default mesh color (ignored if vertex colors exist)
 * @returns {THREE.Mesh}
 */
export function toMesh(geo, options = {}) {
  const bufGeo = toBufferGeometry(geo, { computeNormals: true });
  const hasColors = bufGeo.getAttribute('color') !== undefined;

  const material = options.material || new THREE.MeshStandardMaterial({
    color: hasColors ? 0xffffff : (options.color ?? 0x4488cc),
    vertexColors: hasColors,
    wireframe: options.wireframe ?? false,
    flatShading: options.flat ?? false,
    side: THREE.DoubleSide,
    roughness: 0.6,
    metalness: 0.1,
  });

  return new THREE.Mesh(bufGeo, material);
}

/**
 * Convert a ProcGeo Geometry to a Three.js wireframe LineSegments
 * using the **original polygon edges**, not the triangulated mesh edges.
 *
 * Walks each primitive via primPointIndices() and emits one line segment
 * per unique edge, so quads show 4 edges (not 5 with a diagonal) and
 * n-gons show their true outline.
 *
 * @param {object} geo - ProcGeo Geometry instance (must have primPointIndices)
 * @param {object} [options]
 * @param {number|string} [options.color=0x88aaff] - Wire color
 * @param {number} [options.linewidth=1] - Line width (limited by WebGL)
 * @returns {THREE.LineSegments}
 */
export function toWireframe(geo, options = {}) {
  const positions = geo.getPositions();
  const numPrims = geo.numPrims;

  // Collect unique edges using "min-max" dedup
  const edgeSet = new Set();
  const edgePairs = [];

  for (let p = 0; p < numPrims; p++) {
    const pts = geo.primPointIndices(p);
    const n = pts.length;
    if (n < 2) continue;
    for (let i = 0; i < n; i++) {
      const a = pts[i];
      const b = pts[(i + 1) % n];
      const lo = Math.min(a, b);
      const hi = Math.max(a, b);
      const key = lo * 1000000 + hi;
      if (!edgeSet.has(key)) {
        edgeSet.add(key);
        edgePairs.push(a, b);
      }
    }
  }

  const linePositions = new Float32Array(edgePairs.length * 3);
  for (let i = 0; i < edgePairs.length; i++) {
    const ptIdx = edgePairs[i];
    linePositions[i * 3]     = positions[ptIdx * 3];
    linePositions[i * 3 + 1] = positions[ptIdx * 3 + 1];
    linePositions[i * 3 + 2] = positions[ptIdx * 3 + 2];
  }

  const lineGeo = new THREE.BufferGeometry();
  lineGeo.setAttribute('position', new THREE.BufferAttribute(linePositions, 3));

  const material = new THREE.LineBasicMaterial({
    color: options.color ?? 0x88aaff,
    linewidth: options.linewidth ?? 1,
  });

  return new THREE.LineSegments(lineGeo, material);
}

/**
 * Convert a ProcGeo Geometry to a Three.js point cloud.
 * Useful for scatter results.
 *
 * @param {object} geo - ProcGeo Geometry instance
 * @param {object} [options]
 * @param {number|string} [options.color=0xffcc44] - Point color
 * @param {number} [options.size=0.05] - Point size
 * @returns {THREE.Points}
 */
export function toPointCloud(geo, options = {}) {
  const bufGeo = new THREE.BufferGeometry();
  const positions = geo.getPositions();
  bufGeo.setAttribute('position', new THREE.BufferAttribute(positions, 3));

  const hasColors = geo.getColors?.();
  if (hasColors) {
    bufGeo.setAttribute('color', new THREE.BufferAttribute(hasColors, 3));
  }

  const material = new THREE.PointsMaterial({
    color: hasColors ? 0xffffff : (options.color ?? 0xffcc44),
    vertexColors: !!hasColors,
    size: options.size ?? 0.05,
    sizeAttenuation: true,
  });

  return new THREE.Points(bufGeo, material);
}

/**
 * Convert a ProcGeo Geometry to a Three.js edges outline.
 *
 * @param {object} geo - ProcGeo Geometry instance
 * @param {object} [options]
 * @param {number} [options.thresholdAngle=15] - Angle threshold for edge detection
 * @param {number|string} [options.color=0x222222] - Edge color
 * @returns {THREE.LineSegments}
 */
export function toEdges(geo, options = {}) {
  const bufGeo = toBufferGeometry(geo, { computeNormals: true });
  const edgesGeo = new THREE.EdgesGeometry(bufGeo, options.thresholdAngle ?? 15);

  const material = new THREE.LineBasicMaterial({
    color: options.color ?? 0x222222,
  });

  return new THREE.LineSegments(edgesGeo, material);
}

/**
 * Create a simple Three.js scene with camera, lights, and orbit controls setup.
 * Returns an object with { scene, camera, renderer, animate }.
 *
 * @param {HTMLCanvasElement|HTMLElement} container - Canvas or parent element
 * @param {object} [options]
 * @param {number|string} [options.background=0x1a1a2e] - Background color
 * @param {boolean} [options.antialias=true] - Enable antialiasing
 * @returns {{ scene: THREE.Scene, camera: THREE.PerspectiveCamera, renderer: THREE.WebGLRenderer, animate: (callback: () => void) => void }}
 */
export function createScene(container, options = {}) {
  const isCanvas = container instanceof HTMLCanvasElement;
  const canvas = isCanvas ? container : undefined;
  const parent = isCanvas ? container.parentElement : container;

  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: options.antialias ?? true,
    alpha: false,
  });
  renderer.setPixelRatio(window.devicePixelRatio);
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.2;

  if (!isCanvas) {
    container.appendChild(renderer.domElement);
  }

  const scene = new THREE.Scene();
  scene.background = new THREE.Color(options.background ?? 0x1a1a2e);

  const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 100);
  camera.position.set(3, 2, 4);
  camera.lookAt(0, 0, 0);

  // Lighting
  const ambient = new THREE.AmbientLight(0xffffff, 0.4);
  scene.add(ambient);

  const directional = new THREE.DirectionalLight(0xffffff, 1.0);
  directional.position.set(5, 8, 6);
  scene.add(directional);

  const fill = new THREE.DirectionalLight(0x8899cc, 0.3);
  fill.position.set(-3, 2, -4);
  scene.add(fill);

  // Resize handling
  function resize() {
    const w = parent?.clientWidth ?? window.innerWidth;
    const h = parent?.clientHeight ?? window.innerHeight;
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    renderer.setSize(w, h);
  }
  window.addEventListener('resize', resize);
  resize();

  // Animation loop
  function animate(callback) {
    function loop() {
      requestAnimationFrame(loop);
      if (callback) callback();
      renderer.render(scene, camera);
    }
    loop();
  }

  return { scene, camera, renderer, animate };
}
