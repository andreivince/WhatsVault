import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";

import { evaluateSigningReadiness } from "./signing-readiness.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appDir = dirname(scriptDir);
const defaultRepoRoot = resolve(appDir, "../..");

const DEFAULT_FACTS = {
  realIphoneBackupProof: true,
  realBackupChatRenderProof: true,
  realBackupMediaPreviewProof: true,
  realBackupHtmlExportProof: true,
  packagedRenderSmoke: "passed",
  macosSigning: false,
  windowsSigning: false,
};

const BLOCKER_DEFINITIONS = [
  {
    id: "real-iphone-backup-proof",
    label: "real iPhone backup proof",
    isBlocking: (facts) => facts.realIphoneBackupProof !== true,
    nextAction: "Run the proof command against a real local MobileSync backup and record aggregate counts only.",
    documentedBy: [
      /real(?: |-)?backup proof pending/i,
      /real iPhone-backup browsing still needs proof/i,
      /real local iPhone backup is still required/i,
      /real local MobileSync backup is still required/i,
    ],
  },
  {
    id: "real-backup-chat-render-proof",
    label: "real backup chat render proof",
    isBlocking: (facts) =>
      facts.realIphoneBackupProof === true && facts.realBackupChatRenderProof !== true,
    nextAction: "Open a real backup chat in the desktop UI and verify text plus media states without committing private artifacts.",
    documentedBy: [
      /real backup chat render proof.*pending/i,
      /real backup chat rendering.*pending/i,
      /real backup chat rendering is still required/i,
      /real chat rendering from that backup still needs/i,
    ],
  },
  {
    id: "real-backup-media-preview-proof",
    label: "real backup media preview proof",
    isBlocking: (facts) =>
      facts.realBackupChatRenderProof === true && facts.realBackupMediaPreviewProof !== true,
    nextAction: "Open media from a real iPhone backup chat in the desktop UI without committing private artifacts.",
    documentedBy: [
      /real(?: |-)?backup media (?:UI )?(?:smoke|proof).*pending/i,
      /real(?: |-)?backup media preview.*pending/i,
      /real(?: |-)?backup media UI proof pending/i,
    ],
  },
  {
    id: "real-backup-html-export-proof",
    label: "real backup HTML export proof",
    isBlocking: (facts) =>
      facts.realBackupChatRenderProof === true && facts.realBackupHtmlExportProof !== true,
    nextAction: "Export one real iPhone backup chat to HTML locally and record only sanitized aggregate evidence.",
    documentedBy: [
      /real(?: |-)?backup export (?:smoke|proof).*pending/i,
      /real(?: |-)?backup HTML export.*pending/i,
      /real(?: |-)?backup export proof pending/i,
    ],
  },
  {
    id: "packaged-render-smoke",
    label: "packaged app render smoke",
    isBlocking: (facts) => facts.packagedRenderSmoke !== "passed",
    nextAction: "Confirm the packaged app opens to a nonblank source screen from a fresh install path.",
    documentedBy: [
      /packaged-window smoke.*blocked/i,
      /local package smoke.*blocked/i,
      /packaged.*app.*rendering/i,
      /browser visual checks.*do not prove packaged/i,
    ],
  },
  {
    id: "code-signing",
    label: "code signing",
    isBlocking: (facts) => facts.macosSigning !== true || facts.windowsSigning !== true,
    nextAction: "Configure macOS signing and notarization plus Windows code signing before a stable release.",
    documentedBy: [
      /release artifacts are unsigned/i,
      /unsigned/i,
      /signing.*notarization/i,
      /Windows code signing/i,
    ],
  },
];

const DOC_PATHS = {
  architecture: "docs/architecture.md",
  ciRelease: "docs/ci-release.md",
  proofEvidence: "docs/proof-evidence.md",
  readme: "README.md",
  supportedSources: "docs/supported-sources.md",
};
const TAURI_CONFIG_PATH = "apps/desktop/src-tauri/tauri.conf.json";

const PROOF_EVIDENCE_PATTERNS = [
  /real local iPhone backup/i,
  /Manifest\.db/i,
  /ChatStorage\.sqlite/i,
  /desktop chat rendering/i,
  /bounded media preview/i,
  /bounded HTML export/i,
  /does not include/i,
];

const PROOF_EVIDENCE_LINK_PATTERN = /docs\/proof-evidence\.md/i;

export async function loadReleaseReadinessContext(repoRoot = defaultRepoRoot, env = process.env) {
  const docs = {};
  await Promise.all(
    Object.entries(DOC_PATHS).map(async ([key, repoPath]) => {
      docs[key] = await readFile(join(repoRoot, repoPath), "utf8");
    }),
  );
  const tauriConfig = JSON.parse(await readFile(join(repoRoot, TAURI_CONFIG_PATH), "utf8"));

  return {
    docs,
    facts: factsFromEnv(env, { tauriConfig }),
  };
}

