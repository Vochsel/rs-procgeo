import * as monaco from 'monaco-editor';
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { examples } from './examples.js';
import procgeoTypes from './procgeo-editor-types.d.ts?raw';

// ── WASM Loading ─────────────────────────────────────────
// We load procgeo from the pre-built pkg/ directory
// In production this would come from @vochsel/procgeo-js npm package
let pg = null;
let wasmReady = false;

async function loadWasm() {
    const wasmModule = await import('procgeo-wasm');
    await wasmModule.default();
    pg = wasmModule;
    wasmReady = true;
    setStatus('Ready', 'success');
    // Initialize GPU context for COP image processing in background (optional)
    if (navigator.gpu) {
        wasmModule.initCopGpu()
            .then(() => console.log('COP GPU ready'))
            .catch(e => console.warn('COP GPU init skipped:', e.message));
    }
}

// ── Three.js Scene ───────────────────────────────────────
const canvas = document.getElementById('canvas');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
renderer.setPixelRatio(window.devicePixelRatio);
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 1.1;

const scene = new THREE.Scene();
scene.background = new THREE.Color(0x181820);

const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 1000);
camera.position.set(3, 2.2, 3.5);

const controls = new OrbitControls(camera, canvas);
controls.enableDamping = false;

// Lights
scene.add(new THREE.AmbientLight(0xffffff, 0.35));
const dirLight = new THREE.DirectionalLight(0xffffff, 1.2);
dirLight.position.set(5, 8, 6);
scene.add(dirLight);
scene.add(new THREE.DirectionalLight(0x6688bb, 0.4).translateX(-4).translateY(3).translateZ(-5));

// Grid
scene.add(new THREE.GridHelper(8, 16, 0x222233, 0x1a1a28));

// Geometry group
const meshGroup = new THREE.Group();
scene.add(meshGroup);

let currentGeo = null;
let currentMode = 'geo'; // 'geo' or 'cop'
let viewMode = 'shaded_wire'; // 'shaded' | 'shaded_wire' | 'wire'

// ── COP Image Rendering ─────────────────────────────────
const copCanvas = document.getElementById('cop-canvas');
const copCtx2d = copCanvas.getContext('2d');

async function showCopImage(copImage) {
    currentMode = 'cop';
    currentGeo = null;

    const w = copImage.width;
    const h = copImage.height;
    const pixels = await copImage.getPixels(); // Float32Array RGBA

    // Convert RGBA f32 → RGBA u8
    const rgba8 = new Uint8ClampedArray(w * h * 4);
    for (let i = 0; i < w * h * 4; i++) {
        rgba8[i] = Math.round(Math.min(1, Math.max(0, pixels[i])) * 255);
    }

    copCanvas.width = w;
    copCanvas.height = h;
    const imageData = new ImageData(rgba8, w, h);
    copCtx2d.putImageData(imageData, 0, 0);

    // Show COP canvas, hide 3D
    copCanvas.style.display = 'block';
    canvas.style.display = 'none';

    setStatus(`${w}x${h} image`, 'success');
}

function showGeometry() {
    if (currentMode !== 'geo') {
        currentMode = 'geo';
        copCanvas.style.display = 'none';
        canvas.style.display = 'block';
        resizeViewer();
    }
}

function resizeViewer() {
    const view = document.getElementById('viewport-view');
    const w = view.clientWidth;
    const h = view.clientHeight;
    if (w > 0 && h > 0) {
        camera.aspect = w / h;
        camera.updateProjectionMatrix();
        renderer.setSize(w, h);
    }
}

/**
 * Build a LineSegments wireframe from the original polygon edges,
 * not from the triangulated Three.js mesh.
 * Walks each primitive via primPointIndices() and emits edge pairs.
 */
