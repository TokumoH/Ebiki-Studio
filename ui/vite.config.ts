import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// `--mode mock` força o transporte simulado (UI no navegador sem o núcleo Rust).
export default defineConfig(({ mode }) => ({
  plugins: [react()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  define: { __FORCE_MOCK__: JSON.stringify(mode === "mock") },
  build: { target: "es2022", sourcemap: false, chunkSizeWarningLimit: 4000 },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
}));
