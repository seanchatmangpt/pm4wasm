import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

export default defineConfig({
  plugins: [wasm(), topLevelAwait()],
  root: "demo",
  resolve: {
    alias: {
      "@pm4py/pm4wasm": new URL("./src/index.ts", import.meta.url).pathname,
    },
  },
  server: {
    port: 5173,
    open: true,
  },
  build: {
    rollupOptions: {
      input: {
        main: './demo/index.html',
        llm: './demo/llm/index.html'
      }
    }
  }
});
