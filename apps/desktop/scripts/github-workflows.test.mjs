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

    expect(releaseSource).toContain("publish_release:");
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
    expect(releaseSource).toContain(
      "WHATSVAULT_RELEASE_DRAFT: ${{ inputs.publish_release == true && 'false' || 'true' }}",
    );
    expect(releaseSource).toContain(
      "releaseDraft: ${{ env.WHATSVAULT_RELEASE_DRAFT == 'true' }}",
    );
    expect(releaseSource).toContain("prerelease: ${{ env.WHATSVAULT_STABLE_RELEASE != 'true' }}");
    expect(releaseSource).not.toContain("Developer ID Application:");
    expect(releaseSource).not.toContain("secret-value");
  });

  it("keeps release publishing explicit while stable releases remain signing-gated", async () => {
    const releaseSource = await readFile(workflowPaths[1], "utf8");
    const publishInputIndex = releaseSource.indexOf("publish_release:");
    const publishDefaultIndex = releaseSource.indexOf("default: false", publishInputIndex);
    const draftEnvIndex = releaseSource.indexOf("WHATSVAULT_RELEASE_DRAFT");
    const draftActionIndex = releaseSource.indexOf("releaseDraft:");
    const prereleaseActionIndex = releaseSource.indexOf("prerelease:");
    const stablePreflightIndex = releaseSource.indexOf("stable-signing-preflight:");
    const matrixGateIndex = releaseSource.indexOf("needs.stable-signing-preflight.result == 'success'");

    expect(publishInputIndex).toBeGreaterThan(-1);
    expect(publishDefaultIndex).toBeGreaterThan(publishInputIndex);
    expect(draftEnvIndex).toBeGreaterThan(publishInputIndex);
    expect(draftActionIndex).toBeGreaterThan(draftEnvIndex);
    expect(prereleaseActionIndex).toBeGreaterThan(draftActionIndex);
    expect(stablePreflightIndex).toBeGreaterThan(-1);
    expect(matrixGateIndex).toBeGreaterThan(stablePreflightIndex);
    expect(releaseSource).toContain(
      "if: always() && (inputs.stable_release != true || needs.stable-signing-preflight.result == 'success')",
    );
  });

  it("does not pass empty macOS signing secrets directly to the Tauri release action", async () => {
    const releaseSource = await readFile(workflowPaths[1], "utf8");
    const actionStepIndex = releaseSource.indexOf("Build and upload Tauri artifacts");
    const actionWithIndex = releaseSource.indexOf("with:", actionStepIndex);
    const actionEnvBlock = releaseSource.slice(actionStepIndex, actionWithIndex);

    expect(actionStepIndex).toBeGreaterThan(-1);
    expect(actionWithIndex).toBeGreaterThan(actionStepIndex);
    expect(actionEnvBlock).not.toContain(
      "APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}",
    );
    expect(actionEnvBlock).not.toContain("APPLE_ID: ${{ secrets.APPLE_ID }}");
    expect(releaseSource).toContain("Export macOS release signing environment");
    expect(releaseSource).toContain("APPLE_SIGNING_IDENTITY=-");
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

  it("attests release artifacts from the checksum manifest before uploading checksums", async () => {
    const releaseSource = await readFile(workflowPaths[1], "utf8");
    const permissionsIndex = releaseSource.indexOf("permissions:");
    const generateChecksumsIndex = releaseSource.indexOf("Generate release checksums");
    const attestIndex = releaseSource.indexOf("Attest release artifacts");
    const uploadChecksumsIndex = releaseSource.indexOf("Upload release checksums");

    expect(permissionsIndex).toBeGreaterThan(-1);
    expect(releaseSource.slice(permissionsIndex, generateChecksumsIndex)).toContain("id-token: write");
    expect(releaseSource.slice(permissionsIndex, generateChecksumsIndex)).toContain("attestations: write");
    expect(attestIndex).toBeGreaterThan(generateChecksumsIndex);
    expect(uploadChecksumsIndex).toBeGreaterThan(attestIndex);
    expect(releaseSource).toContain("uses: actions/attest@v4");
    expect(releaseSource).toContain(
      "subject-checksums: target/release/release-metadata/${{ matrix.checksum_manifest }}",
    );
    expect(releaseSource).not.toContain("subject-path: target/release/bundle");
  });
});
