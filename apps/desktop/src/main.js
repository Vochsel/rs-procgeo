import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { invoke } from '@tauri-apps/api/core';

// ---------------------------------------------------------------------------
// Scene
// ---------------------------------------------------------------------------
const canvas = document.getElementById('canvas');
const statusEl = document.getElementById('status');

const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
renderer.setPixelRatio(window.devicePixelRatio);

const scene = new THREE.Scene();
scene.background = new THREE.Color(0x181820);

const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 1000);
camera.position.set(3, 2.2, 3.5);

const controls = new OrbitControls(camera, canvas);
controls.enableDamping = false;

scene.add(new THREE.AmbientLight(0xffffff, 0.35));
const dirLight = new THREE.DirectionalLight(0xffffff, 1.2);
dirLight.position.set(5, 8, 6);
scene.add(dirLight);
scene.add(new THREE.GridHelper(8, 16, 0x222233, 0x1a1a28));

const material = new THREE.MeshStandardMaterial({
  color: 0x88aaff,
  roughness: 0.55,
  metalness: 0.1,
  flatShading: false,
  side: THREE.DoubleSide,
});
let mesh = null;

function resize() {
  const w = canvas.clientWidth;
  const h = canvas.clientHeight;
  renderer.setSize(w, h, false);
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
}
window.addEventListener('resize', resize);

function animate() {
  requestAnimationFrame(animate);
  controls.update();
  renderer.render(scene, camera);
}

// ---------------------------------------------------------------------------
// Native cook — procgeo runs as Rust, not WASM. We send a SOP graph and get
// back render-ready buffers computed natively.
// ---------------------------------------------------------------------------
function buffersToGeometry(buf) {
  const geo = new THREE.BufferGeometry();
  geo.setAttribute('position', new THREE.Float32BufferAttribute(buf.positions, 3));
  if (buf.indices && buf.indices.length) geo.setIndex(buf.indices);
  if (buf.colors) geo.setAttribute('color', new THREE.Float32BufferAttribute(buf.colors, 3));
  if (buf.normals) geo.setAttribute('normal', new THREE.Float32BufferAttribute(buf.normals, 3));
  else geo.computeVertexNormals();
  return geo;
}

async function cook(graph) {
  const t0 = performance.now();
  try {
    const buf = await invoke('cook', { graph });
    if (mesh) {
      scene.remove(mesh);
      mesh.geometry.dispose();
    }
    mesh = new THREE.Mesh(buffersToGeometry(buf), material);
    scene.add(mesh);
    const ms = (performance.now() - t0).toFixed(1);
    statusEl.textContent =
      `${buf.numPoints} points · ${buf.numPrims} prims · cooked natively in ${ms} ms`;
  } catch (err) {
    statusEl.textContent = `Error: ${err}`;
    console.error(err);
  }
}

// ---------------------------------------------------------------------------
// Toolbar — a few example native SOP graphs
// ---------------------------------------------------------------------------
const presets = {
  Box: { create: { name: 'box', params: { size: [1.5, 1.5, 1.5] } } },
  Sphere: { create: { name: 'sphere', params: { radius: 1.0 } } },
  'Subdivided box': {
    create: { name: 'box', params: { size: [1.5, 1.5, 1.5] } },
    modifiers: [
      { name: 'subdivide', params: { depth: 2 } },
      { name: 'normal', params: {} },
    ],
  },
  Torus: { create: { name: 'torus', params: {} } },
};

const toolbar = document.getElementById('toolbar');
for (const [label, graph] of Object.entries(presets)) {
  const btn = document.createElement('button');
  btn.textContent = label;
  btn.addEventListener('click', () => cook(graph));
  toolbar.appendChild(btn);
}

// ---------------------------------------------------------------------------
resize();
animate();
cook(presets['Subdivided box']);
