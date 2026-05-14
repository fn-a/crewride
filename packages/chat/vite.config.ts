import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import unpluginIcons from 'unplugin-icons/vite';
import { ExternalPackageIconLoader } from 'unplugin-icons/loaders';
import path from 'node:path';

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
    resolve: {
        alias: {
            '@': path.resolve(import.meta.dirname, './src'),
        },
    },
    server: {
        port: 5173,
        proxy: {
            '/v1': {
                target: 'http://127.0.0.1:8899',
                changeOrigin: true,
            },
        },
    },
});