function buildTrueWireframe(geo, color = 0x88aaff) {
    const positions = geo.getPositions(); // flat [x0,y0,z0, x1,y1,z1, ...]
    const numPrims = geo.numPrims;

    // Collect unique edges as "minIdx-maxIdx" to avoid duplicates
    const edgeSet = new Set();
    const edgePairs = [];

    for (let p = 0; p < numPrims; p++) {
        const pts = geo.primPointIndices(p);
        const n = pts.length;
        if (n < 2) continue;
        const isClosed = geo.primIsClosed(p);
        const edgeCount = isClosed ? n : n - 1;

        for (let i = 0; i < edgeCount; i++) {
            const a = pts[i];
            const b = pts[(i + 1) % n];
            const lo = Math.min(a, b);
            const hi = Math.max(a, b);
            const key = lo * 1000000 + hi; // fast numeric key
            if (!edgeSet.has(key)) {
                edgeSet.add(key);
                edgePairs.push(a, b);
            }
        }
    }

    // Build line geometry from edge pairs
    const linePositions = new Float32Array(edgePairs.length * 3);
    for (let i = 0; i < edgePairs.length; i++) {
        const ptIdx = edgePairs[i];
        linePositions[i * 3]     = positions[ptIdx * 3];
        linePositions[i * 3 + 1] = positions[ptIdx * 3 + 1];
        linePositions[i * 3 + 2] = positions[ptIdx * 3 + 2];
    }

    const lineGeo = new THREE.BufferGeometry();
    lineGeo.setAttribute('position', new THREE.BufferAttribute(linePositions, 3));

    const material = new THREE.LineBasicMaterial({ color });
    return new THREE.LineSegments(lineGeo, material);
}

function toBufferGeometry(geo) {
    const bufGeo = new THREE.BufferGeometry();
    bufGeo.setAttribute('position', new THREE.BufferAttribute(geo.getPositions(), 3));
    bufGeo.setIndex(new THREE.BufferAttribute(geo.getTriangleIndices(), 1));
    const normals = geo.getNormals?.();
    if (normals) bufGeo.setAttribute('normal', new THREE.BufferAttribute(normals, 3));
    else bufGeo.computeVertexNormals();
    const colors = geo.getColors?.();
    if (colors) bufGeo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
    bufGeo.computeBoundingSphere();
    return bufGeo;
}

function clearMeshGroup() {
    while (meshGroup.children.length) {
        const c = meshGroup.children[0];
        // Shared animation buffers are owned by `anim` and reused across frames.
        if (!c.userData?.shared) c.geometry?.dispose();
        c.material?.dispose();
        meshGroup.remove(c);
    }
}

function updateScene(geo) {
    currentGeo = geo;
    rebuildView();
    setStatus(`${geo.numPoints} pts | ${geo.numPrims} prims`, 'success');
    updateSpreadsheet();
}

function rebuildView() {
    if (isAnimating && anim) {
        rebuildAnimView();
        return;
    }

    const geo = currentGeo;
    if (!geo) return;

    clearMeshGroup();

    const showShaded = viewMode === 'shaded' || viewMode === 'shaded_wire';
    const showWire = viewMode === 'wire' || viewMode === 'shaded_wire';

    if (showShaded) {
        const bufGeo = toBufferGeometry(geo);
        const hasColors = !!bufGeo.getAttribute('color');
        const mesh = new THREE.Mesh(bufGeo, new THREE.MeshStandardMaterial({
            color: hasColors ? 0xffffff : 0x4488cc,
            vertexColors: hasColors,
            side: THREE.DoubleSide,
            roughness: 0.55,
            metalness: 0.15,
        }));
        meshGroup.add(mesh);
    }

    if (showWire) {
        // Use true polygon wireframe, not triangulated
        const wireColor = viewMode === 'wire' ? 0x88aaff : 0x223355;
        meshGroup.add(buildTrueWireframe(geo, wireColor));
    }
}

// ── Animation playbar + frame cache ──────────────────────
// When executed code returns a SoftBodySolver, the viewer steps the solver
// once per frame, caches the per-frame point positions, and lets the playbar
// scrub through the cache. Topology is constant, so only positions are stored.

