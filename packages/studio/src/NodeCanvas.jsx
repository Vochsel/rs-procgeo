import { useMemo } from 'react';
import { ReactFlow, Background, Controls } from '@xyflow/react';
import { SopNode } from './SopNode.jsx';

/**
 * The xyflow node canvas. Controlled by the parent (nodes/edges live in Studio).
 */
export function NodeCanvas({
  nodes,
  edges,
  onNodesChange,
  onEdgesChange,
  onConnect,
  onSelectionChange,
}) {
  const nodeTypes = useMemo(() => ({ sop: SopNode }), []);

  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      nodeTypes={nodeTypes}
      onNodesChange={onNodesChange}
      onEdgesChange={onEdgesChange}
      onConnect={onConnect}
      onSelectionChange={onSelectionChange}
      fitView
      proOptions={{ hideAttribution: true }}
      defaultEdgeOptions={{ animated: false }}
    >
      <Background color="#2a2a3a" gap={18} />
      <Controls showInteractive={false} />
    </ReactFlow>
  );
}
