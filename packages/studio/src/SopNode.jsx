import { useContext } from 'react';
import { Handle, Position } from '@xyflow/react';
import { SOPS, portCount } from './sops.js';
import { DirectionContext } from './layoutContext.js';

/** A SOP node. Input handles scale with the SOP's arity; orientation follows
 *  the graph direction (left/right for LR, top/bottom for TB). */
export function SopNode({ data, selected }) {
  const direction = useContext(DirectionContext);
  const vertical = direction === 'TB';
  const def = SOPS[data.sop] || { label: data.sop, inputs: 0, params: [] };

  // `_ports` is injected by Studio for variadic nodes; otherwise use arity.
  const ports = data._ports ?? portCount(data.sop);

  const summary = (def.params || [])
    .filter((p) => data.params?.[p.key] !== undefined)
    .map((p) => `${p.key}=${fmt(data.params[p.key])}`)
    .slice(0, 3)
    .join('  ');

  const targetPos = vertical ? Position.Top : Position.Left;
  const sourcePos = vertical ? Position.Bottom : Position.Right;

  return (
    <div className={`pg-node pg-node-${def.category || 'filter'} ${selected ? 'selected' : ''}`}>
      {Array.from({ length: ports }).map((_, i) => (
        <Handle
          key={i}
          type="target"
          position={targetPos}
          id={`in-${i}`}
          style={handleStyle(ports, i, vertical)}
        />
      ))}
      <div className="pg-node-title">{def.label}</div>
      <div className="pg-node-id">{summary}</div>
      <Handle type="source" position={sourcePos} id="out" />
    </div>
  );
}

function handleStyle(ports, i, vertical) {
  if (ports <= 1) return undefined;
  // Spread multiple ports evenly along the input edge.
  const pct = `${(100 * (i + 1)) / (ports + 1)}%`;
  return vertical ? { left: pct } : { top: pct };
}

function fmt(v) {
  if (Array.isArray(v)) return `[${v.join(',')}]`;
  return String(v);
}
