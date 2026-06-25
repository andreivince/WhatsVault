import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "../../..");
const workflowPaths = [
  join(repoRoot, ".github", "workflows", "ci.yml"),
  join(repoRoot, ".github", "workflows", "release.yml"),
];
const TAURI_ACTION_VERSION = "tauri-apps/tauri-action@v0.6.2";

describe("GitHub workflow configuration", () => {
  it("pins tauri-action to a published release tag instead of an unresolved major alias", async () => {
    const workflowSources = await Promise.all(
      workflowPaths.map((path) => readFile(path, "utf8")),
    );
    const combined = workflowSources.join("\n");
    const actionReferences = combined.match(/tauri-apps\/tauri-action@[^\s]+/g) ?? [];

    expect(actionReferences).not.toHaveLength(0);
    expect(actionReferences).toEqual(actionReferences.map(() => TAURI_ACTION_VERSION));
    expect(combined).not.toContain("tauri-apps/tauri-action@v1");
    expect(combined).not.toContain("tauri-apps/tauri-action@latest");
  });
});
