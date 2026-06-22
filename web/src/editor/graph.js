// ─────────────────────────────────────────────────────────────────────────────
// Graph model — nodes, connections, serialization and topological evaluation.
//
// A graph is a DAG of SOP nodes. Each node produces one geometry output.
// Connections feed a source node's output into a destination node's input port.
// Evaluation is lazy + memoized: we resolve the displayed node and recursively
// cook its upstream dependencies, caching results within a single cook pass.
// ─────────────────────────────────────────────────────────────────────────────

import { getNodeDef, defaultParams, paramsObject } from './registry.js';

let nextId = 1;
function genId() {
    return `n${nextId++}`;
}

export class Graph {
    constructor() {
        /** @type {Map<string, object>} id -> node */
        this.nodes = new Map();
        /** @type {Array<{from:string, to:string, port:number}>} */
        this.connections = [];
        this.displayNodeId = null;
        this._listeners = new Set();
    }

    onChange(fn) {
        this._listeners.add(fn);
        return () => this._listeners.delete(fn);
    }

    emit() {
        for (const fn of this._listeners) fn();
    }

    addNode(type, x = 0, y = 0) {
        const def = getNodeDef(type);
        if (!def) throw new Error(`Unknown node type: ${type}`);
        const id = genId();
        const node = {
            id,
            type,
            x,
            y,
            name: def.label,
            params: defaultParams(type),
        };
        this.nodes.set(id, node);
        if (!this.displayNodeId) this.displayNodeId = id;
        this.emit();
        return node;
    }

    removeNode(id) {
        this.nodes.delete(id);
        this.connections = this.connections.filter((c) => c.from !== id && c.to !== id);
        if (this.displayNodeId === id) {
            this.displayNodeId = this.nodes.size ? [...this.nodes.keys()][0] : null;
        }
        this.emit();
    }

    // Connect `from` node output → `to` node input `port`.
    // Each input port accepts a single connection (last write wins).
    connect(from, to, port) {
        if (from === to) return;
        const def = getNodeDef(this.nodes.get(to).type);
        if (port >= def.inputs.length) return;
        if (this.wouldCreateCycle(from, to)) return;
        this.connections = this.connections.filter((c) => !(c.to === to && c.port === port));
        this.connections.push({ from, to, port });
        this.emit();
    }

    disconnect(to, port) {
        this.connections = this.connections.filter((c) => !(c.to === to && c.port === port));
        this.emit();
    }

    // Inputs feeding `nodeId`, indexed by port (null where unconnected).
    inputsOf(nodeId) {
        const def = getNodeDef(this.nodes.get(nodeId).type);
        const arr = new Array(def.inputs.length).fill(null);
        for (const c of this.connections) {
            if (c.to === nodeId && c.port < arr.length) arr[c.port] = c.from;
        }
        return arr;
    }

    wouldCreateCycle(from, to) {
        // Adding from→to creates a cycle if `from` is reachable downstream of `to`.
        const stack = [to];
        const seen = new Set();
        while (stack.length) {
            const cur = stack.pop();
            if (cur === from) return true;
            if (seen.has(cur)) continue;
            seen.add(cur);
            for (const c of this.connections) {
                if (c.from === cur) stack.push(c.to);
            }
        }
        return false;
    }

    setDisplay(id) {
        this.displayNodeId = id;
        this.emit();
    }

    setParam(id, name, value) {
        const node = this.nodes.get(id);
        if (!node) return;
        node.params[name] = value;
        this.emit();
    }

    // ── Evaluation ────────────────────────────────────────────────────────────
    // Cook the displayed node, returning { geo, errors } where errors maps
    // nodeId -> message for any node that failed.
    cook(pg, targetId = this.displayNodeId) {
        const cache = new Map();
        const errors = new Map();

        const evalNode = (id) => {
            if (cache.has(id)) return cache.get(id);
            const node = this.nodes.get(id);
            if (!node) return null;
            const def = getNodeDef(node.type);

            // Resolve inputs first.
            const inputIds = this.inputsOf(id);
            const inputs = [];
            for (let port = 0; port < def.inputs.length; port++) {
                const srcId = inputIds[port];
                const required = def.inputs[port].required;
                if (srcId == null) {
                    if (required) {
                        errors.set(id, `Missing input: ${def.inputs[port].label}`);
                        cache.set(id, null);
                        return null;
                    }
                    inputs.push(null);
                    continue;
                }
                const upstream = evalNode(srcId);
                if (upstream == null) {
                    // Upstream failed; propagate without overwriting its own error.
                    if (!errors.has(id)) errors.set(id, 'Upstream error');
                    cache.set(id, null);
                    return null;
                }
                inputs.push(upstream);
            }

            try {
                const result = def.make(pg, inputs, paramsObject(def, node.params));
                cache.set(id, result);
                return result;
            } catch (e) {
                errors.set(id, e?.message ?? String(e));
                cache.set(id, null);
                return null;
            }
        };

        const geo = targetId ? evalNode(targetId) : null;
        return { geo, errors };
    }

    // ── Serialization ─────────────────────────────────────────────────────────
    toJSON() {
        return {
            version: 1,
            nodes: [...this.nodes.values()].map((n) => ({
                id: n.id, type: n.type, x: n.x, y: n.y, name: n.name, params: n.params,
            })),
            connections: this.connections.map((c) => ({ ...c })),
            display: this.displayNodeId,
        };
    }

    static fromJSON(data) {
        const g = new Graph();
        let maxId = 0;
        for (const n of data.nodes ?? []) {
            if (!getNodeDef(n.type)) continue; // skip unknown node types
            const merged = { ...defaultParams(n.type), ...(n.params ?? {}) };
            g.nodes.set(n.id, { id: n.id, type: n.type, x: n.x, y: n.y, name: n.name ?? n.type, params: merged });
            const num = parseInt(String(n.id).replace(/\D/g, ''), 10);
            if (Number.isFinite(num)) maxId = Math.max(maxId, num);
        }
        for (const c of data.connections ?? []) {
            if (g.nodes.has(c.from) && g.nodes.has(c.to)) g.connections.push({ from: c.from, to: c.to, port: c.port });
        }
        g.displayNodeId = g.nodes.has(data.display) ? data.display : (g.nodes.size ? [...g.nodes.keys()][0] : null);
        nextId = Math.max(nextId, maxId + 1);
        return g;
    }
}
