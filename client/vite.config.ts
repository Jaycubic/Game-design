import { defineConfig } from "vitest/config";
import { fileURLToPath } from "node:url";

// Dev: `npm run dev` serves the client on :5173 and proxies the API and
// WebSocket to the Rust server (GAME_SERVER, default http://localhost:8070).
const server = process.env.GAME_SERVER ?? "http://localhost:8070";
const root = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig({
  root,
  publicDir: "public",
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
    chunkSizeWarningLimit: 8000,
    rollupOptions: {
      output: {
        manualChunks: (id) => (id.includes("@babylonjs") ? "babylon" : undefined),
      },
    },
  },
  server: {
    port: 5173,
    host: true,
    proxy: {
      "/api": { target: server, changeOrigin: true, secure: false },
      "/health": { target: server, changeOrigin: true, secure: false },
      "/ready": { target: server, changeOrigin: true, secure: false },
      "/ws": { target: server.replace(/^http/, "ws"), ws: true, changeOrigin: true, secure: false },
    },
  },
  test: {
    root,
    include: ["tests/**/*.test.ts"],
    environment: "node",
  },
});
