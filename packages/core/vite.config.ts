import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import dts from 'vite-plugin-dts';

export default defineConfig({
    plugins: [
        react(),
        dts({
            insertTypesEntry: true,
        }),
    ],
    build: {
        lib: {
            entry: 'src/index.ts',
            formats: ['es', 'cjs'],
            fileName: (format, entryName) => {
                const ext = format === 'es' ? 'mjs' : 'cjs';
                return `${entryName}.${ext}`;
            },
        },
        rollupOptions: {
            external: [
                /^react(\/.*)?$/,
                /^react-dom(\/.*)?$/,
                'use-sync-external-store',
                'nanoid',
            ],
        },
    },
});
