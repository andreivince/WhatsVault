import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { loadConfigFromFile } from "vite";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appDir = dirname(scriptDir);
const configPath = join(appDir, "vite.config.ts");

describe("Vite production configuration", () => {
  it("uses a relative asset base so packaged Tauri windows can load built assets", async () => {
    const loaded = await loadConfigFromFile(
      { command: "build", mode: "production" },
      configPath,
      appDir,
    );

    expect(loaded?.config.base).toBe("./");
  });
});
