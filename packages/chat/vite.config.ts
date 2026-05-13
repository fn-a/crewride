import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import Icons from 'unplugin-icons/vite';
import path from 'node:path';

export default defineConfig({
    plugins: [
        react(),
        tailwindcss(),
        Icons({
            compiler: 'raw',
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
