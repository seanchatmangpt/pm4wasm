import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

export default defineConfig({
  plugins: [wasm(), topLevelAwait()],
  build: {
    lib: {
      entry: "src/index.ts",
      name: "PowlWasm",
      formats: ["es"],
      fileName: () => `index.js`,
    },
    rollupOptions: {
      // wasm module is inlined by vite-plugin-wasm — nothing external
    },
  },
});