const DEFAULT_CACHE_FRAMES = 150;
const DEFAULT_PLAYBACK_FPS = 24;

let isAnimating = false;
let anim = null;        // { cache, frameCount, fps, meshGeo, wireGeo, edgeFlat, triIndices, colors }
let currentFrame = 0;
let isPlaying = false;
let loopEnabled = true;
let playAccum = 0;
let lastClock = 0;

const playbarEl = document.getElementById('playbar');
const playBtn = document.getElementById('play-btn');
const stopBtn = document.getElementById('stop-btn');
const frameSlider = document.getElementById('frame-slider');
const frameLabel = document.getElementById('frame-label');
const loopBtn = document.getElementById('loop-btn');
const cacheStatus = document.getElementById('cache-status');

/** Collect unique polygon edges as a flat array of point-index pairs. */
function computeEdgeFlat(geo) {
    const numPrims = geo.numPrims;
    const edgeSet = new Set();
    const flat = [];
    for (let p = 0; p < numPrims; p++) {
        const pts = geo.primPointIndices(p);
        const n = pts.length;
        if (n < 2) continue;
        const isClosed = geo.primIsClosed(p);
        const edgeCount = isClosed ? n : n - 1;
        for (let i = 0; i < edgeCount; i++) {
            const a = pts[i];
            const b = pts[(i + 1) % n];
            const lo = Math.min(a, b);
            const hi = Math.max(a, b);
            const key = lo * 1000000 + hi;
            if (!edgeSet.has(key)) {
                edgeSet.add(key);
                flat.push(a, b);
            }
        }
    }
    return new Uint32Array(flat);
}

function setPlaying(playing) {
    isPlaying = playing && isAnimating;
    playBtn.textContent = isPlaying ? '❚❚' : '▶';
    lastClock = performance.now();
    playAccum = 0;
}

function exitAnimationMode() {
    setPlaying(false);
    isAnimating = false;
    anim = null;
    playbarEl.classList.add('hidden');
}

/**
 * Enter animation mode for a SoftBodySolver-like object. Builds the full
 * position cache up-front, sets up reusable Three.js buffers, and shows the
 * playbar.
 */
function enterAnimationMode(solver) {
    showGeometry();
    setPlaying(false);

    const frameCount = (typeof solver.frames === 'number' && solver.frames > 1)
        ? Math.floor(solver.frames) : DEFAULT_CACHE_FRAMES;
    const fps = (typeof solver.fps === 'number' && solver.fps > 0)
        ? solver.fps : DEFAULT_PLAYBACK_FPS;

    // Snapshot topology/attributes from the rest state (frame 0).
    const baseGeo = solver.geometry();
    const triIndices = baseGeo.getTriangleIndices();
    const colors = baseGeo.getColors?.() ?? null;
    const edgeFlat = computeEdgeFlat(baseGeo);

    // Build the position cache by stepping the solver frame by frame.
    const t0 = performance.now();
    solver.reset();
    const cache = new Array(frameCount);
    cache[0] = solver.getPositions();
    for (let i = 1; i < frameCount; i++) {
        solver.step();
        cache[i] = solver.getPositions();
    }
    const bakeMs = (performance.now() - t0).toFixed(0);

    // Reusable Three.js buffers (positions swapped per frame).
    const meshGeo = new THREE.BufferGeometry();
    meshGeo.setAttribute('position', new THREE.BufferAttribute(cache[0].slice(), 3));
    meshGeo.setIndex(new THREE.BufferAttribute(triIndices, 1));
    if (colors) meshGeo.setAttribute('color', new THREE.BufferAttribute(colors, 3));

    const wireGeo = new THREE.BufferGeometry();
    wireGeo.setAttribute('position', new THREE.BufferAttribute(new Float32Array(edgeFlat.length * 3), 3));

    anim = { cache, frameCount, fps, meshGeo, wireGeo, edgeFlat, hasColors: !!colors };
    isAnimating = true;

    // Keep a frame-0 geometry around for the spreadsheet tab.
    currentGeo = baseGeo;

    playbarEl.classList.remove('hidden');
    frameSlider.max = String(frameCount - 1);
    frameSlider.value = '0';
    cacheStatus.textContent = `${frameCount}f cached (${bakeMs}ms)`;
    loopBtn.classList.toggle('active', loopEnabled);

    rebuildAnimView();
    gotoFrame(0);
    setStatus(`Softbody: ${baseGeo.numPoints} pts | ${frameCount} frames`, 'success');
    updateSpreadsheet();
}

