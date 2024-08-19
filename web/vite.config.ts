import wasm from "vite-plugin-wasm";
import { defineConfig } from "vite";
import { viteStaticCopy } from "vite-plugin-static-copy";

export default defineConfig({
  base: "/nes.rs/",
  build: {
    target: "esnext",
  },
  plugins: [
    wasm(),
    viteStaticCopy({
      targets: [
        {
          src: "coi-serviceworker.min.js",
          dest: "./",
        },
      ],
    }),
    {
      name: "build-html",
      apply: "build",
      transformIndexHtml: (html) => {
        return {
          html,
          tags: [
            {
              tag: "script",
              attrs: {
                src: "/coi-serviceworker.min.js",
              },
              injectTo: "head",
            },
          ],
        };
      },
    },
  ],
});
