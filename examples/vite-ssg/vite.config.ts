import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  // Only needed because this example links active-react from the repository (file:../..).
  resolve: { dedupe: ["react", "react-dom"] },
  ssr: { noExternal: ["active-react"] },
});