/** (Re)build the meshGroup children from the animation buffers for the current view mode. */
function rebuildAnimView() {
    if (!anim) return;
    clearMeshGroup();

    const showShaded = viewMode === 'shaded' || viewMode === 'shaded_wire';
    const showWire = viewMode === 'wire' || viewMode === 'shaded_wire';

    if (showShaded) {
        const mesh = new THREE.Mesh(anim.meshGeo, new THREE.MeshStandardMaterial({
            color: anim.hasColors ? 0xffffff : 0x4488cc,
            vertexColors: anim.hasColors,
            side: THREE.DoubleSide,
            roughness: 0.55,
            metalness: 0.15,
        }));
        mesh.userData.shared = true; // don't dispose shared geometry in clearMeshGroup
        meshGroup.add(mesh);
    }

    if (showWire) {
        const wireColor = viewMode === 'wire' ? 0x88aaff : 0x223355;
        const wire = new THREE.LineSegments(anim.wireGeo, new THREE.LineBasicMaterial({ color: wireColor }));
        wire.userData.shared = true;
        meshGroup.add(wire);
    }
}

/** Display a cached frame: swap buffer positions and refresh derived data. */
function gotoFrame(f) {
    if (!anim) return;
    f = Math.max(0, Math.min(anim.frameCount - 1, f | 0));
    currentFrame = f;
    const pos = anim.cache[f];

    // Mesh positions + normals.
    anim.meshGeo.attributes.position.array.set(pos);
    anim.meshGeo.attributes.position.needsUpdate = true;
    anim.meshGeo.computeVertexNormals();
    anim.meshGeo.computeBoundingSphere();

    // Wireframe positions (expand per edge endpoint).
    const wp = anim.wireGeo.attributes.position.array;
    const ef = anim.edgeFlat;
    for (let i = 0; i < ef.length; i++) {
        const p = ef[i];
        wp[i * 3] = pos[p * 3];
        wp[i * 3 + 1] = pos[p * 3 + 1];
        wp[i * 3 + 2] = pos[p * 3 + 2];
    }
    anim.wireGeo.attributes.position.needsUpdate = true;

    frameSlider.value = String(f);
    frameLabel.textContent = `${f} / ${anim.frameCount - 1}`;
}

function fitCameraToScene() {
    const box = new THREE.Box3().setFromObject(meshGroup);
    if (box.isEmpty()) return;
    const center = box.getCenter(new THREE.Vector3());
    const size = box.getSize(new THREE.Vector3());
    const maxDim = Math.max(size.x, size.y, size.z);
    const dist = maxDim / (2 * Math.tan((camera.fov * Math.PI) / 360)) * 1.4;
    camera.position.copy(center).add(new THREE.Vector3(dist * 0.6, dist * 0.5, dist * 0.7));
    controls.target.copy(center);
    camera.near = dist * 0.01;
    camera.far = dist * 20;
    camera.updateProjectionMatrix();
    controls.update();
}

