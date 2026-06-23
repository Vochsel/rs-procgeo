import { useEffect, useMemo } from 'react';
import { ReactFlow, Background, Controls, useUpdateNodeInternals } from '@xyflow/react';
import { SopNode } from './SopNode.jsx';
import { DirectionContext } from './layoutContext.js';

/**
 * The xyflow node canvas. Controlled by the parent (nodes/edges live in Studio).
 * `direction` drives handle orientation via DirectionContext.
 */
export function NodeCanvas({
  nodes,
  edges,
  direction = 'LR',
  onNodesChange,
  onEdgesChange,
  onConnect,
}) {
  const nodeTypes = useMemo(() => ({ sop: SopNode }), []);

  // Handles move (left/right ↔ top/bottom) when the direction flips, so React
  // Flow must re-measure them or edges stay anchored to the old positions.
  const updateNodeInternals = useUpdateNodeInternals();
  useEffect(() => {
    nodes.forEach((n) => updateNodeInternals(n.id));
    // Only re-measure on a direction change, not on every node edit.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [direction]);

  return (
    <DirectionContext.Provider value={direction}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        fitView
        proOptions={{ hideAttribution: true }}
        defaultEdgeOptions={{ animated: false }}
      >
        <Background color="#2a2a3a" gap={18} />
        <Controls showInteractive={false} />
      </ReactFlow>
    </DirectionContext.Provider>
  );
}
