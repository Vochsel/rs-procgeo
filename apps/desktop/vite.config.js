import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

const repoRoot = path.resolve(__dirname, '../..');

// Vite config for the Tauri webview frontend.
// @procgeo/studio is consumed as source (JSX) so it must be aliased to its src
// and excluded from dep pre-bundling, letting the React plugin transform it.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5174,
    strictPort: true,
    fs: { allow: [repoRoot] },
  },
  resolve: {
    alias: {
      '@procgeo/studio/styles.css': path.resolve(repoRoot, 'packages/studio/src/styles.css'),
      '@procgeo/studio': path.resolve(repoRoot, 'packages/studio/src/index.js'),
    },
  },
  optimizeDeps: { exclude: ['@procgeo/studio'] },
  build: {
    outDir: 'dist',
    target: 'esnext',
    emptyOutDir: true,
  },
});