// Render loop
function render(t) {
    requestAnimationFrame(render);

    if (isAnimating && isPlaying && anim) {
        const now = t ?? performance.now();
        playAccum += (now - lastClock) / 1000;
        lastClock = now;
        const frameDur = 1 / anim.fps;
        let advanced = false;
        while (playAccum >= frameDur) {
            playAccum -= frameDur;
            let nf = currentFrame + 1;
            if (nf >= anim.frameCount) {
                if (loopEnabled) {
                    nf = 0;
                } else {
                    nf = anim.frameCount - 1;
                    setPlaying(false);
                    break;
                }
            }
            currentFrame = nf;
            advanced = true;
        }
        if (advanced) gotoFrame(currentFrame);
    } else {
        lastClock = t ?? performance.now();
    }

    controls.update();
    renderer.render(scene, camera);
}

// ── Status ───────────────────────────────────────────────
function setStatus(text, type = '') {
    const el = document.getElementById('status');
    el.textContent = text;
    el.className = type;
}

// ── Code Execution ───────────────────────────────────────
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;

async function executeCode(code) {
    if (!wasmReady) {
        setStatus('WASM not loaded yet...', 'error');
        return;
    }

    try {
        const t0 = performance.now();

        // Transpile TypeScript → JavaScript via Monaco's built-in TS compiler
        const uri = editor.getModel().uri;
        const worker = await monaco.languages.typescript.getTypeScriptWorker();
        const client = await worker(uri);
        const output = await client.getEmitOutput(uri.toString());
        const js = output.outputFiles[0].text;

        const fn = new AsyncFunction('pg', `"use strict"; ${js}`);
        const result = await fn(pg);
        const elapsed = (performance.now() - t0).toFixed(1);

        if (result && typeof result.step === 'function' && typeof result.geometry === 'function') {
            // A SoftBodySolver (or compatible) → animation with cached playback.
            showGeometry();
            enterAnimationMode(result);
            const el = document.getElementById('status');
            el.textContent += ` | ${elapsed}ms`;
        } else if (result && typeof result.getPositions === 'function') {
            exitAnimationMode();
            showGeometry();
            updateScene(result);
            const el = document.getElementById('status');
            el.textContent += ` | ${elapsed}ms`;
        } else if (result && typeof result.getPixels === 'function') {
            exitAnimationMode();
            await showCopImage(result);
            const el = document.getElementById('status');
            el.textContent += ` | ${elapsed}ms`;
        } else {
            setStatus('Code must return a Geometry or CopImage', 'error');
        }
    } catch (e) {
        setStatus(`Error: ${e.message}`, 'error');
        console.error(e);
    }
}

// ── Monaco Editor ────────────────────────────────────────
// Configure Monaco workers for Vite ESM
import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker';

self.MonacoEnvironment = {
    getWorker: function (_moduleId, label) {
        if (label === 'typescript' || label === 'javascript') {
            return new tsWorker();
        }
        return new editorWorker();
    }
};

// Create a TypeScript model so the TS worker can transpile it
const defaultExampleKey = 'starterScene';
const editorModel = monaco.editor.createModel(
    examples[defaultExampleKey],
    'typescript',
    monaco.Uri.parse('file:///main.ts')
);

// Create editor
const editor = monaco.editor.create(document.getElementById('editor'), {
    model: editorModel,
    theme: 'vs-dark',
    fontSize: 14,
    lineNumbers: 'on',
    minimap: { enabled: false },
    automaticLayout: true,
    scrollBeyondLastLine: false,
    padding: { top: 12 },
    suggestOnTriggerCharacters: true,
    quickSuggestions: true,
    wordBasedSuggestions: 'off',
    tabSize: 2,
});

// Suppress diagnostics — user code runs inside new Function() so top-level return is valid
monaco.languages.typescript.typescriptDefaults.setDiagnosticsOptions({
    noSemanticValidation: true,
    noSyntaxValidation: true,
});

monaco.languages.typescript.typescriptDefaults.setCompilerOptions({
    target: monaco.languages.typescript.ScriptTarget.ESNext,
    allowNonTsExtensions: true,
    module: monaco.languages.typescript.ModuleKind.None,
});

