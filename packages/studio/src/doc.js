// Document (de)serialization + a browser file host fallback. The desktop app
// passes a native host (OS dialogs); web/Node fall back to download / file input.

const APP = 'procgeo-studio';
const VERSION = 1;

export function serializeDoc({ nodes, edges, outputId, direction }) {
  const cleanNodes = nodes.map((n) => ({
    id: n.id,
    type: 'sop',
    position: { x: Math.round(n.position.x), y: Math.round(n.position.y) },
    data: { sop: n.data.sop, params: n.data.params, ...(n.data.bypass ? { bypass: true } : {}) },
  }));
  const cleanEdges = edges.map((e) => ({
    id: e.id,
    source: e.source,
    target: e.target,
    targetHandle: e.targetHandle,
  }));
  return JSON.stringify(
    { app: APP, version: VERSION, direction, outputId, nodes: cleanNodes, edges: cleanEdges },
    null,
    2,
  );
}

export function parseDoc(text) {
  const doc = JSON.parse(text);
  if (doc.app !== APP) throw new Error('not a ProcGeo document');
  const nodes = (doc.nodes || []).map((n) => ({
    id: n.id,
    type: 'sop',
    position: n.position || { x: 0, y: 0 },
    data: { sop: n.data.sop, params: n.data.params || {}, ...(n.data.bypass ? { bypass: true } : {}) },
  }));
  const edges = (doc.edges || []).map((e) => ({
    id: e.id,
    source: e.source,
    target: e.target,
    targetHandle: e.targetHandle,
  }));
  return { nodes, edges, outputId: doc.outputId || null, direction: doc.direction || 'LR' };
}

// Browser host: download to save, <input type=file> to open.
export const browserHost = {
  async saveDocument(text, suggestedName) {
    const blob = new Blob([text], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = suggestedName || 'untitled.procgeo';
    a.click();
    URL.revokeObjectURL(url);
    return suggestedName || null;
  },

  openDocument() {
    return new Promise((resolve) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = '.procgeo,.json,application/json';
      input.onchange = () => {
        const file = input.files?.[0];
        if (!file) return resolve(null);
        const reader = new FileReader();
        reader.onload = () => resolve({ name: file.name, text: String(reader.result) });
        reader.readAsText(file);
      };
      input.click();
    });
  },
};
