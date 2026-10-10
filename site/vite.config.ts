import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import markdown from "./plugins/markdown.ts";

export default defineConfig({
  plugins: [markdown(fileURLToPath(new URL("./docs", import.meta.url))), react()],
  // active-react is linked from the repository root (file:..): keep a single React copy.
  resolve: { dedupe: ["react", "react-dom"] },
  ssr: { noExternal: ["active-react"] },
  build: {
    target: "es2022",
    rollupOptions: {
      // The server bundle imports every docs page eagerly; the client keeps one chunk per page.
      onLog: (level, log, handler) => log.code !== "INEFFECTIVE_DYNAMIC_IMPORT" && handler(level, log),
    },
  },
});
