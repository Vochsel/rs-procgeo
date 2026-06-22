// ─────────────────────────────────────────────────────────────────────────────
// ParamsPanel — edits the parameters of the selected node.
// Renders an input per param type (float/int/bool/vec/enum/string/color) and
// writes changes back through graph.setParam, which triggers a recook.
// ─────────────────────────────────────────────────────────────────────────────

import { getNodeDef } from './registry.js';

export class ParamsPanel {
    constructor(root, graph) {
        this.root = root;
        this.graph = graph;
        this.node = null;
        this.showEmpty();
    }

    showEmpty() {
        this.node = null;
        this.root.innerHTML = '<div class="pp-empty">Select a node to edit its parameters.<br><br>Right-click or press <kbd>Tab</kbd> to add a node.</div>';
    }

    show(node) {
        this.node = node;
        if (!node) return this.showEmpty();
        const def = getNodeDef(node.type);
        this.root.innerHTML = '';

        // Header: node name + type + display button.
        const header = document.createElement('div');
        header.className = 'pp-header';

        const nameInput = document.createElement('input');
        nameInput.className = 'pp-name';
        nameInput.value = node.name;
        nameInput.addEventListener('input', () => {
            node.name = nameInput.value || def.label;
            this.graph.emit();
        });
        header.appendChild(nameInput);

        const typeTag = document.createElement('span');
        typeTag.className = 'pp-type';
        typeTag.textContent = `${def.category} · ${def.label}`;
        header.appendChild(typeTag);

        const displayBtn = document.createElement('button');
        displayBtn.className = 'pp-display-btn';
        const isDisplayed = this.graph.displayNodeId === node.id;
        displayBtn.textContent = isDisplayed ? '● Displayed' : 'Set Display';
        displayBtn.classList.toggle('active', isDisplayed);
        displayBtn.addEventListener('click', () => this.graph.setDisplay(node.id));
        header.appendChild(displayBtn);

        this.root.appendChild(header);

        if (!def.params.length) {
            const none = document.createElement('div');
            none.className = 'pp-empty';
            none.textContent = 'This node has no parameters.';
            this.root.appendChild(none);
            return;
        }

        const body = document.createElement('div');
        body.className = 'pp-body';
        for (const p of def.params) {
            body.appendChild(this._renderParam(node, p));
        }
        this.root.appendChild(body);
    }

    _renderParam(node, p) {
        const row = document.createElement('div');
        row.className = 'pp-row';

        const label = document.createElement('label');
        label.className = 'pp-label';
        label.textContent = p.label;
        row.appendChild(label);

        const control = document.createElement('div');
        control.className = 'pp-control';
        control.appendChild(this._buildControl(node, p));
        row.appendChild(control);
        return row;
    }

    _set(node, name, value) {
        this.graph.setParam(node.id, name, value);
    }

    _buildControl(node, p) {
        const val = node.params[p.name];
        switch (p.type) {
            case 'float':
            case 'int':
                return this._numberInput(node, p, val);
            case 'bool': {
                const cb = document.createElement('input');
                cb.type = 'checkbox';
                cb.checked = !!val;
                cb.className = 'pp-checkbox';
                cb.addEventListener('change', () => this._set(node, p.name, cb.checked));
                return cb;
            }
            case 'enum': {
                const sel = document.createElement('select');
                sel.className = 'pp-select';
                for (const opt of p.options) {
                    const o = document.createElement('option');
                    o.value = opt;
                    o.textContent = opt;
                    if (opt === val) o.selected = true;
                    sel.appendChild(o);
                }
                sel.addEventListener('change', () => this._set(node, p.name, sel.value));
                return sel;
            }
            case 'string': {
                const inp = document.createElement('input');
                inp.type = 'text';
                inp.className = 'pp-text';
                inp.value = val ?? '';
                inp.addEventListener('input', () => this._set(node, p.name, inp.value));
                return inp;
            }
            case 'color': {
                const wrap = document.createElement('div');
                wrap.className = 'pp-color';
                const picker = document.createElement('input');
                picker.type = 'color';
                picker.value = rgbToHex(val);
                picker.addEventListener('input', () => this._set(node, p.name, hexToRgb(picker.value)));
                wrap.appendChild(picker);
                return wrap;
            }
            case 'vec2':
            case 'vec3': {
                const n = p.type === 'vec2' ? 2 : 3;
                const wrap = document.createElement('div');
                wrap.className = 'pp-vec';
                const labels = ['x', 'y', 'z'];
                for (let i = 0; i < n; i++) {
                    const inp = document.createElement('input');
                    inp.type = 'number';
                    inp.step = 'any';
                    inp.className = 'pp-num pp-vec-comp';
                    inp.title = labels[i];
                    inp.value = fmt(val[i]);
                    inp.addEventListener('input', () => {
                        const next = (node.params[p.name] || []).slice();
                        next[i] = parseFloat(inp.value);
                        if (Number.isNaN(next[i])) next[i] = 0;
                        this._set(node, p.name, next);
                    });
                    wrap.appendChild(inp);
                }
                return wrap;
            }
            default:
                return document.createTextNode(String(val));
        }
    }

    // Number field with optional drag-to-scrub and slider when bounded.
    _numberInput(node, p, val) {
        const wrap = document.createElement('div');
        wrap.className = 'pp-numwrap';

        const inp = document.createElement('input');
        inp.type = 'number';
        inp.step = p.type === 'int' ? '1' : 'any';
        inp.className = 'pp-num';
        if (p.min !== undefined) inp.min = p.min;
        if (p.max !== undefined) inp.max = p.max;
        inp.value = fmt(val);

        const commit = (v) => {
            let num = p.type === 'int' ? Math.round(v) : v;
            if (Number.isNaN(num)) return;
            if (p.min !== undefined) num = Math.max(p.min, num);
            if (p.max !== undefined) num = Math.min(p.max, num);
            this._set(node, p.name, num);
        };

        inp.addEventListener('input', () => commit(parseFloat(inp.value)));
        wrap.appendChild(inp);

        // Slider when both bounds are known.
        if (p.min !== undefined && p.max !== undefined) {
            const slider = document.createElement('input');
            slider.type = 'range';
            slider.className = 'pp-slider';
            slider.min = p.min;
            slider.max = p.max;
            slider.step = p.type === 'int' ? 1 : (p.max - p.min) / 200;
            slider.value = val;
            slider.addEventListener('input', () => {
                inp.value = slider.value;
                commit(parseFloat(slider.value));
            });
            inp.addEventListener('input', () => { slider.value = inp.value; });
            wrap.appendChild(slider);
        }
        return wrap;
    }
}

function fmt(v) {
    if (typeof v !== 'number') return v;
    return Number.isInteger(v) ? String(v) : String(Math.round(v * 1e6) / 1e6);
}

function rgbToHex(rgb) {
    const c = (x) => Math.max(0, Math.min(255, Math.round((x ?? 0) * 255))).toString(16).padStart(2, '0');
    return `#${c(rgb[0])}${c(rgb[1])}${c(rgb[2])}`;
}

function hexToRgb(hex) {
    const n = parseInt(hex.slice(1), 16);
    return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}
