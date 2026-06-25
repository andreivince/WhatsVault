import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import {
  evaluateReleaseReadiness,
  exitCodeForReleaseReadiness,
  factsFromEnv,
  formatReleaseReadiness,
  loadReleaseReadinessContext,
} from "./release-readiness.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "../../..");

describe("release readiness guard", () => {
  it("keeps missing backup proof blockers centralized", () => {
    const result = evaluateReleaseReadiness({
      facts: {
        realIphoneBackupProof: true,
        realBackupChatRenderProof: true,
        realBackupMediaPreviewProof: false,
        realBackupHtmlExportProof: false,
        packagedRenderSmoke: "passed",
        macosSigning: false,
        windowsSigning: false,
      },
      docs: {
        readme: "Real iPhone-backup Manifest proof passed. Real backup media UI smoke is still pending. Real-backup export smoke is still pending. release artifacts are unsigned.",
        supportedSources: "real backup ChatStorage proof passed. real backup media UI proof pending. real backup export proof pending.",
        ciRelease: "local package smoke passed. Current release artifacts are unsigned. real backup media preview proof pending. real backup HTML export proof pending.",
        architecture: "real backup chat rendering is verified locally. Real backup media UI proof pending. Real backup export proof pending.",
      },
    });

    expect(result.stableReleaseReady).toBe(false);
    expect(result.blockers.map((blocker) => blocker.id)).toEqual([
      "real-backup-media-preview-proof",
      "real-backup-html-export-proof",
      "code-signing",
    ]);
    expect(result.documentationIssues).toEqual([]);
  });

  it("fails documentation honesty when a blocker is not mentioned", () => {
    const result = evaluateReleaseReadiness({
      facts: {
        realIphoneBackupProof: true,
        realBackupChatRenderProof: true,
        realBackupMediaPreviewProof: false,
        realBackupHtmlExportProof: false,
        packagedRenderSmoke: "blocked",
        macosSigning: false,
        windowsSigning: false,
      },
      docs: {
        readme: "Pre-alpha status only.",
        supportedSources: "",
        ciRelease: "",
        architecture: "",
      },
    });

    expect(result.documentationIssues.map((issue) => issue.blockerId)).toEqual([
      "real-backup-media-preview-proof",
      "real-backup-html-export-proof",
      "packaged-render-smoke",
      "code-signing",
    ]);
  });

  it("requires a public-safe proof evidence page when real-backup proof is marked passed", () => {
    const result = evaluateReleaseReadiness({
      facts: {
        realIphoneBackupProof: true,
        realBackupChatRenderProof: true,
        realBackupMediaPreviewProof: true,
        realBackupHtmlExportProof: true,
        packagedRenderSmoke: "passed",
        macosSigning: false,
        windowsSigning: false,
      },
      docs: {
        readme: "Real backup proof passed. release artifacts are unsigned.",
        supportedSources: "Real backup media preview smoke passed. unsigned.",
        ciRelease: "Current release artifacts are unsigned.",
        architecture: "Real backup chat rendering is verified locally.",
        proofEvidence: "",
      },
    });

    expect(result.documentationIssues.map((issue) => issue.blockerId)).toContain(
      "public-proof-evidence",
    );
  });

  it("reports the current repository as honest but not stable-release ready", async () => {
    const result = evaluateReleaseReadiness(await loadReleaseReadinessContext(repoRoot));

    expect(result.stableReleaseReady).toBe(false);
    expect(result.documentationIssues).toEqual([]);
    expect(result.blockers.map((blocker) => blocker.id)).toEqual(["code-signing"]);
  });

  it("derives signing facts from signing readiness instead of static env flags", () => {
    const facts = factsFromEnv(
      {
        APPLE_SIGNING_IDENTITY: "Developer ID Application: Example",
        APPLE_ID: "developer@example.invalid",
        APPLE_PASSWORD: "secret-value",
        APPLE_TEAM_ID: "TEAMID",
      },
      {
        tauriConfig: {
          bundle: {
            windows: {
              certificateThumbprint: "A1B2",
              digestAlgorithm: "sha256",
              timestampUrl: "https://timestamp.example.invalid",
            },
          },
        },
      },
    );

    expect(facts.macosSigning).toBe(true);
    expect(facts.windowsSigning).toBe(true);
  });

  it("formats a readable status report and fails only stable preflight mode", async () => {
    const result = evaluateReleaseReadiness(await loadReleaseReadinessContext(repoRoot));
    const report = formatReleaseReadiness(result);

    expect(report).toContain("stable release: blocked");
    expect(report).toContain("code signing");
    expect(report).not.toContain("real backup media preview proof");
    expect(report).not.toContain("real backup HTML export proof");
    expect(exitCodeForReleaseReadiness(result, { stablePreflight: false })).toBe(0);
    expect(exitCodeForReleaseReadiness(result, { stablePreflight: true })).toBe(1);
  });
});