export function factsFromEnv(env = process.env, { tauriConfig = {} } = {}) {
  const signingReadiness = evaluateSigningReadiness({ env, tauriConfig });

  return {
    ...DEFAULT_FACTS,
    realIphoneBackupProof: env.WHATSVAULT_REAL_BACKUP_PROOF
      ? env.WHATSVAULT_REAL_BACKUP_PROOF === "passed"
      : DEFAULT_FACTS.realIphoneBackupProof,
    realBackupChatRenderProof: env.WHATSVAULT_REAL_BACKUP_CHAT_RENDER_PROOF
      ? env.WHATSVAULT_REAL_BACKUP_CHAT_RENDER_PROOF === "passed"
      : DEFAULT_FACTS.realBackupChatRenderProof,
    realBackupMediaPreviewProof: env.WHATSVAULT_REAL_BACKUP_MEDIA_PREVIEW_PROOF
      ? env.WHATSVAULT_REAL_BACKUP_MEDIA_PREVIEW_PROOF === "passed"
      : DEFAULT_FACTS.realBackupMediaPreviewProof,
    realBackupHtmlExportProof: env.WHATSVAULT_REAL_BACKUP_HTML_EXPORT_PROOF
      ? env.WHATSVAULT_REAL_BACKUP_HTML_EXPORT_PROOF === "passed"
      : DEFAULT_FACTS.realBackupHtmlExportProof,
    packagedRenderSmoke: env.WHATSVAULT_PACKAGED_RENDER_SMOKE
      ? env.WHATSVAULT_PACKAGED_RENDER_SMOKE === "passed" ? "passed" : "blocked"
      : DEFAULT_FACTS.packagedRenderSmoke,
    macosSigning: signingReadiness.macos.ready,
    windowsSigning: signingReadiness.windows.ready,
  };
}

export function evaluateReleaseReadiness(context) {
  const facts = { ...DEFAULT_FACTS, ...context.facts };
  const docs = context.docs ?? {};
  const documentationText = Object.values(docs).join("\n\n");
  const blockers = BLOCKER_DEFINITIONS.filter((definition) => definition.isBlocking(facts)).map((definition) => ({
    id: definition.id,
    label: definition.label,
    nextAction: definition.nextAction,
  }));
  const documentationIssues = blockers
    .filter((blocker) => {
      const definition = BLOCKER_DEFINITIONS.find((candidate) => candidate.id === blocker.id);
      return !definition.documentedBy.some((pattern) => pattern.test(documentationText));
    })
    .map((blocker) => ({
      blockerId: blocker.id,
      message: `Missing public documentation for blocker: ${blocker.label}.`,
    }));
  documentationIssues.push(...evaluateProofEvidenceDocumentation(facts, docs));

  return {
    blockers,
    documentationIssues,
    stableReleaseReady: blockers.length === 0 && documentationIssues.length === 0,
  };
}

export function formatReleaseReadiness(result) {
  const lines = [
    "WhatsVault release readiness",
    `stable release: ${result.stableReleaseReady ? "ready" : "blocked"}`,
  ];

  if (result.blockers.length > 0) {
    lines.push("", "Blockers:");
    for (const blocker of result.blockers) {
      lines.push(`- ${blocker.label}: ${blocker.nextAction}`);
    }
  }

  if (result.documentationIssues.length > 0) {
    lines.push("", "Documentation issues:");
    for (const issue of result.documentationIssues) {
      lines.push(`- ${issue.message}`);
    }
  }

  if (!result.stableReleaseReady && result.documentationIssues.length === 0) {
    lines.push("", "Current status is documented honestly for pre-alpha development.");
  }

  return `${lines.join("\n")}\n`;
}

export function exitCodeForReleaseReadiness(result, { stablePreflight = false } = {}) {
  if (result.documentationIssues.length > 0) {
    return 1;
  }

  if (stablePreflight && !result.stableReleaseReady) {
    return 1;
  }

  return 0;
}

function evaluateProofEvidenceDocumentation(facts, docs) {
  if (
    facts.realIphoneBackupProof !== true ||
    facts.realBackupChatRenderProof !== true ||
    facts.realBackupMediaPreviewProof !== true ||
    facts.realBackupHtmlExportProof !== true
  ) {
    return [];
  }

  const issues = [];
  const proofEvidence = docs.proofEvidence ?? "";
  const publicSurface = [docs.readme, docs.ciRelease, docs.supportedSources].filter(Boolean).join("\n\n");

  if (!PROOF_EVIDENCE_LINK_PATTERN.test(publicSurface)) {
    issues.push({
      blockerId: "public-proof-evidence",
      message: "Passed real-backup proof claims must link to docs/proof-evidence.md from the public docs.",
    });
  }

  const missingPatterns = PROOF_EVIDENCE_PATTERNS.filter((pattern) => !pattern.test(proofEvidence));
  if (missingPatterns.length > 0) {
    issues.push({
      blockerId: "public-proof-evidence",
      message: "docs/proof-evidence.md must summarize sanitized real-backup proof coverage.",
    });
  }

  return issues;
}

async function main() {
  const stablePreflight = process.argv.includes("--stable");
  const result = evaluateReleaseReadiness(await loadReleaseReadinessContext(defaultRepoRoot));
  process.stdout.write(formatReleaseReadiness(result));
  process.exitCode = exitCodeForReleaseReadiness(result, { stablePreflight });
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main();
}
