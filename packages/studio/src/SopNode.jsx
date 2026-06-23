import { Handle, Position } from '@xyflow/react';
import { SOPS } from './sops.js';

/** A single SOP node: input handles on the left, one output on the right. */
export function SopNode({ data, selected }) {
  const def = SOPS[data.sop] || { label: data.sop, inputs: 0, params: [] };
  const inputCount = def.inputs;

  const summary = (def.params || [])
    .filter((p) => data.params?.[p.key] !== undefined)
    .map((p) => `${p.key}=${fmt(data.params[p.key])}`)
    .slice(0, 3)
    .join('  ');

  return (
    <div className={`pg-node pg-node-${def.category || 'filter'} ${selected ? 'selected' : ''}`}>
      {Array.from({ length: inputCount }).map((_, i) => (
        <Handle
          key={i}
          type="target"
          position={Position.Left}
          id={`in-${i}`}
          style={{ top: 24 + i * 16 }}
        />
      ))}
      <div className="pg-node-title">{def.label}</div>
      <div className="pg-node-id">{/* id shown by xyflow selection; keep params */}{summary}</div>
      <Handle type="source" position={Position.Right} id="out" />
    </div>
  );
}

function fmt(v) {
  if (Array.isArray(v)) return `[${v.join(',')}]`;
  return String(v);
}
