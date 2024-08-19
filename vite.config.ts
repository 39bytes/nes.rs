import wasm from "vite-plugin-wasm";
import { defineConfig } from "vite";

export default defineConfig({
  root: "./web",
  build: {
    target: "esnext",
  },
  plugins: [wasm()],
});
