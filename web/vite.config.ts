import wasm from "vite-plugin-wasm";
import { defineConfig } from "vite";

export default defineConfig({
  build: {
    target: "esnext",
  },
  plugins: [wasm()],
});
