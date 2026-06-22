// ─────────────────────────────────────────────────────────────────────────────
// ProcGeo Visual Editor — node-graph entry point.
// Loads the procgeo WASM module, builds the graph + network view + params panel
// + 3D viewport, cooks the displayed node on every change, and persists the
// graph to the URL for sharing.
// ─────────────────────────────────────────────────────────────────────────────

import { Graph } from './graph.js';
import { NetworkView } from './network-view.js';
import { ParamsPanel } from './params-panel.js';
import { Viewport } from './viewport.js';
import { STARTER_GRAPH, PRESETS } from './presets.js';

// ── WASM ──────────────────────────────────────────────────────────────────────
let pg = null;
let wasmReady = false;

async function loadWasm() {
    const mod = await import('procgeo-wasm');
    await mod.default();
    pg = mod;
    wasmReady = true;
}

// ── Status ──────────────────────────────────────────────────────────────────
function setStatus(text, type = '') {
    const el = document.getElementById('status');
    el.textContent = text;
    el.className = type;
}

// ── App wiring ────────────────────────────────────────────────────────────────
const graph = Graph.fromJSON(STARTER_GRAPH);

const viewport = new Viewport(
    document.getElementById('canvas'),
    document.getElementById('viewport-view'),
);

const paramsPanel = new ParamsPanel(document.getElementById('params-panel'), graph);

const network = new NetworkView(document.getElementById('network'), graph, {
    onSelect: (node) => paramsPanel.show(node),
});

document.getElementById('network').addEventListener('nv-frame', () => viewport.frame());

// ── Cook + persist on every graph change ───────────────────────────────────────
let cookScheduled = false;
function scheduleCook() {
    if (cookScheduled) return;
    cookScheduled = true;
    requestAnimationFrame(() => {
        cookScheduled = false;
        cook();
        persist();
    });
}

function cook() {
    if (!wasmReady) return;
    if (!graph.displayNodeId) {
        viewport.clear();
        network.setErrors(new Map());
        setStatus('Empty graph', '');
        return;
    }
    const t0 = performance.now();
    const { geo, errors } = graph.cook(pg);
    network.setErrors(errors);

    if (geo && typeof geo.getPositions === 'function') {
        viewport.setGeometry(geo);
        const dt = (performance.now() - t0).toFixed(1);
        setStatus(`${geo.numPoints} pts · ${geo.numPrims} prims · ${dt}ms`, 'success');
    } else {
        viewport.clear();
        const displayErr = errors.get(graph.displayNodeId);
        setStatus(displayErr ? `Error: ${displayErr}` : 'No output', 'error');
    }
}

graph.onChange(scheduleCook);

// ── URL persistence (shared with the code playground's scheme) ──────────────────
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
async function compress(text) {
    const stream = new Blob([new TextEncoder().encode(text)]).stream()
        .pipeThrough(new CompressionStream('deflate-raw'));
    return toBase64Url(new Uint8Array(await new Response(stream).arrayBuffer()));
}
async function decompress(encoded) {
    const stream = new Blob([fromBase64Url(encoded)]).stream()
        .pipeThrough(new DecompressionStream('deflate-raw'));
    return new TextDecoder().decode(await new Response(stream).arrayBuffer());
}

let persistTimer = null;
function persist() {
    clearTimeout(persistTimer);
    persistTimer = setTimeout(async () => {
        try {
            const encoded = await compress(JSON.stringify(graph.toJSON()));
            const url = new URL(window.location.href);
            url.search = '?graph=' + encoded;
            window.history.replaceState(null, '', url);
        } catch (e) { /* non-fatal */ }
    }, 400);
}

async function loadFromUrl() {
    const params = new URLSearchParams(window.location.search);
    const encoded = params.get('graph');
    if (!encoded) return null;
    try {
        return JSON.parse(await decompress(encoded));
    } catch (e) {
        console.warn('Failed to decode graph from URL', e);
        return null;
    }
}

