import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
export default defineConfig({
    plugins: [react()],
    optimizeDeps: {
        include: ["@stellar/stellar-sdk"],
    },
    build: {
        commonjsOptions: {
            transformMixedEsModules: true,
        },
    },
    define: {
        global: "globalThis",
    },
    server: {
        port: 5173,
    },
    test: {
        environment: "jsdom",
        globals: true,
        setupFiles: [],
    },
});
