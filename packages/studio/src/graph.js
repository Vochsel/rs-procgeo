// Document model helpers. The xyflow `nodes`/`edges` are the source of truth;
// everything else (cook DAG, code) is derived from them.
//
// Node shape (xyflow):  { id, type: 'sop', position, data: { sop, params } }
// Edge shape (xyflow):  { id, source, target, targetHandle: 'in-<i>' }

import { SOPS, defaultParams } from './sops.js';

/** Generate a unique node id like `box1`, `box2`, … */
export function uniqueId(sopType, nodes) {
  const used = new Set(nodes.map((n) => n.id));
  let i = 1;
  while (used.has(`${sopType}${i}`)) i += 1;
  return `${sopType}${i}`;
}

/** Ordered input node-ids for a target node, indexed by its `in-<i>` handles. */
export function inputsOf(nodeId, edges) {
  const incoming = edges.filter((e) => e.target === nodeId);
  const byIndex = [];
  for (const e of incoming) {
    const idx = handleIndex(e.targetHandle);
    byIndex[idx] = e.source;
  }
  // Collapse holes (e.g. only `in-1` connected) into a dense ordered list.
  return byIndex.filter((v) => v !== undefined);
}

export function handleIndex(handle) {
  if (!handle) return 0;
  const m = /in-(\d+)/.exec(handle);
  return m ? Number(m[1]) : 0;
}

/**
 * Build the cook DAG the engine understands:
 *   { nodes: [{ id, type, params, inputs: [ids] }], output }
 */
export function buildCookDag(nodes, edges, outputId) {
  const dagNodes = nodes.map((n) => ({
    id: n.id,
    type: n.data.sop,
    params: prunedParams(n.data.sop, n.data.params),
    inputs: inputsOf(n.id, edges),
  }));
  const output = outputId || (nodes.length ? nodes[nodes.length - 1].id : null);
  return { nodes: dagNodes, output };
}

/** Drop params equal to their default so the cook payload stays minimal. */
export function prunedParams(sopType, params) {
  const sop = SOPS[sopType];
  if (!sop) return {};
  const defs = defaultParams(sopType);
  const out = {};
  for (const p of sop.params) {
    const v = params?.[p.key];
    if (v === undefined) continue;
    if (JSON.stringify(v) !== JSON.stringify(defs[p.key])) out[p.key] = v;
  }
  return out;
}

/**
 * Layered auto-layout. `direction` is 'LR' (left→right) or 'TB' (top→bottom).
 * Returns { [id]: { x, y } }. Accepts nodes as objects with `.id` or strings.
 */
export function autoLayout(nodes, edges, direction = 'LR') {
  const ids = nodes.map((n) => (typeof n === 'string' ? n : n.id));
  const idSet = new Set(ids);
  const depth = new Map();
  const depthOf = (id, seen = new Set()) => {
    if (depth.has(id)) return depth.get(id);
    if (seen.has(id) || !idSet.has(id)) return 0;
    seen.add(id);
    const ins = inputsOf(id, edges);
    const d = ins.length ? 1 + Math.max(...ins.map((i) => depthOf(i, seen))) : 0;
    depth.set(id, d);
    return d;
  };
  ids.forEach((id) => depthOf(id));

  const rowByCol = new Map();
  const pos = {};
  const COL = 240;
  const ROW = 130;
  for (const id of ids) {
    const col = depth.get(id) || 0;
    const row = rowByCol.get(col) || 0;
    rowByCol.set(col, row + 1);
    pos[id] =
      direction === 'TB' ? { x: row * 200, y: col * ROW } : { x: col * COL, y: row * ROW };
  }
  return pos;
}

/** Pick a sensible default output node: a node nothing else consumes. */
export function inferOutput(nodes, edges) {
  if (!nodes.length) return null;
  const consumed = new Set(edges.map((e) => e.source));
  const sinks = nodes.filter((n) => !consumed.has(n.id));
  return (sinks[sinks.length - 1] || nodes[nodes.length - 1]).id;
}
