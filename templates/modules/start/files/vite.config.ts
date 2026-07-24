import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// https://tanstack.com/start/latest/docs/framework/react/build-from-scratch
export default defineConfig({
  server: {
    port: 3000,
  },
  plugins: [
    tanstackStart(),
    // React's Vite plugin must come after Start's Vite plugin.
    react(),
  ],
  // Vite resolves tsconfig `paths` aliases natively, replacing the
  // vite-tsconfig-paths plugin.
  resolve: {
    tsconfigPaths: true,
  },
});
