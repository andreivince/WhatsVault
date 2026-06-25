import react from "@vitejs/plugin-react";
import { configDefaults, defineConfig } from "vitest/config";

const playwrightTestGlobs = ["tests/readme-demo/**", "tests/visual/**"];

export default defineConfig({
  plugins: [react()],
  base: "./",
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  test: {
    environment: "node",
    exclude: [...configDefaults.exclude, ...playwrightTestGlobs],
  },
});
