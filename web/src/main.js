import * as monaco from 'monaco-editor';
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { examples } from './examples.js';

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
}

// ── Three.js Scene ───────────────────────────────────────
const canvas = document.getElementById('canvas');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
renderer.setPixelRatio(window.devicePixelRatio);
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 1.1;

const scene = new THREE.Scene();
scene.background = new THREE.Color(0x181820);

const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 100);
camera.position.set(3, 2.2, 3.5);

const controls = new OrbitControls(camera, canvas);
controls.enableDamping = true;
controls.dampingFactor = 0.08;

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

function resizeViewer() {
    const panel = document.getElementById('viewer-panel');
    const w = panel.clientWidth;
    const h = panel.clientHeight;
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    renderer.setSize(w, h);
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

function updateScene(geo) {
    currentGeo = geo;

    // Clear old meshes
    while (meshGroup.children.length) {
        const c = meshGroup.children[0];
        c.geometry?.dispose();
        c.material?.dispose();
        meshGroup.remove(c);
    }

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

    // Edges
    const edgesGeo = new THREE.EdgesGeometry(bufGeo, 20);
    meshGroup.add(new THREE.LineSegments(edgesGeo, new THREE.LineBasicMaterial({ color: 0x223355 })));

    setStatus(`${geo.numPoints} pts | ${geo.numPrims} prims`, 'success');
}

// Render loop
function render() {
    requestAnimationFrame(render);
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
function executeCode(code) {
    if (!wasmReady) {
        setStatus('WASM not loaded yet...', 'error');
        return;
    }

    try {
        const t0 = performance.now();
        // Wrap the code in an async function with pg available
        const fn = new Function('pg', `"use strict"; ${code}`);
        const result = fn(pg);
        const elapsed = (performance.now() - t0).toFixed(1);

        if (result && typeof result.getPositions === 'function') {
            updateScene(result);
            const el = document.getElementById('status');
            el.textContent += ` | ${elapsed}ms`;
        } else {
            setStatus('Code must return a Geometry object', 'error');
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

// ProcGeo type definitions for autocomplete
// Using `declare const pg` so Monaco knows pg is a variable with these methods
const procgeoTypes = `
interface ProcGeoGeometry {
    /** Number of points in the geometry. */
    readonly numPoints: number;
    /** Number of primitives (faces) in the geometry. */
    readonly numPrims: number;
    /** Number of vertices in the geometry. */
    readonly numVertices: number;
    /** Get all positions as a flat Float32Array [x0,y0,z0, x1,y1,z1, ...]. */
    getPositions(): Float32Array;
    /** Get triangle indices as a flat Uint32Array. */
    getTriangleIndices(): Uint32Array;
    /** Get normals as a flat Float32Array (if "N" attribute exists). */
    getNormals(): Float32Array | undefined;
    /** Get vertex colors as a flat Float32Array (if "Cd" attribute exists). */
    getColors(): Float32Array | undefined;
    /** Get position of a point by index. */
    pointPos(index: number): Float32Array;
    /** Get the axis-aligned bounding box. */
    boundingBox(): { min: Float32Array, max: Float32Array };
    /** Export geometry as OBJ string. */
    toObj(): string;
    /** Export geometry as GLB binary (Uint8Array). */
    toGlb(): Uint8Array;
}

declare const pg: {
    // ── Creation ──
    /** Create a box. Default: unit cube at origin. */
    createBox(params?: { size?: [number, number, number]; center?: [number, number, number] }): ProcGeoGeometry;
    /** Create a grid of quads. Default: 10x10, size 10x10 on XZ plane. */
    createGrid(params?: { rows?: number; cols?: number; sizeX?: number; sizeY?: number; center?: [number, number, number] }): ProcGeoGeometry;
    /** Create a UV sphere. Default: radius 0.5, 12 rows, 24 cols. */
    createSphere(params?: { radius?: number; rows?: number; cols?: number; center?: [number, number, number] }): ProcGeoGeometry;
    /** Create a line (open polyline). Default: unit length along Y. */
    createLine(params?: { origin?: [number, number, number]; direction?: [number, number, number]; length?: number; points?: number }): ProcGeoGeometry;
    /** Create a circle (closed polygon). Default: radius 1, 40 divisions. */
    createCircle(params?: { radius?: number; divisions?: number; center?: [number, number, number] }): ProcGeoGeometry;
    /** Create a tube/cylinder. Default: radius 0.5, height 1. */
    createTube(params?: { radiusBottom?: number; radiusTop?: number; height?: number; cols?: number; rows?: number; center?: [number, number, number] }): ProcGeoGeometry;
    /** Create a torus. Default: outer radius 1, inner radius 0.3. */
    createTorus(params?: { radiusOuter?: number; radiusInner?: number; rows?: number; cols?: number; center?: [number, number, number] }): ProcGeoGeometry;

    // ── Manipulation ──
    /** Transform geometry: translate, rotate (degrees), scale with optional pivot. */
    transform(geo: ProcGeoGeometry, params?: { translate?: [number, number, number]; rotate?: [number, number, number]; scale?: [number, number, number]; pivot?: [number, number, number] }): ProcGeoGeometry;
    /** Compute vertex normals (area-weighted). */
    computeNormals(geo: ProcGeoGeometry): ProcGeoGeometry;
    /** Subdivide geometry. mode: "linear" or "catmullClark". */
    subdivide(geo: ProcGeoGeometry, params?: { depth?: number; mode?: "linear" | "catmullClark" }): ProcGeoGeometry;
    /** Scatter random points on mesh surface (area-weighted). */
    scatter(geo: ProcGeoGeometry, params?: { count?: number; seed?: number }): ProcGeoGeometry;
    /** Copy source geometry onto each point of target. */
    copyToPoints(source: ProcGeoGeometry, target: ProcGeoGeometry): ProcGeoGeometry;
    /** Extrude polygon faces along their normals. */
    polyExtrude(geo: ProcGeoGeometry, params?: { distance?: number; inset?: number; outputFront?: boolean; outputSide?: boolean }): ProcGeoGeometry;
    /** Laplacian smoothing. */
    smooth(geo: ProcGeoGeometry, params?: { iterations?: number; strength?: number }): ProcGeoGeometry;
    /** Clip geometry by a plane, keeping one side. */
    clip(geo: ProcGeoGeometry, params?: { origin?: [number, number, number]; normal?: [number, number, number]; keepAbove?: boolean }): ProcGeoGeometry;
    /** Reverse polygon winding order (flip normals). */
    reverse(geo: ProcGeoGeometry): ProcGeoGeometry;
    /** Set vertex color attribute (Cd). */
    color(geo: ProcGeoGeometry, params?: { color?: [number, number, number] }): ProcGeoGeometry;
    /** Merge coincident points within a distance tolerance. */
    fuse(geo: ProcGeoGeometry, params?: { distance?: number }): ProcGeoGeometry;
    /** Voronoi fracture a mesh into pieces. */
    voronoiFracture(geo: ProcGeoGeometry, params?: { numPoints?: number; seed?: number; createInsideFaces?: boolean }): ProcGeoGeometry;
};
`;

// Create editor
const editor = monaco.editor.create(document.getElementById('editor'), {
    value: examples.basic,
    language: 'javascript',
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
monaco.languages.typescript.javascriptDefaults.setDiagnosticsOptions({
    noSemanticValidation: true,
    noSyntaxValidation: true,
});

monaco.languages.typescript.javascriptDefaults.setCompilerOptions({
    target: monaco.languages.typescript.ScriptTarget.ESNext,
    allowNonTsExtensions: true,
    allowJs: true,
    checkJs: true,
});

monaco.languages.typescript.javascriptDefaults.addExtraLib(procgeoTypes, 'procgeo.d.ts');

// ── Debounced auto-compile ───────────────────────────────
let debounceTimer = null;

function scheduleRun() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
        executeCode(editor.getValue());
    }, 500);
}

editor.onDidChangeModelContent(scheduleRun);

// Ctrl+Enter to run immediately
editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
    clearTimeout(debounceTimer);
    executeCode(editor.getValue());
});

// ── Examples dropdown ────────────────────────────────────
document.getElementById('examples').addEventListener('change', (e) => {
    const key = e.target.value;
    if (key && examples[key]) {
        editor.setValue(examples[key]);
        executeCode(examples[key]);
    }
    e.target.value = '';
});

// ── Run button ───────────────────────────────────────────
document.getElementById('run-btn').addEventListener('click', () => {
    clearTimeout(debounceTimer);
    executeCode(editor.getValue());
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
loadWasm().then(() => {
    executeCode(editor.getValue());
}).catch(e => {
    setStatus(`Failed to load WASM: ${e.message}`, 'error');
    console.error(e);
});
