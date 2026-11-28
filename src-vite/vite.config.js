import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath, URL } from "node:url";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  clearScreen: false,
  server: {
    port: 3580,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 3581 } : undefined,
    watch: {
      ignored: ["**/src-tauri/**", "**/crates/**", "**/../src/**", "**/../dist/**"],
    },
  },
  build: {
    target: "es2021",
    minify: "esbuild",
    sourcemap: false,
  },
});
