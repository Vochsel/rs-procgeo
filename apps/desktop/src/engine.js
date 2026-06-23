import { invoke } from '@tauri-apps/api/core';

// Native engine: cooks SOP DAGs in Rust via Tauri commands. The geometry never
// touches WASM — only packed binary render buffers cross the IPC boundary
// (decoded into typed-array views here, zero per-element JSON cost).
export const nativeEngine = {
  async cookGraph(dag) {
    const res = await invoke('cook_graph', { graph: dag });
    return decode(res);
  },
  listSops() {
    return invoke('list_sops');
  },
};

// Mirrors the Rust `pack()` layout in src-tauri/src/lib.rs.
function decode(res) {
  const u8 = res instanceof ArrayBuffer ? new Uint8Array(res) : new Uint8Array(res.buffer || res);
  const ab = u8.buffer;
  const dv = new DataView(ab, u8.byteOffset, u8.byteLength);

  const numPoints = dv.getUint32(0, true);
  const numPrims = dv.getUint32(4, true);
  const posLen = dv.getUint32(8, true);
  const idxLen = dv.getUint32(12, true);
  const nrmLen = dv.getUint32(16, true);
  const colLen = dv.getUint32(20, true);

  let off = u8.byteOffset + 24;
  const positions = new Float32Array(ab, off, posLen);
  off += posLen * 4;
  const indices = new Uint32Array(ab, off, idxLen);
  off += idxLen * 4;
  const normals = nrmLen ? new Float32Array(ab, off, nrmLen) : null;
  off += nrmLen * 4;
  const colors = colLen ? new Float32Array(ab, off, colLen) : null;

  return { positions, indices, normals, colors, numPoints, numPrims };
}
