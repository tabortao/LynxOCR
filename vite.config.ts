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
    chunkSizeWarningLimit: 700,
    rollupOptions: {
      input: {
        main: path.resolve(__dirname, "index.html"),
        screenshot: path.resolve(__dirname, "screenshot.html"),
      },
      output: {
        manualChunks(id: string) {
          if (
            id.includes("node_modules/react") ||
            id.includes("node_modules/react-dom")
          ) {
            return "vendor-react"
          }
          if (id.includes("node_modules/@tauri-apps")) {
            return "vendor-tauri"
          }
          if (id.includes("node_modules/@dnd-kit")) {
            return "vendor-dnd"
          }
          if (
            id.includes("node_modules/@radix-ui") ||
            id.includes("node_modules/radix-ui")
          ) {
            return "vendor-radix"
          }
          if (id.includes("node_modules/lucide-react")) {
            return "vendor-icons"
          }
          if (
            id.includes("node_modules/react-markdown") ||
            id.includes("node_modules/rehype") ||
            id.includes("node_modules/remark") ||
            id.includes("node_modules/katex") ||
            id.includes("node_modules/mdast") ||
            id.includes("node_modules/micromark") ||
            id.includes("node_modules/unified") ||
            id.includes("node_modules/unist") ||
            id.includes("node_modules/hast") ||
            id.includes("node_modules/vfile") ||
            id.includes("node_modules/bail") ||
            id.includes("node_modules/trough") ||
            id.includes("node_modules/is-plain-obj") ||
            id.includes("node_modules/mdurl") ||
            id.includes("node_modules/character-entities") ||
            id.includes("node_modules/decode-named-character-reference") ||
            id.includes("node_modules/trim-lines") ||
            id.includes("node_modules/space-separated-tokens") ||
            id.includes("node_modules/comma-separated-tokens") ||
            id.includes("node_modules/property-information") ||
            id.includes("node_modules/ccount") ||
            id.includes("node_modules/markdown-table") ||
            id.includes("node_modules/zwitch") ||
            id.includes("node_modules/longest-streak") ||
            id.includes("node_modules/html-void-elements") ||
            id.includes("node_modules/web-namespaces") ||
            id.includes("node_modules/stringify-entities") ||
            id.includes("node_modules/character-reference-invalid")
          ) {
            return "vendor-markdown"
          }
          if (id.includes("node_modules/@tanstack")) {
            return "vendor-table"
          }
          if (id.includes("node_modules/zod")) {
            return "vendor-zod"
          }
        },
      },
    },
  },
})
