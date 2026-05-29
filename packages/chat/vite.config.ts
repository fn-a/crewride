import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import unpluginIcons from 'unplugin-icons/vite';
import { ExternalPackageIconLoader } from 'unplugin-icons/loaders';
import path from 'node:path';

const BASE_DIR = path.resolve(import.meta.dirname, '../');
const ROOT_DIR = path.resolve(BASE_DIR, '../');

export default defineConfig({
    plugins: [
        react(),
        tailwindcss(),
        unpluginIcons({
            compiler: 'raw',
            customCollections: {
                ...ExternalPackageIconLoader('@llmicons-json/lobe'),
            },
        }),
    ],
    build: {
        outDir: path.resolve(ROOT_DIR, 'dist'),
        emptyOutDir: true,
    },
    resolve: {
        alias: {
            '@': path.resolve(BASE_DIR, 'chat/src'),
            '@crewride/core': path.resolve(BASE_DIR, 'core/src'),
        },
    },
    server: {
        port: 5173,
        proxy: {
            '/v1': {
                target: 'http://127.0.0.1:8899',
                changeOrigin: true,
            },
            '/api': {
                target: 'http://127.0.0.1:8899',
                changeOrigin: true,
            },
        },
    },
});
