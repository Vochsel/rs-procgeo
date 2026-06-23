// Catalog of SOPs exposed as nodes. Param `key`s MUST match the Rust struct
// serde field names — the registry deserializes params JSON straight into the
// Params struct (merging over serialized defaults), so partial objects are fine.
//
// Param types: 'number' | 'int' | 'bool' | 'vec2' | 'vec3' | 'enum'.

export const SOPS = {
  // ── Creation (0 inputs) ────────────────────────────────────────────────
  box: {
    label: 'Box',
    category: 'create',
    inputs: 0,
    params: [
      { key: 'size', type: 'vec3', default: [1, 1, 1] },
      { key: 'center', type: 'vec3', default: [0, 0, 0] },
    ],
  },
  sphere: {
    label: 'Sphere',
    category: 'create',
    inputs: 0,
    params: [
      { key: 'radius', type: 'vec3', default: [1, 1, 1] },
      { key: 'rows', type: 'int', default: 16 },
      { key: 'cols', type: 'int', default: 32 },
    ],
  },
  grid: {
    label: 'Grid',
    category: 'create',
    inputs: 0,
    params: [
      { key: 'size', type: 'vec2', default: [10, 10] },
      { key: 'rows', type: 'int', default: 10 },
      { key: 'cols', type: 'int', default: 10 },
    ],
  },
  torus: {
    label: 'Torus',
    category: 'create',
    inputs: 0,
    params: [
      { key: 'radius_outer', type: 'number', default: 1 },
      { key: 'radius_inner', type: 'number', default: 0.3 },
      { key: 'rows', type: 'int', default: 16 },
      { key: 'cols', type: 'int', default: 24 },
    ],
  },
  tube: {
    label: 'Tube',
    category: 'create',
    inputs: 0,
    params: [
      { key: 'radius_bottom', type: 'number', default: 1 },
      { key: 'radius_top', type: 'number', default: 1 },
      { key: 'height', type: 'number', default: 2 },
      { key: 'cols', type: 'int', default: 16 },
    ],
  },
  circle: {
    label: 'Circle',
    category: 'create',
    inputs: 0,
    params: [
      { key: 'radius', type: 'number', default: 1 },
      { key: 'divisions', type: 'int', default: 16 },
    ],
  },

  // ── Filters (1 input) ──────────────────────────────────────────────────
  subdivide: {
    label: 'Subdivide',
    category: 'filter',
    inputs: 1,
    params: [
      { key: 'depth', type: 'int', default: 1 },
      { key: 'mode', type: 'enum', options: ['Linear', 'CatmullClark'], default: 'Linear' },
    ],
  },
  transform: {
    label: 'Transform',
    category: 'filter',
    inputs: 1,
    params: [
      { key: 'translate', type: 'vec3', default: [0, 0, 0] },
      { key: 'rotate', type: 'vec3', default: [0, 0, 0] },
      { key: 'scale', type: 'vec3', default: [1, 1, 1] },
    ],
  },
  smooth: {
    label: 'Smooth',
    category: 'filter',
    inputs: 1,
    params: [
      { key: 'iterations', type: 'int', default: 3 },
      { key: 'strength', type: 'number', default: 0.5 },
    ],
  },
  polyextrude: {
    label: 'PolyExtrude',
    category: 'filter',
    inputs: 1,
    params: [
      { key: 'distance', type: 'number', default: 0.2 },
      { key: 'inset', type: 'number', default: 0 },
    ],
  },
  scatter: {
    label: 'Scatter',
    category: 'filter',
    inputs: 1,
    params: [
      { key: 'count', type: 'int', default: 100 },
      { key: 'seed', type: 'int', default: 0 },
    ],
  },
  normal: {
    label: 'Normal',
    category: 'filter',
    inputs: 1,
    params: [],
  },

  // ── Combine (multiple inputs) ──────────────────────────────────────────
  copy_to_points: {
    label: 'CopyToPoints',
    category: 'combine',
    inputs: 2, // [0] geometry to copy, [1] target points
    params: [],
  },
  merge: {
    label: 'Merge',
    category: 'combine',
    inputs: 2, // minimum ports shown; grows as inputs connect
    variadic: true,
    params: [],
  },
};

/** Number of input ports to show for a node, given how many are connected. */
export function portCount(type, connected = 0) {
  const sop = SOPS[type];
  if (!sop) return 0;
  if (sop.variadic) return Math.max(sop.inputs || 1, connected + 1);
  return sop.inputs || 0;
}

/** Default param object for a SOP type (clone so callers can mutate freely). */
export function defaultParams(type) {
  const sop = SOPS[type];
  if (!sop) return {};
  const out = {};
  for (const p of sop.params) {
    out[p.key] = Array.isArray(p.default) ? [...p.default] : p.default;
  }
  return out;
}

export function sopList() {
  return Object.entries(SOPS).map(([type, def]) => ({ type, ...def }));
}
