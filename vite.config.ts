/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import { configDefaults } from "vitest/config";
import react from "@vitejs/plugin-react";
export default defineConfig({
  plugins: [react()],
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/target/**"] },
  },
  // scripts/ holds plain Node checks (run with `node`), not Vitest suites.
  test: { exclude: [...configDefaults.exclude, "scripts/**"] },
  clearScreen: false,
});
