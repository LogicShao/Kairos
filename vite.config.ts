import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"
import tailwindcss from "@tailwindcss/vite"
import path from "path"

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  clearScreen: false,
  server: {
    host: process.env.TAURI_DEV_HOST ?? process.env.VITE_DEV_HOST ?? "127.0.0.1",
    port: 5173,
    strictPort: true,
    allowedHosts: [
      "localhost",
      "127.0.0.1",
      ...(process.env.TAURI_DEV_HOST ? [process.env.TAURI_DEV_HOST] : []),
      ...(process.env.VITE_DEV_HOST ? [process.env.VITE_DEV_HOST] : []),
    ],
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "esnext",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
})
