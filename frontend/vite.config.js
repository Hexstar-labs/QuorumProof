import { defineConfig } from 'vite';

export default defineConfig({
  // Allow stellar-sdk browser build
  optimizeDeps: {
    include: ['@stellar/stellar-sdk'],
  },
  build: {
    commonjsOptions: {
      transformMixedEsModules: true,
    },
  },
  define: {
    global: 'globalThis',
  },
  server: {
    port: 5173,
  },
  test: {
    environment: 'jsdom',
    globals: true,
  },
});
