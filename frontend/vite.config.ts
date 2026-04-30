import { defineConfig } from 'vite';

export default defineConfig({
  server: {
    proxy: {
      '/ws': {
        target: 'ws://localhost:3000',
        ws: true,
        rewriteWsOrigin: true,
      },
    },
  },
  build: {
    outDir: '../server/static',
    emptyOutDir: true,
  },
});
