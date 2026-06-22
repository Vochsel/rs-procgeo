// Headless smoke test for the visual editor's node registry + graph evaluation.
// Loads the real WASM module in Node and cooks every node type and every preset,
// catching param-name mismatches or runtime errors that a bundler build can't.
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const webRoot = path.resolve(here, '..');

const pgMod = await import(path.join(webRoot, 'wasm/procgeo_wasm.js'));
const bytes = await readFile(path.join(webRoot, 'wasm/procgeo_wasm_bg.wasm'));
await pgMod.default({ module_or_path: bytes });
const pg = pgMod;

const { NODE_REGISTRY, defaultParams, paramsObject } = await import(path.join(webRoot, 'src/editor/registry.js'));
const { Graph } = await import(path.join(webRoot, 'src/editor/graph.js'));
const { PRESETS } = await import(path.join(webRoot, 'src/editor/presets.js'));

let failures = 0;

// Some nodes need a specific upstream (e.g. blast needs an existing group).
function makeUpstream(g, type) {
    if (type === 'blast') {
        const box = g.addNode('box', 0, 0);
        const grp = g.addNode('groupCreate', 0, 0);
        grp.params.name = 'group1';
        grp.params.mode = 'boundingBox';
        g.connect(box.id, grp.id, 0);
        return grp.id;
    }
    return g.addNode('box', 0, 0).id;
}

// 1) Every single-output node: feed each input and cook it.
console.log('— Node coverage —');
for (const [type, def] of NODE_REGISTRY) {
    const g = new Graph();
    const target = g.addNode(type, 0, 0);
    // Wire a suitable source into each required input.
    def.inputs.forEach((inp, port) => {
        g.connect(makeUpstream(g, type), target.id, port);
    });
    const { geo, errors } = g.cook(pg, target.id);
    if (geo && typeof geo.getPositions === 'function') {
        console.log(`  ok   ${type.padEnd(18)} ${geo.numPoints} pts / ${geo.numPrims} prims`);
    } else {
        console.log(`  FAIL ${type.padEnd(18)} ${errors.get(target.id) ?? 'no output'}`);
        failures++;
    }
}

// 2) Every preset graph.
console.log('\n— Presets —');
for (const [key, preset] of Object.entries(PRESETS)) {
    const g = Graph.fromJSON(preset.graph);
    const { geo, errors } = g.cook(pg);
    if (geo && typeof geo.getPositions === 'function') {
        console.log(`  ok   ${key.padEnd(12)} ${geo.numPoints} pts / ${geo.numPrims} prims`);
    } else {
        const msg = [...errors.values()].join('; ') || 'no output';
        console.log(`  FAIL ${key.padEnd(12)} ${msg}`);
        failures++;
    }
}

console.log(`\n${failures === 0 ? 'ALL PASSED' : failures + ' FAILURE(S)'}`);
process.exit(failures === 0 ? 0 : 1);
