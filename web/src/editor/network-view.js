// ─────────────────────────────────────────────────────────────────────────────
// NetworkView — the visual node-graph editor.
//
// Renders graph nodes as DOM elements inside a pannable/zoomable "world" layer,
// with connections drawn as bezier paths in an SVG layer. Supports node drag,
// wiring ports together, selection, the display flag, and an add-node menu.
//
// Geometry flows top → bottom: input ports sit on a node's top edge, the single
// output port on its bottom edge.
// ─────────────────────────────────────────────────────────────────────────────

import { getNodeDef, NODE_CATEGORIES } from './registry.js';

const NODE_W = 168;
const NODE_H = 48;
const SVG_NS = 'http://www.w3.org/2000/svg';

export class NetworkView {
    constructor(root, graph, { onSelect } = {}) {
        this.root = root;
        this.graph = graph;
        this.onSelect = onSelect || (() => {});

        this.scale = 1;
        this.pan = { x: 60, y: 60 };
        this.selectedId = null;
        this.errorNodes = new Map();

        this._buildDom();
        this._bindEvents();

        graph.onChange(() => this.render());
        this.render();
    }

    _buildDom() {
        this.root.classList.add('nv-root');
        this.root.innerHTML = '';

        this.world = document.createElement('div');
        this.world.className = 'nv-world';

        this.svg = document.createElementNS(SVG_NS, 'svg');
        this.svg.classList.add('nv-wires');
        this.world.appendChild(this.svg);

        // Temporary wire shown while dragging a new connection.
        this.tempPath = document.createElementNS(SVG_NS, 'path');
        this.tempPath.classList.add('nv-wire', 'nv-wire-temp');
        this.tempPath.style.display = 'none';
        this.svg.appendChild(this.tempPath);

        this.root.appendChild(this.world);

        // Add-node menu (built lazily, reused).
        this.menu = document.createElement('div');
        this.menu.className = 'nv-menu';
        this.menu.style.display = 'none';
        this.root.appendChild(this.menu);

        this._applyTransform();
    }

    _applyTransform() {
        this.world.style.transform = `translate(${this.pan.x}px, ${this.pan.y}px) scale(${this.scale})`;
    }

    // ── Coordinate helpers ─────────────────────────────────────────────────────
    screenToWorld(clientX, clientY) {
        const rect = this.root.getBoundingClientRect();
        return {
            x: (clientX - rect.left - this.pan.x) / this.scale,
            y: (clientY - rect.top - this.pan.y) / this.scale,
        };
    }

    inputPortPos(node, port, count) {
        return { x: node.x + (NODE_W * (port + 1)) / (count + 1), y: node.y };
    }

    outputPortPos(node) {
        return { x: node.x + NODE_W / 2, y: node.y + NODE_H };
    }

    // ── Rendering ──────────────────────────────────────────────────────────────
    render() {
        // Remove existing node elements (keep svg).
        for (const el of this.world.querySelectorAll('.nv-node')) el.remove();

        for (const node of this.graph.nodes.values()) {
            this.world.appendChild(this._renderNode(node));
        }
        this._renderWires();
    }

    _renderNode(node) {
        const def = getNodeDef(node.type);
        const el = document.createElement('div');
        el.className = 'nv-node';
        el.dataset.id = node.id;
        el.style.left = `${node.x}px`;
        el.style.top = `${node.y}px`;
        el.style.width = `${NODE_W}px`;
        el.style.height = `${NODE_H}px`;
        if (node.id === this.selectedId) el.classList.add('selected');
        if (node.id === this.graph.displayNodeId) el.classList.add('display');
        if (this.errorNodes.has(node.id)) {
            el.classList.add('error');
            el.title = this.errorNodes.get(node.id);
        }

        // Input ports (top edge).
        const n = def.inputs.length;
        for (let p = 0; p < n; p++) {
            const pos = this.inputPortPos(node, p, n);
            const dot = document.createElement('div');
            dot.className = 'nv-port nv-port-in';
            dot.dataset.node = node.id;
            dot.dataset.port = String(p);
            dot.title = def.inputs[p].label;
            dot.style.left = `${(pos.x - node.x) - 6}px`;
            dot.style.top = '-6px';
            if (!def.inputs[p].required) dot.classList.add('optional');
            el.appendChild(dot);
        }

        // Output port (bottom edge) — creation nodes still emit geometry.
        const out = document.createElement('div');
        out.className = 'nv-port nv-port-out';
        out.dataset.node = node.id;
        out.style.left = `${NODE_W / 2 - 6}px`;
        out.style.bottom = '-6px';
        el.appendChild(out);

        // Display flag.
        const flag = document.createElement('div');
        flag.className = 'nv-flag';
        flag.title = 'Set display flag';
        flag.dataset.node = node.id;
        el.appendChild(flag);

        const label = document.createElement('div');
        label.className = 'nv-node-label';
        label.textContent = node.name;
        el.appendChild(label);

        const cat = document.createElement('div');
        cat.className = 'nv-node-cat';
        cat.textContent = def.category;
        el.appendChild(cat);

        return el;
    }

