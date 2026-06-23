import { SOPS } from './sops.js';

/** Param editor for the selected node. */
export function Inspector({ node, isOutput, onChangeParams, onSetOutput, onDelete }) {
  if (!node) {
    return <div className="pg-inspector pg-inspector-empty">Select a node to edit its parameters.</div>;
  }
  const def = SOPS[node.data.sop] || { label: node.data.sop, params: [] };
  const params = node.data.params || {};

  const set = (key, value) => onChangeParams(node.id, { ...params, [key]: value });

  return (
    <div className="pg-inspector">
      <div className="pg-inspector-head">
        <span className="pg-inspector-title">{def.label}</span>
        <span className="pg-inspector-sub">{node.id}</span>
      </div>

      {def.params.length === 0 && <div className="pg-inspector-empty">No parameters.</div>}

      {def.params.map((p) => (
        <label key={p.key} className="pg-field">
          <span className="pg-field-label">{p.key}</span>
          {renderControl(p, params[p.key], (v) => set(p.key, v))}
        </label>
      ))}

      <div className="pg-inspector-actions">
        <button className={`pg-btn ${isOutput ? 'active' : ''}`} onClick={() => onSetOutput(node.id)}>
          {isOutput ? '★ Output' : 'Set as output'}
        </button>
        <button className="pg-btn pg-btn-danger" onClick={() => onDelete(node.id)}>
          Delete
        </button>
      </div>
    </div>
  );
}

function renderControl(p, value, onChange) {
  switch (p.type) {
    case 'bool':
      return <input type="checkbox" checked={!!value} onChange={(e) => onChange(e.target.checked)} />;
    case 'enum':
      return (
        <select value={value} onChange={(e) => onChange(e.target.value)}>
          {p.options.map((o) => (
            <option key={o} value={o}>
              {o}
            </option>
          ))}
        </select>
      );
    case 'vec2':
    case 'vec3': {
      const n = p.type === 'vec2' ? 2 : 3;
      const arr = Array.isArray(value) ? value : Array(n).fill(0);
      return (
        <span className="pg-vec">
          {Array.from({ length: n }).map((_, i) => (
            <input
              key={i}
              type="number"
              step="0.1"
              value={arr[i] ?? 0}
              onChange={(e) => {
                const next = [...arr];
                next[i] = numOr(e.target.value, arr[i] ?? 0);
                onChange(next);
              }}
            />
          ))}
        </span>
      );
    }
    case 'int':
      return (
        <input
          type="number"
          step="1"
          value={value ?? 0}
          onChange={(e) => onChange(Math.round(numOr(e.target.value, value ?? 0)))}
        />
      );
    default:
      return (
        <input
          type="number"
          step="0.1"
          value={value ?? 0}
          onChange={(e) => onChange(numOr(e.target.value, value ?? 0))}
        />
      );
  }
}

function numOr(s, fallback) {
  const n = parseFloat(s);
  return Number.isFinite(n) ? n : fallback;
}
