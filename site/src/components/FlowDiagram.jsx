import { Background, Controls, ReactFlow } from "@xyflow/react";

export function FlowDiagram({ nodes = [], edges = [], height = 360 }) {
  return (
    <div
      className="my-4 overflow-hidden rounded-xl border border-[var(--sl-color-gray-5)] bg-[var(--sl-color-gray-6)]"
      style={{ height }}
    >
      <ReactFlow
        nodes={nodes}
        edges={edges}
        fitView
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable={false}
      >
        <Background />
        <Controls showInteractive={false} />
      </ReactFlow>
    </div>
  );
}