    _wirePath(sx, sy, ex, ey) {
        const dy = Math.max(30, Math.abs(ey - sy) * 0.4);
        return `M ${sx} ${sy} C ${sx} ${sy + dy}, ${ex} ${ey - dy}, ${ex} ${ey}`;
    }

    _renderWires() {
        for (const el of this.svg.querySelectorAll('.nv-wire:not(.nv-wire-temp)')) el.remove();
        for (const c of this.graph.connections) {
            const from = this.graph.nodes.get(c.from);
            const to = this.graph.nodes.get(c.to);
            if (!from || !to) continue;
            const def = getNodeDef(to.type);
            const s = this.outputPortPos(from);
            const e = this.inputPortPos(to, c.port, def.inputs.length);
            const path = document.createElementNS(SVG_NS, 'path');
            path.classList.add('nv-wire');
            path.setAttribute('d', this._wirePath(s.x, s.y, e.x, e.y));
            this.svg.appendChild(path);
        }
    }

    setErrors(errorsMap) {
        this.errorNodes = errorsMap || new Map();
        this.render();
    }

    select(id) {
        this.selectedId = id;
        for (const el of this.world.querySelectorAll('.nv-node')) {
            el.classList.toggle('selected', el.dataset.id === id);
        }
        this.onSelect(id ? this.graph.nodes.get(id) : null);
    }

