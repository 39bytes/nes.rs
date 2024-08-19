import wasm from "vite-plugin-wasm";
import { defineConfig } from "vite";

export default defineConfig({
  base: "/nes.rs/",
  build: {
    target: "esnext",
  },
  plugins: [wasm()],
});
