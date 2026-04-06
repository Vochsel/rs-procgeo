import { defineConfig } from 'vite';
import path from 'path';

export default defineConfig({
    root: '.',
    publicDir: 'public',
    server: {
        port: 5173,
        open: true,
    },
    resolve: {
        alias: {
            'three/addons/': path.resolve(__dirname, 'node_modules/three/examples/jsm/'),
            'procgeo-wasm': path.resolve(__dirname, 'wasm/procgeo_wasm.js'),
        },
    },
    optimizeDeps: {
        include: ['three', 'monaco-editor'],
        exclude: ['procgeo-wasm'],
    },
    assetsInclude: ['**/*.wasm'],
});
