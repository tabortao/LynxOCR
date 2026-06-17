import path from "path"
import tailwindcss from "@tailwindcss/vite"
import react from "@vitejs/plugin-react"
import { defineConfig } from "vite"

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
     target: "esnext",
    rollupOptions: {
      input: {
        main: path.resolve(__dirname, "index.html"),
        screenshot: path.resolve(__dirname, "screenshot.html"),
      },
      output: {
        manualChunks(id: string) {
          if (id.includes("node_modules/react") || id.includes("node_modules/react-dom")) {
            return "vendor-react"
          }
          if (id.includes("node_modules/@tauri-apps")) {
            return "vendor-tauri"
          }
          if (id.includes("node_modules/@dnd-kit")) {
            return "vendor-dnd"
          }
          if (id.includes("node_modules/@radix-ui") || id.includes("node_modules/radix-ui")) {
            return "vendor-radix"
          }
          if (id.includes("node_modules/lucide-react")) {
            return "vendor-icons"
          }
        },
      },
    },
  },
})
