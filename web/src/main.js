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
    // Try loading from the WASM package
    // The built WASM files should be copied/linked to public/
    const wasmModule = await import('/wasm/procgeo_wasm.js');
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
// Configure Monaco workers
self.MonacoEnvironment = {
    getWorkerUrl: function (moduleId, label) {
        if (label === 'typescript' || label === 'javascript') {
            return `data:text/javascript;charset=utf-8,${encodeURIComponent(`
                self.MonacoEnvironment = { baseUrl: 'https://cdn.jsdelivr.net/npm/monaco-editor@0.52.0/min/' };
                importScripts('https://cdn.jsdelivr.net/npm/monaco-editor@0.52.0/min/vs/base/worker/workerMain.js');
            `)}`;
        }
        return `data:text/javascript;charset=utf-8,${encodeURIComponent(`
            self.MonacoEnvironment = { baseUrl: 'https://cdn.jsdelivr.net/npm/monaco-editor@0.52.0/min/' };
            importScripts('https://cdn.jsdelivr.net/npm/monaco-editor@0.52.0/min/vs/base/worker/workerMain.js');
        `)}`;
    }
};

// ProcGeo type definitions for autocomplete
const procgeoTypes = `
declare namespace pg {
    interface Geometry {
        readonly numPoints: number;
        readonly numPrims: number;
        readonly numVertices: number;
        getPositions(): Float32Array;
        getTriangleIndices(): Uint32Array;
        getNormals(): Float32Array | undefined;
        getColors(): Float32Array | undefined;
        pointPos(index: number): Float32Array;
        boundingBox(): { min: Float32Array, max: Float32Array };
        toObj(): string;
        toGlb(): Uint8Array;
    }

    /** Create a box. */
    function createBox(params?: { size?: [number, number, number]; center?: [number, number, number] }): Geometry;
    /** Create a grid. */
    function createGrid(params?: { rows?: number; cols?: number; sizeX?: number; sizeY?: number; center?: [number, number, number] }): Geometry;
    /** Create a UV sphere. */
    function createSphere(params?: { radius?: number; rows?: number; cols?: number; center?: [number, number, number] }): Geometry;
    /** Create a line. */
    function createLine(params?: { origin?: [number, number, number]; direction?: [number, number, number]; length?: number; points?: number }): Geometry;
    /** Create a circle. */
    function createCircle(params?: { radius?: number; divisions?: number; center?: [number, number, number] }): Geometry;
    /** Create a tube/cylinder. */
    function createTube(params?: { radiusBottom?: number; radiusTop?: number; height?: number; cols?: number; rows?: number; center?: [number, number, number] }): Geometry;
    /** Create a torus. */
    function createTorus(params?: { radiusOuter?: number; radiusInner?: number; rows?: number; cols?: number; center?: [number, number, number] }): Geometry;
    /** Transform geometry (translate, rotate in degrees, scale). */
    function transform(geo: Geometry, params?: { translate?: [number, number, number]; rotate?: [number, number, number]; scale?: [number, number, number]; pivot?: [number, number, number] }): Geometry;
    /** Compute vertex normals. */
    function computeNormals(geo: Geometry): Geometry;
    /** Subdivide geometry. Mode: "linear" or "catmullClark". */
    function subdivide(geo: Geometry, params?: { depth?: number; mode?: "linear" | "catmullClark" }): Geometry;
    /** Scatter random points on mesh surface. */
    function scatter(geo: Geometry, params?: { count?: number; seed?: number }): Geometry;
    /** Copy source geometry to each point of target. */
    function copyToPoints(source: Geometry, target: Geometry): Geometry;
    /** Extrude polygon faces. */
    function polyExtrude(geo: Geometry, params?: { distance?: number; inset?: number; outputFront?: boolean; outputSide?: boolean }): Geometry;
    /** Laplacian smoothing. */
    function smooth(geo: Geometry, params?: { iterations?: number; strength?: number }): Geometry;
    /** Clip geometry by a plane. */
    function clip(geo: Geometry, params?: { origin?: [number, number, number]; normal?: [number, number, number]; keepAbove?: boolean }): Geometry;
    /** Reverse polygon winding (flip normals). */
    function reverse(geo: Geometry): Geometry;
    /** Set vertex color attribute (Cd). */
    function color(geo: Geometry, params?: { color?: [number, number, number] }): Geometry;
    /** Merge coincident points within distance. */
    function fuse(geo: Geometry, params?: { distance?: number }): Geometry;
    /** Voronoi fracture a mesh into pieces. */
    function voronoiFracture(geo: Geometry, params?: { numPoints?: number; seed?: number; createInsideFaces?: boolean }): Geometry;
}
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

// Add type definitions for autocomplete
monaco.languages.typescript.javascriptDefaults.setDiagnosticsOptions({
    noSemanticValidation: false,
    noSyntaxValidation: false,
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
