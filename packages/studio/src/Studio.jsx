import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  ReactFlowProvider,
  applyNodeChanges,
  applyEdgeChanges,
  addEdge,
  reconnectEdge,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { NodeCanvas } from './NodeCanvas.jsx';
import { CodeEditor } from './CodeEditor.jsx';
import { Viewport } from './Viewport.jsx';
import { Inspector } from './Inspector.jsx';
import { SOPS, sopList, defaultParams } from './sops.js';
import { buildCookDag, uniqueId, inputsOf, autoLayout } from './graph.js';
import { graphToCode, codeToGraph } from './codegen.js';
import { TEMPLATES } from './templates.js';
import { serializeDoc, parseDoc, browserHost } from './doc.js';

const DEFAULT_CODE = `const box1 = box({ size: [2, 2, 2] })
const subdivide1 = subdivide(box1, { depth: 2 })
const normal1 = normal(subdivide1)
return normal1
`;

/**
 * The full ProcGeo studio. Engine-agnostic: `engine.cookGraph(dag)` returns
 * render buffers. Used by both the web (WASM) and desktop (native) apps.
 */
export function Studio({ engine, host = browserHost }) {
  const seed = useMemo(() => safeParse(DEFAULT_CODE), []);
  const [nodes, setNodes] = useState(seed.nodes);
  const [edges, setEdges] = useState(seed.edges);
  const [outputId, setOutputId] = useState(seed.outputId);
  const [code, setCode] = useState(DEFAULT_CODE);
  const [tab, setTab] = useState('nodes');
  const [direction, setDirection] = useState('LR');
  const [viewMode, setViewMode] = useState('shaded_wire');
  const [showSettings, setShowSettings] = useState(false);
  const [showFileMenu, setShowFileMenu] = useState(false);
  const [showTemplates, setShowTemplates] = useState(false);
  const [leftWidth, setLeftWidth] = useState(560);
  const [docName, setDocName] = useState('untitled');
  const [docPath, setDocPath] = useState(null);
  const [status, setStatus] = useState('');
  const [error, setError] = useState(null);

  const viewportRef = useRef(null);
  const suppressCodeRegen = useRef(false); // set when a graph change came FROM code
  const cookTimer = useRef(null);
  const didFit = useRef(false);
  const lastDag = useRef(''); // skip cooks when geometry-affecting inputs are unchanged
  const directionRef = useRef(direction);
  directionRef.current = direction;

  const selected = nodes.find((n) => n.selected) || null;
  const sops = useMemo(() => sopList(), []);

  // Inject a dynamic input-port count into variadic nodes (e.g. merge).
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

  // ── Regenerate code from the graph (unless the change came from code) ──
  useEffect(() => {
    if (suppressCodeRegen.current) {
      suppressCodeRegen.current = false;
    } else {
      setCode(graphToCode(nodes, edges, outputId));
    }
  }, [nodes, edges, outputId]);

  // ── Cook (debounced) only when the geometry-affecting graph changes ──
  // The cook DAG excludes node positions/selection, so dragging nodes around
  // never triggers a recook.
  useEffect(() => {
    const dag = buildCookDag(nodes, edges, outputId);
    if (!dag.nodes.length) {
      setStatus('empty graph');
      return;
    }
    const key = JSON.stringify(dag);
    if (key === lastDag.current) return;
    lastDag.current = key;

    clearTimeout(cookTimer.current);
    cookTimer.current = setTimeout(async () => {
      try {
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
    }, 40);
    return () => clearTimeout(cookTimer.current);
  }, [nodes, edges, outputId, engine]);

  // ── Re-run auto-layout when the flow direction is toggled ──
  const prevDir = useRef(direction);
  useEffect(() => {
    if (prevDir.current === direction) return;
    prevDir.current = direction;
    setNodes((ns) => {
      const pos = autoLayout(ns, edges, direction);
      return ns.map((n) => ({ ...n, position: pos[n.id] || n.position }));
    });
  }, [direction, edges]);

  // ── Resizable left/right split ──
  const dragging = useRef(false);
  useEffect(() => {
    const move = (e) => {
      if (!dragging.current) return;
      setLeftWidth(Math.max(300, Math.min(window.innerWidth - 280, e.clientX)));
    };
    const up = () => {
      dragging.current = false;
      document.body.style.userSelect = '';
    };
    window.addEventListener('mousemove', move);
    window.addEventListener('mouseup', up);
    return () => {
      window.removeEventListener('mousemove', move);
      window.removeEventListener('mouseup', up);
    };
  }, []);

  // ── Graph editing ──
  const onNodesChange = useCallback((changes) => setNodes((ns) => applyNodeChanges(changes, ns)), []);
  const onEdgesChange = useCallback((changes) => setEdges((es) => applyEdgeChanges(changes, es)), []);

  const onConnect = useCallback((conn) => {
    setEdges((es) => {
      const cleaned = es.filter(
        (e) => !(e.target === conn.target && e.targetHandle === conn.targetHandle),
      );
      return addEdge({ ...conn, id: `${conn.source}->${conn.target}:${conn.targetHandle}` }, cleaned);
    });
  }, []);

  const onReconnect = useCallback((oldEdge, conn) => {
    setEdges((es) => {
      // Keep one edge per target input handle (excluding the one being moved).
      const cleaned = es.filter(
        (e) =>
          e.id === oldEdge.id ||
          !(e.target === conn.target && e.targetHandle === conn.targetHandle),
      );
      return reconnectEdge(oldEdge, conn, cleaned);
    });
  }, []);

  const changeParams = useCallback((id, params) => {
    setNodes((ns) => ns.map((n) => (n.id === id ? { ...n, data: { ...n.data, params } } : n)));
  }, []);

  const addNodeAt = useCallback((type, position) => {
    setNodes((ns) => {
      const id = uniqueId(type, ns);
      const node = {
        id,
        type: 'sop',
        position: position || { x: 60 + ns.length * 24, y: 60 + ns.length * 24 },
        selected: true,
        data: { sop: type, params: defaultParams(type) },
      };
      return ns.map((n) => ({ ...n, selected: false })).concat(node);
    });
  }, []);

  const deleteNode = useCallback((id) => {
    setNodes((ns) => ns.filter((n) => n.id !== id));
    setEdges((es) => es.filter((e) => e.source !== id && e.target !== id));
    setOutputId((cur) => (cur === id ? null : cur));
  }, []);

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

  // ── File: new / open / save / templates ──
  const loadGraph = useCallback((g) => {
    if (g.direction) {
      prevDir.current = g.direction; // skip the relayout effect; keep loaded positions
      setDirection(g.direction);
    }
    suppressCodeRegen.current = false; // regenerate code from the loaded graph
    setNodes(g.nodes);
    setEdges(g.edges);
    setOutputId(g.outputId);
    didFit.current = false;
  }, []);

  const newDoc = useCallback(() => {
    loadGraph(codeToGraph(DEFAULT_CODE, directionRef.current));
    setDocName('untitled');
    setDocPath(null);
    setShowFileMenu(false);
  }, [loadGraph]);

  const openDoc = useCallback(async () => {
    setShowFileMenu(false);
    try {
      const r = await host.openDocument();
      if (!r) return;
      loadGraph(parseDoc(r.text));
      setDocName((r.name || 'untitled').replace(/\.(procgeo|json)$/i, ''));
      setDocPath(r.path || null);
    } catch (e) {
      setError(`open: ${e.message}`);
    }
  }, [host, loadGraph]);

  const saveDoc = useCallback(
    async (asNew) => {
      setShowFileMenu(false);
      try {
        const text = serializeDoc({ nodes, edges, outputId, direction });
        const saved = await host.saveDocument(text, `${docName}.procgeo`, asNew ? null : docPath);
        if (saved) {
          setDocPath(saved);
          setDocName(String(saved).split(/[\\/]/).pop().replace(/\.(procgeo|json)$/i, ''));
        }
      } catch (e) {
        setError(`save: ${e.message}`);
      }
    },
    [host, nodes, edges, outputId, direction, docName, docPath],
  );

  const loadTemplate = useCallback(
    (t) => {
      loadGraph(codeToGraph(t.code, directionRef.current));
      setDocName(t.name.replace(/\s+/g, '-').toLowerCase());
      setDocPath(null);
      setShowTemplates(false);
      setShowFileMenu(false);
    },
    [loadGraph],
  );

  // Keyboard shortcuts: Ctrl/Cmd + N/O/S.
  useEffect(() => {
    const onKey = (e) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      const k = e.key.toLowerCase();
      if (k === 's') { e.preventDefault(); saveDoc(e.shiftKey); }
      else if (k === 'o') { e.preventDefault(); openDoc(); }
      else if (k === 'n') { e.preventDefault(); newDoc(); }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [saveDoc, openDoc, newDoc]);

  // Close the File menu on any outside click.
  useEffect(() => {
    if (!showFileMenu) return;
    const close = () => setShowFileMenu(false);
    window.addEventListener('click', close);
    return () => window.removeEventListener('click', close);
  }, [showFileMenu]);

  return (
    <div className="pg-studio">
      <div className="pg-toolbar">
        <div className="pg-filemenu">
          <button onClick={(e) => { e.stopPropagation(); setShowFileMenu((v) => !v); }}>
            File ▾
          </button>
          {showFileMenu && (
            <div className="pg-menu" onClick={(e) => e.stopPropagation()}>
              <button className="pg-menu-item" onClick={newDoc}>
                New <span className="pg-menu-kbd">Ctrl N</span>
              </button>
              <button className="pg-menu-item" onClick={openDoc}>
                Open… <span className="pg-menu-kbd">Ctrl O</span>
              </button>
              <button className="pg-menu-item" onClick={() => saveDoc(false)}>
                Save <span className="pg-menu-kbd">Ctrl S</span>
              </button>
              <button className="pg-menu-item" onClick={() => saveDoc(true)}>
                Save As… <span className="pg-menu-kbd">Ctrl ⇧ S</span>
              </button>
              <div className="pg-menu-sep" />
              <button
                className="pg-menu-item"
                onClick={() => { setShowTemplates(true); setShowFileMenu(false); }}
              >
                New from Template…
              </button>
            </div>
          )}
        </div>
        <div className="pg-tabs">
          <button className={tab === 'nodes' ? 'active' : ''} onClick={() => setTab('nodes')}>
            Nodes
          </button>
          <button className={tab === 'code' ? 'active' : ''} onClick={() => setTab('code')}>
            Code
          </button>
        </div>
        <AddNodeMenu sops={sops} onAdd={(t) => addNodeAt(t)} />
        <span className="pg-docname">{docName}</span>
        <div className="pg-spacer" />
        <button onClick={() => viewportRef.current?.fit()}>frame</button>
        <button onClick={() => setShowSettings(true)}>⚙ Settings</button>
      </div>

      <div className="pg-body">
        <div className="pg-left" style={{ width: leftWidth }}>
          <div className="pg-editor" style={{ display: tab === 'nodes' ? 'block' : 'none' }}>
            <ReactFlowProvider>
              <NodeCanvas
                nodes={displayNodes}
                edges={edges}
                direction={direction}
                sops={sops}
                onNodesChange={onNodesChange}
                onEdgesChange={onEdgesChange}
                onConnect={onConnect}
                onReconnect={onReconnect}
                onAddNode={addNodeAt}
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

        <div
          className="pg-divider"
          onMouseDown={(e) => {
            dragging.current = true;
            document.body.style.userSelect = 'none';
            e.preventDefault();
          }}
        />

        <div className="pg-right">
          <Viewport ref={viewportRef} viewMode={viewMode} />
          <div className={`pg-status ${error ? 'error' : ''}`}>{error || status}</div>
        </div>
      </div>

      {showSettings && (
        <SettingsModal
          direction={direction}
          setDirection={setDirection}
          viewMode={viewMode}
          setViewMode={setViewMode}
          onClose={() => setShowSettings(false)}
        />
      )}

      {showTemplates && (
        <TemplatesModal
          templates={TEMPLATES}
          onPick={loadTemplate}
          onClose={() => setShowTemplates(false)}
        />
      )}
    </div>
  );
}

function TemplatesModal({ templates, onPick, onClose }) {
  return (
    <div className="pg-modal-backdrop" onMouseDown={onClose}>
      <div className="pg-modal pg-modal-wide" onMouseDown={(e) => e.stopPropagation()}>
        <div className="pg-modal-head">
          <span>New from Template</span>
          <button className="pg-modal-x" onClick={onClose}>
            ✕
          </button>
        </div>
        <div className="pg-template-grid">
          {templates.map((t) => (
            <button key={t.name} className="pg-template" onClick={() => onPick(t)}>
              <span className="pg-template-name">{t.name}</span>
              <span className="pg-template-desc">{t.description}</span>
            </button>
          ))}
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

function SettingsModal({ direction, setDirection, viewMode, setViewMode, onClose }) {
  return (
    <div className="pg-modal-backdrop" onMouseDown={onClose}>
      <div className="pg-modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="pg-modal-head">
          <span>Settings</span>
          <button className="pg-modal-x" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="pg-setting">
          <span className="pg-setting-label">Node layout</span>
          <div className="pg-seg">
            <button className={direction === 'LR' ? 'active' : ''} onClick={() => setDirection('LR')}>
              ⇥ Horizontal
            </button>
            <button className={direction === 'TB' ? 'active' : ''} onClick={() => setDirection('TB')}>
              ⤓ Vertical
            </button>
          </div>
        </div>

        <div className="pg-setting">
          <span className="pg-setting-label">View mode</span>
          <div className="pg-seg">
            {['shaded', 'shaded_wire', 'wire'].map((m) => (
              <button key={m} className={viewMode === m ? 'active' : ''} onClick={() => setViewMode(m)}>
                {m === 'shaded_wire' ? 'both' : m}
              </button>
            ))}
          </div>
        </div>
      </div>
    </div>
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
