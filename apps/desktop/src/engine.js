import { invoke } from '@tauri-apps/api/core';

// Native engine: cooks SOP DAGs in Rust via Tauri commands. The geometry never
// touches WASM — only render buffers cross the IPC boundary.
export const nativeEngine = {
  cookGraph(dag) {
    return invoke('cook_graph', { graph: dag });
  },
  listSops() {
    return invoke('list_sops');
  },
};
