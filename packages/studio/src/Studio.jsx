import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  ReactFlowProvider,
  applyNodeChanges,
  applyEdgeChanges,
  addEdge,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { NodeCanvas } from './NodeCanvas.jsx';
import { CodeEditor } from './CodeEditor.jsx';
import { Viewport } from './Viewport.jsx';
import { Inspector } from './Inspector.jsx';
import { SOPS, sopList, defaultParams } from './sops.js';
import { buildCookDag, uniqueId, inputsOf, autoLayout } from './graph.js';
import { graphToCode, codeToGraph } from './codegen.js';

const DEFAULT_CODE = `const box1 = box({ size: [2, 2, 2] })
const subdivide1 = subdivide(box1, { depth: 2 })
const normal1 = normal(subdivide1)
return normal1
`;

/**
 * The full ProcGeo studio. Engine-agnostic: `engine.cookGraph(dag)` returns
 * render buffers. Used by both the web (WASM) and desktop (native) apps.
 */
export function Studio({ engine }) {
  const seed = useMemo(() => safeParse(DEFAULT_CODE), []);
  const [nodes, setNodes] = useState(seed.nodes);
  const [edges, setEdges] = useState(seed.edges);
  const [outputId, setOutputId] = useState(seed.outputId);
  const [code, setCode] = useState(DEFAULT_CODE);
  const [tab, setTab] = useState('nodes');
  const [direction, setDirection] = useState('LR');
  const [viewMode, setViewMode] = useState('shaded_wire');
  const [status, setStatus] = useState('');
  const [error, setError] = useState(null);

  const viewportRef = useRef(null);
  const suppressCodeRegen = useRef(false); // set when a graph change came FROM code
  const cookTimer = useRef(null);
  const didFit = useRef(false);
  const directionRef = useRef(direction);
  directionRef.current = direction;

  const selected = nodes.find((n) => n.selected) || null;

  // Inject a dynamic input-port count into variadic nodes (e.g. merge) so the
  // canvas shows connected ports + one spare. Canonical state stays untouched.
  const displayNodes = useMemo(
    () =>
      nodes.map((n) => {
        const def = SOPS[n.data.sop];
        if (!def?.variadic) return n;
        const connected = inputsOf(n.id, edges).length;
        return { ...n, data: { ...n.data, _ports: Math.max(def.inputs || 1, connected + 1) } };
      }),
    [nodes, edges],
  );

  // Re-run auto-layout when the flow direction is toggled.
  const prevDir = useRef(direction);
  useEffect(() => {
    if (prevDir.current === direction) return;
    prevDir.current = direction;
    setNodes((ns) => {
      const pos = autoLayout(ns, edges, direction);
      return ns.map((n) => ({ ...n, position: pos[n.id] || n.position }));
    });
  }, [direction, edges]);

  // ── Regenerate code from the graph (unless the graph change came from code) ──
  useEffect(() => {
    if (suppressCodeRegen.current) {
      suppressCodeRegen.current = false;
    } else {
      setCode(graphToCode(nodes, edges, outputId));
    }
  }, [nodes, edges, outputId]);

  // ── Cook (debounced) on any graph change ──
  useEffect(() => {
    clearTimeout(cookTimer.current);
    cookTimer.current = setTimeout(async () => {
      try {
        const dag = buildCookDag(nodes, edges, outputId);
        if (!dag.nodes.length) {
          setStatus('empty graph');
          return;
        }
        const t0 = performance.now();
        const buffers = await engine.cookGraph(dag);
        const ms = (performance.now() - t0).toFixed(1);
        viewportRef.current?.setBuffers(buffers);
        if (!didFit.current) {
          didFit.current = true;
          requestAnimationFrame(() => viewportRef.current?.fit());
        }
        setError(null);
        setStatus(`${buffers.numPoints} pts · ${buffers.numPrims} prims · cooked in ${ms} ms`);
      } catch (e) {
        setError(String(e?.message || e));
      }
    }, 250);
    return () => clearTimeout(cookTimer.current);
  }, [nodes, edges, outputId, engine]);

  // ── Graph editing ──
  const onNodesChange = useCallback((changes) => setNodes((ns) => applyNodeChanges(changes, ns)), []);
  const onEdgesChange = useCallback((changes) => setEdges((es) => applyEdgeChanges(changes, es)), []);

  const onConnect = useCallback((conn) => {
    setEdges((es) => {
      // Enforce one edge per target input handle.
      const cleaned = es.filter(
        (e) => !(e.target === conn.target && e.targetHandle === conn.targetHandle),
      );
      return addEdge({ ...conn, id: `${conn.source}->${conn.target}:${conn.targetHandle}` }, cleaned);
    });
  }, []);

  const changeParams = useCallback((id, params) => {
    setNodes((ns) => ns.map((n) => (n.id === id ? { ...n, data: { ...n.data, params } } : n)));
  }, []);

  const addNode = useCallback(
    (type) => {
      setNodes((ns) => {
        const id = uniqueId(type, ns);
        const node = {
          id,
          type: 'sop',
          position: { x: 40 + ns.length * 24, y: 40 + ns.length * 24 },
          selected: true,
          data: { sop: type, params: defaultParams(type) },
        };
        return ns.map((n) => ({ ...n, selected: false })).concat(node);
      });
    },
    [],
  );

  const deleteNode = useCallback(
    (id) => {
      setNodes((ns) => ns.filter((n) => n.id !== id));
      setEdges((es) => es.filter((e) => e.source !== id && e.target !== id));
      setOutputId((cur) => (cur === id ? null : cur));
    },
    [],
  );

  // ── Code editing → graph (debounced parse) ──
  const codeTimer = useRef(null);
  const onCodeChange = useCallback((text) => {
    setCode(text);
    clearTimeout(codeTimer.current);
    codeTimer.current = setTimeout(() => {
      try {
        const g = codeToGraph(text, directionRef.current);
        suppressCodeRegen.current = true;
        setNodes((prev) => mergePositions(g.nodes, prev));
        setEdges(g.edges);
        setOutputId(g.outputId);
        setError(null);
      } catch (e) {
        setError(`code: ${e.message}`);
      }
    }, 400);
  }, []);

  const sops = useMemo(() => sopList(), []);

  return (
    <div className="pg-studio">
      <div className="pg-toolbar">
        <div className="pg-tabs">
          <button className={tab === 'nodes' ? 'active' : ''} onClick={() => setTab('nodes')}>
            Nodes
          </button>
          <button className={tab === 'code' ? 'active' : ''} onClick={() => setTab('code')}>
            Code
          </button>
        </div>
        <AddNodeMenu sops={sops} onAdd={addNode} />
        <button
          className="pg-dir"
          title="Toggle node layout direction"
          onClick={() => setDirection((d) => (d === 'LR' ? 'TB' : 'LR'))}
        >
          {direction === 'LR' ? 'Layout: ⇥ horizontal' : 'Layout: ⤓ vertical'}
        </button>
        <div className="pg-spacer" />
        <div className="pg-viewmodes">
          {['shaded', 'shaded_wire', 'wire'].map((m) => (
            <button key={m} className={viewMode === m ? 'active' : ''} onClick={() => setViewMode(m)}>
              {m === 'shaded_wire' ? 'both' : m}
            </button>
          ))}
          <button onClick={() => viewportRef.current?.fit()}>frame</button>
        </div>
      </div>

      <div className="pg-body">
        <div className="pg-left">
          <div className="pg-editor" style={{ display: tab === 'nodes' ? 'block' : 'none' }}>
            <ReactFlowProvider>
              <NodeCanvas
                nodes={displayNodes}
                edges={edges}
                direction={direction}
                onNodesChange={onNodesChange}
                onEdgesChange={onEdgesChange}
                onConnect={onConnect}
              />
            </ReactFlowProvider>
          </div>
          {tab === 'code' && <CodeEditor value={code} onChange={onCodeChange} />}
          <Inspector
            node={selected}
            isOutput={selected?.id === outputId}
            onChangeParams={changeParams}
            onSetOutput={setOutputId}
            onDelete={deleteNode}
          />
        </div>

        <div className="pg-right">
          <Viewport ref={viewportRef} viewMode={viewMode} />
          <div className={`pg-status ${error ? 'error' : ''}`}>{error || status}</div>
        </div>
      </div>
    </div>
  );
}

function AddNodeMenu({ sops, onAdd }) {
  return (
    <select
      className="pg-add"
      value=""
      onChange={(e) => {
        if (e.target.value) onAdd(e.target.value);
        e.target.value = '';
      }}
    >
      <option value="">+ Add node…</option>
      {['create', 'filter', 'combine'].map((cat) => (
        <optgroup key={cat} label={cat}>
          {sops
            .filter((s) => s.category === cat)
            .map((s) => (
              <option key={s.type} value={s.type}>
                {s.label}
              </option>
            ))}
        </optgroup>
      ))}
    </select>
  );
}

function safeParse(code) {
  try {
    return codeToGraph(code);
  } catch {
    return { nodes: [], edges: [], outputId: null };
  }
}

// Keep existing on-canvas positions when re-parsing code, so typing params
// doesn't make nodes jump back to the auto-layout.
function mergePositions(newNodes, prevNodes) {
  const prev = new Map(prevNodes.map((n) => [n.id, n]));
  return newNodes.map((n) => {
    const old = prev.get(n.id);
    return old ? { ...n, position: old.position, selected: old.selected } : n;
  });
}