function replaceGraph(data) {
    const fresh = Graph.fromJSON(data);
    graph.nodes = fresh.nodes;
    graph.connections = fresh.connections;
    graph.displayNodeId = fresh.displayNodeId;
    network.selectedId = null;
    paramsPanel.showEmpty();
    graph.emit();
    setTimeout(() => viewport.frame(), 50);
}

function showToast(msg) {
    const el = document.getElementById('toast');
    el.textContent = msg;
    el.classList.add('show');
    setTimeout(() => el.classList.remove('show'), 2000);
}

// ── Toolbar ────────────────────────────────────────────────────────────────────
document.getElementById('view-modes').addEventListener('click', (e) => {
    const btn = e.target.closest('.view-mode-btn');
    if (!btn) return;
    document.querySelectorAll('.view-mode-btn').forEach((b) => b.classList.remove('active'));
    btn.classList.add('active');
    viewport.setViewMode(btn.dataset.mode);
});

document.getElementById('fit-btn').addEventListener('click', () => viewport.frame());

document.getElementById('new-btn').addEventListener('click', () => {
    if (!confirm('Clear the current graph?')) return;
    replaceGraph({ nodes: [], connections: [], display: null });
});

document.getElementById('share-btn').addEventListener('click', async () => {
    try {
        const encoded = await compress(JSON.stringify(graph.toJSON()));
        const url = new URL(window.location.href);
        url.search = '?graph=' + encoded;
        await navigator.clipboard.writeText(url.toString());
        window.history.replaceState(null, '', url);
        showToast('Link copied to clipboard');
    } catch (e) {
        showToast('Failed to copy link');
    }
});

document.getElementById('export-obj').addEventListener('click', () => {
    const geo = viewport.currentGeo;
    if (!geo) return showToast('Nothing to export');
    download(new Blob([geo.toObj()], { type: 'text/plain' }), 'procgeo.obj');
});
document.getElementById('export-glb').addEventListener('click', () => {
    const geo = viewport.currentGeo;
    if (!geo) return showToast('Nothing to export');
    download(new Blob([geo.toGlb()], { type: 'model/gltf-binary' }), 'procgeo.glb');
});
function download(blob, name) {
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = name;
    a.click();
}

// Presets dropdown.
const presetSelect = document.getElementById('presets');
for (const key of Object.keys(PRESETS)) {
    const o = document.createElement('option');
    o.value = key;
    o.textContent = PRESETS[key].label;
    presetSelect.appendChild(o);
}
presetSelect.addEventListener('change', (e) => {
    const key = e.target.value;
    if (key && PRESETS[key]) replaceGraph(PRESETS[key].graph);
    e.target.value = '';
});

// ── Resizable divider between network and viewport ─────────────────────────────
const divider = document.getElementById('divider');
const networkPanel = document.getElementById('network-panel');
let dragging = false;
divider.addEventListener('mousedown', () => { dragging = true; document.body.style.cursor = 'col-resize'; });
window.addEventListener('mousemove', (e) => {
    if (!dragging) return;
    const total = document.getElementById('panels').clientWidth;
    const w = Math.max(280, Math.min(total - 360, e.clientX));
    networkPanel.style.width = w + 'px';
    viewport.resize();
});
window.addEventListener('mouseup', () => { dragging = false; document.body.style.cursor = ''; });

window.addEventListener('resize', () => viewport.resize());

// ── Boot ────────────────────────────────────────────────────────────────────────
setStatus('Loading WASM…', '');
loadWasm().then(async () => {
    const urlGraph = await loadFromUrl();
    if (urlGraph) replaceGraph(urlGraph);
    viewport.resize();
    cook();
    setTimeout(() => viewport.frame(), 80);
}).catch((e) => {
    setStatus(`Failed to load WASM: ${e.message}`, 'error');
    console.error(e);
});
