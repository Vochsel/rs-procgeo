// Round-trip between the node graph and a small, readable procgeo DSL.
//
//   const box1 = box({ size: [2, 2, 2] })
//   const subdivide1 = subdivide(box1, { depth: 2 })
//   const normal1 = normal(subdivide1)
//   return normal1
//
// Creation SOPs take just params; filters take (input, params); combine SOPs
// take (inputA, inputB, params). A trailing `return <id>` marks the output.

import { parse } from 'acorn';
import { SOPS, defaultParams } from './sops.js';
import { inputsOf, prunedParams, autoLayout } from './graph.js';

// ── Graph → Code ─────────────────────────────────────────────────────────

export function graphToCode(nodes, edges, outputId) {
  if (!nodes.length) return '// add nodes to begin\n';
  const order = topoSort(nodes, edges);
  const lines = [];
  for (const node of order) {
    const inputs = inputsOf(node.id, edges);
    const params = prunedParams(node.data.sop, node.data.params);
    const args = [...inputs];
    if (Object.keys(params).length) args.push(toLiteral(params));
    lines.push(`const ${node.id} = ${node.data.sop}(${args.join(', ')})`);
  }
  const out = outputId || order[order.length - 1].id;
  lines.push(`return ${out}`);
  return lines.join('\n') + '\n';
}

function toLiteral(v) {
  if (Array.isArray(v)) return `[${v.map(toLiteral).join(', ')}]`;
  if (v === null) return 'null';
  if (typeof v === 'string') return JSON.stringify(v);
  if (typeof v === 'number' || typeof v === 'boolean') return String(v);
  if (typeof v === 'object') {
    const body = Object.entries(v)
      .map(([k, val]) => `${k}: ${toLiteral(val)}`)
      .join(', ');
    return `{ ${body} }`;
  }
  return String(v);
}

// ── Code → Graph ─────────────────────────────────────────────────────────

export function codeToGraph(code, direction = 'LR') {
  const program = parse(code, { ecmaVersion: 2022, allowReturnOutsideFunction: true });

  const parsed = []; // { id, sop, inputs: [ids], params }
  let outputId = null;

  for (const stmt of program.body) {
    if (stmt.type === 'VariableDeclaration') {
      const decl = stmt.declarations[0];
      if (!decl || decl.id.type !== 'Identifier' || !decl.init) continue;
      if (decl.init.type !== 'CallExpression' || decl.init.callee.type !== 'Identifier') {
        throw new Error(`'${decl.id.name}' must be assigned a SOP call`);
      }
      const sop = decl.init.callee.name;
      if (!SOPS[sop]) throw new Error(`unknown SOP '${sop}'`);

      const inputs = [];
      let params = {};
      for (const arg of decl.init.arguments) {
        if (arg.type === 'Identifier') inputs.push(arg.name);
        else if (arg.type === 'ObjectExpression') params = evalLiteral(arg);
        else throw new Error(`unexpected argument in '${sop}(...)'`);
      }
      parsed.push({ id: decl.id.name, sop, inputs, params });
    } else if (stmt.type === 'ReturnStatement' && stmt.argument?.type === 'Identifier') {
      outputId = stmt.argument.name;
    }
  }

  // Validate referenced inputs exist.
  const ids = new Set(parsed.map((p) => p.id));
  for (const p of parsed) {
    for (const inp of p.inputs) {
      if (!ids.has(inp)) throw new Error(`'${p.id}' references unknown node '${inp}'`);
    }
  }

  const edges = [];
  for (const p of parsed) {
    p.inputs.forEach((src, i) => {
      edges.push({
        id: `${src}->${p.id}:${i}`,
        source: src,
        target: p.id,
        targetHandle: `in-${i}`,
      });
    });
  }

  const positions = autoLayout(parsed, edges, direction);
  const nodes = parsed.map((p) => ({
    id: p.id,
    type: 'sop',
    position: positions[p.id],
    data: { sop: p.sop, params: { ...defaultParams(p.sop), ...p.params } },
  }));

  if (!outputId && parsed.length) outputId = parsed[parsed.length - 1].id;
  return { nodes, edges, outputId };
}

function evalLiteral(node) {
  switch (node.type) {
    case 'Literal':
      return node.value;
    case 'ArrayExpression':
      return node.elements.map(evalLiteral);
    case 'ObjectExpression': {
      const obj = {};
      for (const prop of node.properties) {
        const key = prop.key.type === 'Identifier' ? prop.key.name : prop.key.value;
        obj[key] = evalLiteral(prop.value);
      }
      return obj;
    }
    case 'UnaryExpression':
      if (node.operator === '-') return -evalLiteral(node.argument);
      if (node.operator === '+') return +evalLiteral(node.argument);
      break;
  }
  throw new Error(`unsupported literal: ${node.type}`);
}

// ── Shared graph utilities ───────────────────────────────────────────────

/** Topological order of xyflow nodes by their input edges. */
export function topoSort(nodes, edges) {
  const byId = new Map(nodes.map((n) => [n.id, n]));
  const mark = new Map();
  const out = [];
  const visit = (id) => {
    const m = mark.get(id);
    if (m === 'done') return;
    if (m === 'temp') throw new Error(`cycle through '${id}'`);
    const node = byId.get(id);
    if (!node) return;
    mark.set(id, 'temp');
    for (const src of inputsOf(id, edges)) visit(src);
    mark.set(id, 'done');
    out.push(node);
  };
  for (const n of nodes) visit(n.id);
  return out;
}
