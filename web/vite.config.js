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
        },
    },
    optimizeDeps: {
        include: ['three', 'monaco-editor'],
    },
});
