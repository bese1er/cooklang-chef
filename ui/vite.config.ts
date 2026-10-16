import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";

export default defineConfig({
  plugins: [wasm()],

  optimizeDeps: {
    exclude: ["@cooklang/cooklang"],
  },

  build: {
    target: "esnext",
    outDir: "assets/js",
    emptyOutDir: false,

    rolldownOptions: {
      input: {
        editor: "src/editor.ts",
        // Ajoute ici tes autres points d'entrée :
        // search: "src/search.ts",
        // settings: "src/settings.ts",
      },

      output: {
        entryFileNames: "[name].js",
        chunkFileNames: "chunks/[name]-[hash].js",
        assetFileNames: "assets/[name]-[hash][extname]",
      },
    },
  },
});