    // ── Event handling ──────────────────────────────────────────────────────────
    _bindEvents() {
        this.root.addEventListener('mousedown', (e) => this._onMouseDown(e));
        this.root.addEventListener('wheel', (e) => this._onWheel(e), { passive: false });
        this.root.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            this._openMenu(e.clientX, e.clientY);
        });
        window.addEventListener('keydown', (e) => this._onKeyDown(e));
        // Dismiss menu on outside click.
        document.addEventListener('mousedown', (e) => {
            if (this.menu.style.display !== 'none' && !this.menu.contains(e.target)) {
                this._closeMenu();
            }
        });
    }

    _onMouseDown(e) {
        if (e.button !== 0 && e.button !== 1) return;
        if (e.target.closest('.nv-menu')) return; // let the menu handle its own clicks
        const portIn = e.target.closest('.nv-port-in');
        const portOut = e.target.closest('.nv-port-out');
        const flag = e.target.closest('.nv-flag');
        const nodeEl = e.target.closest('.nv-node');

        if (flag) {
            e.stopPropagation();
            this.graph.setDisplay(flag.dataset.node);
            return;
        }
        if (portOut) {
            e.stopPropagation();
            this._startConnectFromOutput(portOut.dataset.node, e);
            return;
        }
        if (portIn) {
            e.stopPropagation();
            this._startReconnectInput(portIn.dataset.node, parseInt(portIn.dataset.port, 10), e);
            return;
        }
        if (nodeEl) {
            this.select(nodeEl.dataset.id);
            this._startNodeDrag(nodeEl.dataset.id, e);
            return;
        }
        // Empty background → pan (left or middle button).
        this.select(null);
        this._startPan(e);
    }

    _startNodeDrag(id, e) {
        const node = this.graph.nodes.get(id);
        const start = this.screenToWorld(e.clientX, e.clientY);
        const offX = start.x - node.x;
        const offY = start.y - node.y;
        const el = this.world.querySelector(`.nv-node[data-id="${id}"]`);

        const move = (ev) => {
            const w = this.screenToWorld(ev.clientX, ev.clientY);
            node.x = Math.round(w.x - offX);
            node.y = Math.round(w.y - offY);
            el.style.left = `${node.x}px`;
            el.style.top = `${node.y}px`;
            this._renderWires();
        };
        const up = () => {
            window.removeEventListener('mousemove', move);
            window.removeEventListener('mouseup', up);
            this.onSelect(this.graph.nodes.get(id)); // refresh panel position state
            this.graph.emit(); // persist layout
        };
        window.addEventListener('mousemove', move);
        window.addEventListener('mouseup', up);
    }

    _startPan(e) {
        const startX = e.clientX;
        const startY = e.clientY;
        const px = this.pan.x;
        const py = this.pan.y;
        this.root.classList.add('panning');
        const move = (ev) => {
            this.pan.x = px + (ev.clientX - startX);
            this.pan.y = py + (ev.clientY - startY);
            this._applyTransform();
        };
        const up = () => {
            this.root.classList.remove('panning');
            window.removeEventListener('mousemove', move);
            window.removeEventListener('mouseup', up);
        };
        window.addEventListener('mousemove', move);
        window.addEventListener('mouseup', up);
    }

    _startConnectFromOutput(fromId, e) {
        const from = this.graph.nodes.get(fromId);
        const s = this.outputPortPos(from);
        this.tempPath.style.display = '';

        const move = (ev) => {
            const w = this.screenToWorld(ev.clientX, ev.clientY);
            this.tempPath.setAttribute('d', this._wirePath(s.x, s.y, w.x, w.y));
        };
        const up = (ev) => {
            window.removeEventListener('mousemove', move);
            window.removeEventListener('mouseup', up);
            this.tempPath.style.display = 'none';
            const portIn = ev.target.closest?.('.nv-port-in');
            if (portIn) {
                this.graph.connect(fromId, portIn.dataset.node, parseInt(portIn.dataset.port, 10));
            }
        };
        window.addEventListener('mousemove', move);
        window.addEventListener('mouseup', up);
    }

    // Grab an existing input connection (or start a fresh one) and rewire it.
    _startReconnectInput(toId, port, e) {
        const existing = this.graph.connections.find((c) => c.to === toId && c.port === port);
        if (existing) {
            // Detach and drag from the original source's output.
            this.graph.disconnect(toId, port);
            this._startConnectFromOutput(existing.from, e);
        }
    }

    _onWheel(e) {
        e.preventDefault();
        const rect = this.root.getBoundingClientRect();
        const mx = e.clientX - rect.left;
        const my = e.clientY - rect.top;
        const factor = e.deltaY < 0 ? 1.1 : 1 / 1.1;
        const newScale = Math.max(0.2, Math.min(2.5, this.scale * factor));
        // Zoom around cursor.
        const wx = (mx - this.pan.x) / this.scale;
        const wy = (my - this.pan.y) / this.scale;
        this.pan.x = mx - wx * newScale;
        this.pan.y = my - wy * newScale;
        this.scale = newScale;
        this._applyTransform();
    }

    _onKeyDown(e) {
        if (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT' || e.target.tagName === 'TEXTAREA') return;
        if ((e.key === 'Delete' || e.key === 'Backspace') && this.selectedId) {
            const id = this.selectedId;
            this.select(null);
            this.graph.removeNode(id);
        }
        if (e.key === 'Tab') {
            e.preventDefault();
            const rect = this.root.getBoundingClientRect();
            this._openMenu(rect.left + rect.width / 2, rect.top + rect.height / 3);
        }
        if (e.key === 'f' && this.selectedId) {
            // handled by main (frame) — dispatch a custom event
            this.root.dispatchEvent(new CustomEvent('nv-frame'));
        }
    }

    // ── Add-node menu ────────────────────────────────────────────────────────────
    _openMenu(clientX, clientY) {
        const world = this.screenToWorld(clientX, clientY);
        this.menu.innerHTML = '';

        const search = document.createElement('input');
        search.className = 'nv-menu-search';
        search.placeholder = 'Add node…';
        this.menu.appendChild(search);

        const list = document.createElement('div');
        list.className = 'nv-menu-list';
        this.menu.appendChild(list);

        const buildList = (filter = '') => {
            list.innerHTML = '';
            const f = filter.trim().toLowerCase();
            for (const [cat, defs] of NODE_CATEGORIES) {
                const matched = defs.filter((d) => !f || d.label.toLowerCase().includes(f) || d.type.toLowerCase().includes(f));
                if (!matched.length) continue;
                const header = document.createElement('div');
                header.className = 'nv-menu-cat';
                header.textContent = cat;
                list.appendChild(header);
                for (const d of matched) {
                    const item = document.createElement('div');
                    item.className = 'nv-menu-item';
                    item.textContent = d.label;
                    item.addEventListener('click', () => {
                        const node = this.graph.addNode(d.type, Math.round(world.x - NODE_W / 2), Math.round(world.y - NODE_H / 2));
                        this._closeMenu();
                        this.select(node.id);
                    });
                    list.appendChild(item);
                }
            }
        };
        buildList();

        search.addEventListener('input', () => buildList(search.value));
        search.addEventListener('keydown', (ev) => {
            if (ev.key === 'Escape') this._closeMenu();
            if (ev.key === 'Enter') {
                const first = list.querySelector('.nv-menu-item');
                if (first) first.click();
            }
        });

        const rect = this.root.getBoundingClientRect();
        this.menu.style.left = `${Math.min(clientX - rect.left, rect.width - 240)}px`;
        this.menu.style.top = `${Math.min(clientY - rect.top, rect.height - 320)}px`;
        this.menu.style.display = 'block';
        search.focus();
    }

    _closeMenu() {
        this.menu.style.display = 'none';
    }

    frameSelected(viewport) {
        viewport.frame();
    }
}
