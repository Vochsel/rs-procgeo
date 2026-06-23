import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';

const FILTERS = [{ name: 'ProcGeo Graph', extensions: ['procgeo', 'json'] }];

// Native file host: real OS open/save dialogs (dialog plugin) + std::fs read/write
// (custom Rust commands). Studio calls these for File ▸ Open / Save.
export const nativeHost = {
  async saveDocument(text, suggestedName, existingPath) {
    const path = existingPath || (await save({ defaultPath: suggestedName, filters: FILTERS }));
    if (!path) return null;
    await invoke('write_text', { path, contents: text });
    return path;
  },

  async openDocument() {
    const path = await open({ multiple: false, filters: FILTERS });
    if (!path) return null;
    const text = await invoke('read_text', { path });
    return { name: baseName(path), path, text };
  },
};

function baseName(p) {
  const parts = String(p).split(/[\\/]/);
  return parts[parts.length - 1] || 'untitled';
}
