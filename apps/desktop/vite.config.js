import { defineConfig } from 'vite';

// Vite config for the Tauri webview frontend.
// Tauri controls the dev server lifecycle, so don't auto-open a browser.
export default defineConfig({
  clearScreen: false,
  server: {
    port: 5174,
    strictPort: true,
  },
  build: {
    outDir: 'dist',
    target: 'esnext',
    emptyOutDir: true,
  },
});