monaco.languages.typescript.typescriptDefaults.addExtraLib(procgeoTypes, 'file:///procgeo.d.ts');

// ── Debounced auto-compile ───────────────────────────────
let debounceTimer = null;

function scheduleRun() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
        executeCode(editor.getValue());
    }, 1000);
}

editor.onDidChangeModelContent(scheduleRun);

// Compile immediately when the editor loses focus
editor.onDidBlurEditorText(() => {
    clearTimeout(debounceTimer);
    executeCode(editor.getValue());
});

// Ctrl+Enter to run immediately
editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
    clearTimeout(debounceTimer);
    executeCode(editor.getValue());
});

// ── Examples dropdown ────────────────────────────────────
document.getElementById('examples').addEventListener('change', async (e) => {
    const key = e.target.value;
    if (key && examples[key]) {
        editor.setValue(examples[key]);
        await executeCode(examples[key]);
        fitCameraToScene();
    }
    e.target.value = '';
});

// ── Run button ───────────────────────────────────────────
document.getElementById('run-btn').addEventListener('click', () => {
    clearTimeout(debounceTimer);
    executeCode(editor.getValue());
});

// ── URL Sharing (encode/decode code in query param) ──
function toBase64Url(bytes) {
    let binary = '';
    for (const b of bytes) binary += String.fromCharCode(b);
    return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function fromBase64Url(str) {
    str = str.replace(/-/g, '+').replace(/_/g, '/');
    while (str.length % 4) str += '=';
    const binary = atob(str);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    return bytes;
}

async function compressCode(code) {
    const stream = new Blob([new TextEncoder().encode(code)])
        .stream()
        .pipeThrough(new CompressionStream('deflate-raw'));
    const buf = await new Response(stream).arrayBuffer();
    return toBase64Url(new Uint8Array(buf));
}

async function decompressCode(encoded) {
    const stream = new Blob([fromBase64Url(encoded)])
        .stream()
        .pipeThrough(new DecompressionStream('deflate-raw'));
    const buf = await new Response(stream).arrayBuffer();
    return new TextDecoder().decode(buf);
}

function showToast(message) {
    const el = document.getElementById('toast');
    el.textContent = message;
    el.classList.add('show');
    setTimeout(() => el.classList.remove('show'), 2000);
}

async function getCodeFromUrl() {
    const params = new URLSearchParams(window.location.search);
    const encoded = params.get('code');
    if (!encoded) return null;
    try {
        return await decompressCode(encoded);
    } catch (e) {
        console.warn('Failed to decode code from URL:', e);
        return null;
    }
}

document.getElementById('share-btn').addEventListener('click', async () => {
    const code = editor.getValue();
    try {
        const encoded = await compressCode(code);
        const url = new URL(window.location.href);
        url.search = '?code=' + encoded;
        await navigator.clipboard.writeText(url.toString());
        window.history.replaceState(null, '', url);
        showToast('Link copied to clipboard');
    } catch (e) {
        console.error('Share failed:', e);
        showToast('Failed to copy link');
    }
});

// ── Export buttons ────────────────────────────────────────
document.getElementById('export-obj').addEventListener('click', () => {
    if (!currentGeo) return;
    const blob = new Blob([currentGeo.toObj()], { type: 'text/plain' });
    const a = document.createElement('a'); a.href = URL.createObjectURL(blob);
    a.download = 'procgeo.obj'; a.click();
});

document.getElementById('export-glb').addEventListener('click', () => {
    if (!currentGeo) return;
    const blob = new Blob([currentGeo.toGlb()], { type: 'model/gltf-binary' });
    const a = document.createElement('a'); a.href = URL.createObjectURL(blob);
    a.download = 'procgeo.glb'; a.click();
});

// ── View Mode Toggle ─────────────────────────────────────
document.getElementById('view-modes').addEventListener('click', (e) => {
    const btn = e.target.closest('.view-mode-btn');
    if (!btn) return;
    const mode = btn.dataset.mode;
    if (mode === viewMode) return;
    viewMode = mode;
    document.querySelectorAll('.view-mode-btn').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    rebuildView();
});

// ── Animation Playbar controls ───────────────────────────
playBtn.addEventListener('click', () => setPlaying(!isPlaying));
stopBtn.addEventListener('click', () => {
    setPlaying(false);
    gotoFrame(0);
});
frameSlider.addEventListener('input', () => {
    setPlaying(false);
    gotoFrame(parseInt(frameSlider.value, 10));
});
loopBtn.addEventListener('click', () => {
    loopEnabled = !loopEnabled;
    loopBtn.classList.toggle('active', loopEnabled);
});
// Spacebar toggles playback when an animation is loaded.
window.addEventListener('keydown', (e) => {
    if (e.code === 'Space' && isAnimating && !(e.target instanceof HTMLInputElement)) {
        // Avoid hijacking the Monaco editor.
        const inEditor = document.getElementById('editor')?.contains(document.activeElement);
        if (!inEditor) {
            e.preventDefault();
            setPlaying(!isPlaying);
        }
    }
});

// ── Geometry Spreadsheet ─────────────────────────────────
let spreadsheetClass = 'point';

function updateSpreadsheet() {
    const geo = currentGeo;
    const thead = document.getElementById('spreadsheet-thead-row');
    const tbody = document.getElementById('spreadsheet-tbody');
    const info = document.getElementById('spreadsheet-info');

    thead.innerHTML = '';
    tbody.innerHTML = '';

    if (!geo) { info.textContent = 'No geometry'; return; }
    if (typeof geo.attribNames !== 'function') {
        info.textContent = 'Rebuild WASM to enable spreadsheet (pnpm build:wasm)';
        return;
    }

    const cls = spreadsheetClass;
    const names = geo.attribNames(cls);
    const rowCount = cls === 'point' ? geo.numPoints
        : cls === 'vertex' ? geo.numVertices
        : cls === 'primitive' ? geo.numPrims
        : 1; // detail

    info.textContent = `${rowCount} ${cls}${rowCount !== 1 ? 's' : ''} | ${names.length} attrib${names.length !== 1 ? 's' : ''}`;

    // Build column definitions: index + each attrib (expanded per component)
    const cols = [{ name: '#', size: 1 }];
    const attribs = [];
    for (const name of names) {
        const size = geo.attribSize(cls, name) ?? 1;
        const type = geo.attribType(cls, name) ?? 'Unknown';
        attribs.push({ name, size, type });
        if (size === 1) {
            cols.push({ name, size: 1 });
        } else {
            const suffixes = size === 2 ? ['x', 'y']
                : size === 3 ? ['x', 'y', 'z']
                : size === 4 ? ['x', 'y', 'z', 'w']
                : Array.from({ length: size }, (_, i) => String(i));
            for (const s of suffixes) {
                cols.push({ name: `${name}.${s}`, size: 1 });
            }
        }
    }

    // Extra topology columns for specific classes
    if (cls === 'vertex') cols.push({ name: 'point', size: 1 });
    if (cls === 'primitive') {
        cols.push({ name: 'vertices', size: 1 });
        cols.push({ name: 'points', size: 1 });
    }

    // Header row
    for (const col of cols) {
        const th = document.createElement('th');
        th.textContent = col.name;
        thead.appendChild(th);
    }

    // Fetch all attrib data upfront
    const attribDataArrays = attribs.map(a => {
        if (a.type === 'String') return { strings: geo.attribDataString(cls, a.name) ?? [], numeric: null, size: a.size };
        return { strings: null, numeric: geo.attribData(cls, a.name), size: a.size };
    });

    // Limit rows for performance
    const maxRows = Math.min(rowCount, 5000);
    const frag = document.createDocumentFragment();

    for (let i = 0; i < maxRows; i++) {
        const tr = document.createElement('tr');

        // Index column
        const tdIdx = document.createElement('td');
        tdIdx.textContent = i;
        tr.appendChild(tdIdx);

        // Attribute columns
        for (const ad of attribDataArrays) {
            if (ad.strings) {
                const td = document.createElement('td');
                td.textContent = ad.strings[i] ?? '';
                tr.appendChild(td);
            } else if (ad.numeric) {
                const offset = i * ad.size;
                for (let c = 0; c < ad.size; c++) {
                    const td = document.createElement('td');
                    const val = ad.numeric[offset + c];
                    td.textContent = val !== undefined ? (Number.isInteger(val) ? val : val.toFixed(4)) : '';
                    tr.appendChild(td);
                }
            } else {
                for (let c = 0; c < ad.size; c++) {
                    const td = document.createElement('td');
                    td.textContent = '';
                    tr.appendChild(td);
                }
            }
        }

        // Topology columns
        if (cls === 'vertex') {
            const td = document.createElement('td');
            td.textContent = geo.vertexPoint(i);
            tr.appendChild(td);
        }
        if (cls === 'primitive') {
            const td1 = document.createElement('td');
            td1.textContent = geo.primVertexCount(i);
            tr.appendChild(td1);
            const td2 = document.createElement('td');
            td2.textContent = geo.primPointIndices(i).join(', ');
            tr.appendChild(td2);
        }

        frag.appendChild(tr);
    }

    tbody.appendChild(frag);

    if (rowCount > maxRows) {
        info.textContent += ` (showing ${maxRows})`;
    }
}

// Spreadsheet subtab switching (Points/Vertices/Primitives/Detail)
document.getElementById('spreadsheet-subtabs').addEventListener('click', (e) => {
    const tab = e.target.closest('.subtab');
    if (!tab) return;
    document.querySelectorAll('#spreadsheet-subtabs .subtab').forEach(t => t.classList.remove('active'));
    tab.classList.add('active');
    spreadsheetClass = tab.dataset.class;
    updateSpreadsheet();
});

// Viewer tab switching (Viewport / Spreadsheet)
document.getElementById('viewer-tabs').addEventListener('click', (e) => {
    const tab = e.target.closest('.viewer-tab');
    if (!tab) return;
    document.querySelectorAll('.viewer-tab').forEach(t => t.classList.remove('active'));
    tab.classList.add('active');
    document.querySelectorAll('.viewer-view').forEach(v => v.classList.remove('active'));
    const viewId = tab.dataset.view === 'spreadsheet' ? 'spreadsheet-view' : 'viewport-view';
    document.getElementById(viewId).classList.add('active');
    if (tab.dataset.view === 'viewport') resizeViewer();
    if (tab.dataset.view === 'spreadsheet') updateSpreadsheet();
});

// ── Resizable divider ────────────────────────────────────
const divider = document.getElementById('divider');
const editorPanel = document.getElementById('editor-panel');
let isDragging = false;

divider.addEventListener('mousedown', () => { isDragging = true; });
window.addEventListener('mousemove', (e) => {
    if (!isDragging) return;
    const containerWidth = document.getElementById('panels').clientWidth;
    const newWidth = Math.max(200, Math.min(containerWidth - 200, e.clientX));
    editorPanel.style.width = newWidth + 'px';
    resizeViewer();
});
window.addEventListener('mouseup', () => { isDragging = false; });

// ── Init ─────────────────────────────────────────────────
window.addEventListener('resize', resizeViewer);
resizeViewer();
render();

setStatus('Loading WASM...', '');
loadWasm().then(async () => {
    const urlCode = await getCodeFromUrl();
    if (urlCode) {
        editor.setValue(urlCode);
    }
    await executeCode(editor.getValue());
    fitCameraToScene();
}).catch(e => {
    setStatus(`Failed to load WASM: ${e.message}`, 'error');
    console.error(e);
});
