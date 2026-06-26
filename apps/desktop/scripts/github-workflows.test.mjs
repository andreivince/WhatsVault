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

  it("runs public hygiene and release readiness guards before building the frontend", async () => {
    const ciSource = await readFile(workflowPaths[0], "utf8");
    const hygieneIndex = ciSource.indexOf("npm run hygiene:public");
    const readinessIndex = ciSource.indexOf("npm run release:readiness");
    const buildIndex = ciSource.indexOf("npm run build");

    expect(hygieneIndex).toBeGreaterThan(-1);
    expect(readinessIndex).toBeGreaterThan(-1);
    expect(buildIndex).toBeGreaterThan(-1);
    expect(hygieneIndex).toBeLessThan(buildIndex);
    expect(readinessIndex).toBeLessThan(buildIndex);
  });

  it("gates desktop visual checks in CI after installing Playwright Chromium", async () => {
    const ciSource = await readFile(workflowPaths[0], "utf8");
    const buildIndex = ciSource.indexOf("npm run build");
    const playwrightInstallIndex = ciSource.indexOf("npx playwright install --with-deps chromium");
    const visualCheckIndex = ciSource.indexOf("npm run visual:check");
    const auditIndex = ciSource.indexOf("npm audit --audit-level=low");

    expect(buildIndex).toBeGreaterThan(-1);
    expect(playwrightInstallIndex).toBeGreaterThan(buildIndex);
    expect(visualCheckIndex).toBeGreaterThan(playwrightInstallIndex);
    expect(auditIndex).toBeGreaterThan(visualCheckIndex);
  });

  it("keeps release notes aligned with current proof gates", async () => {
    const releaseSource = await readFile(workflowPaths[1], "utf8");

    expect(releaseSource).toContain(
      "iPhone backup chat rendering, bounded media preview, and bounded HTML export have local proof.",
    );
    expect(releaseSource).not.toMatch(/proof-gated|media preview and HTML export are still/i);
  });

  it("wires release signing preflight without committing signing values", async () => {
    const releaseSource = await readFile(workflowPaths[1], "utf8");

    expect(releaseSource).toContain("stable_release:");
    expect(releaseSource).toContain("stable-signing-preflight:");
    expect(releaseSource).toContain("needs: stable-signing-preflight");
    expect(releaseSource).toContain("needs.stable-signing-preflight.result == 'success'");
    expect(releaseSource).toContain("npm run release:prepare-signing-config");
    expect(releaseSource).toContain("npm run release:signing");
    expect(releaseSource).toContain("npm run release:preflight");
    expect(releaseSource).toContain("shell: bash");
    expect(releaseSource).toContain("APPLE_API_KEY_PRIVATE_KEY: ${{ secrets.APPLE_API_KEY_PRIVATE_KEY }}");
    expect(releaseSource).toContain("WINDOWS_CERTIFICATE_THUMBPRINT: ${{ secrets.WINDOWS_CERTIFICATE_THUMBPRINT }}");
    expect(releaseSource).toContain("prerelease: ${{ env.WHATSVAULT_STABLE_RELEASE != 'true' }}");
    expect(releaseSource).not.toContain("Developer ID Application:");
    expect(releaseSource).not.toContain("secret-value");
  });

  it("runs strict signing preflight once before release matrix builds", async () => {
    const releaseSource = await readFile(workflowPaths[1], "utf8");
    const preflightJobIndex = releaseSource.indexOf("stable-signing-preflight:");
    const matrixJobIndex = releaseSource.indexOf("tauri-release:");
    const strictPreflightIndex = releaseSource.indexOf("npm run release:preflight");
    const buildActionIndex = releaseSource.indexOf("Build and upload Tauri artifacts");
    const matrixReadinessIndex = releaseSource.lastIndexOf("Check signing readiness");

    expect(preflightJobIndex).toBeGreaterThan(-1);
    expect(matrixJobIndex).toBeGreaterThan(preflightJobIndex);
    expect(strictPreflightIndex).toBeGreaterThan(preflightJobIndex);
    expect(strictPreflightIndex).toBeLessThan(matrixJobIndex);
    expect(buildActionIndex).toBeGreaterThan(matrixJobIndex);
    expect(matrixReadinessIndex).toBeGreaterThan(matrixJobIndex);
    expect(releaseSource.slice(matrixReadinessIndex, buildActionIndex)).not.toContain(
      "npm run release:preflight",
    );
  });
});
