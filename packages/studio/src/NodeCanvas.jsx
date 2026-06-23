import { useEffect, useMemo, useState } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  useUpdateNodeInternals,
  useReactFlow,
} from '@xyflow/react';
import { SopNode } from './SopNode.jsx';
import { DirectionContext } from './layoutContext.js';

const CATEGORIES = ['create', 'filter', 'combine'];

/**
 * The xyflow node canvas. Controlled by the parent (nodes/edges live in Studio).
 * Adds a right-click "add node" menu and draggable edge reconnection.
 */
export function NodeCanvas({
  nodes,
  edges,
  direction = 'LR',
  sops = [],
  onNodesChange,
  onEdgesChange,
  onConnect,
  onReconnect,
  onAddNode,
}) {
  const nodeTypes = useMemo(() => ({ sop: SopNode }), []);

  // Handles move (left/right ↔ top/bottom) when the direction flips, so React
  // Flow must re-measure them or edges stay anchored to the old positions.
  const updateNodeInternals = useUpdateNodeInternals();
  useEffect(() => {
    nodes.forEach((n) => updateNodeInternals(n.id));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [direction]);

  // ── Right-click context menu (add node at cursor) ──
  const { screenToFlowPosition } = useReactFlow();
  const [menu, setMenu] = useState(null);
  const closeMenu = () => setMenu(null);
  const openMenu = (e) => {
    e.preventDefault();
    setMenu({ x: e.clientX, y: e.clientY, flow: screenToFlowPosition({ x: e.clientX, y: e.clientY }) });
  };
  useEffect(() => {
    if (!menu) return;
    const close = () => closeMenu();
    window.addEventListener('click', close);
    window.addEventListener('contextmenu', close);
    return () => {
      window.removeEventListener('click', close);
      window.removeEventListener('contextmenu', close);
    };
  }, [menu]);

  return (
    <DirectionContext.Provider value={direction}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onReconnect={onReconnect}
        onPaneContextMenu={openMenu}
        onPaneClick={closeMenu}
        onMoveStart={closeMenu}
        fitView
        proOptions={{ hideAttribution: true }}
        defaultEdgeOptions={{ animated: false }}
      >
        <Background color="#2a2a3a" gap={18} />
        <Controls showInteractive={false} />
      </ReactFlow>

      {menu && (
        <div
          className="pg-ctxmenu"
          style={{ left: menu.x, top: menu.y }}
          onClick={(e) => e.stopPropagation()}
          onContextMenu={(e) => e.preventDefault()}
        >
          {CATEGORIES.map((cat) => (
            <div key={cat} className="pg-ctx-group">
              <div className="pg-ctx-cat">{cat}</div>
              {sops
                .filter((s) => s.category === cat)
                .map((s) => (
                  <button
                    key={s.type}
                    className="pg-ctx-item"
                    onClick={() => {
                      onAddNode(s.type, menu.flow);
                      closeMenu();
                    }}
                  >
                    {s.label}
                  </button>
                ))}
            </div>
          ))}
        </div>
      )}
    </DirectionContext.Provider>
  );
}
