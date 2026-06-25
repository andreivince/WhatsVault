import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { access, readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";
import { promisify } from "node:util";

import {
  auditTextContent,
  auditTrackedFileNames,
  hasPrivateDemoText,
  isTextFile,
} from "./privacy-rules.mjs";

const execFileAsync = promisify(execFile);
const scriptDir = dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = resolve(scriptDir, "../../..");

export { auditTextContent, auditTrackedFileNames, hasPrivateDemoText } from "./privacy-rules.mjs";

const DEMO_ASSET_MANIFEST_PATH = "docs/assets/demo-assets-manifest.json";

export async function gitPublicCandidateFiles(repoRoot = defaultRepoRoot) {
  const { stdout } = await execFileAsync("git", ["ls-files", "-z", "--cached", "--others", "--exclude-standard"], {
    cwd: repoRoot,
    maxBuffer: 10 * 1024 * 1024,
  });
  const repoPaths = stdout.split("\0").filter(Boolean).sort();
  const existing = await Promise.all(
    repoPaths.map(async (repoPath) => {
      try {
        await access(join(repoRoot, repoPath));
        return repoPath;
      } catch (error) {
        if (error?.code === "ENOENT") {
          return null;
        }
        throw error;
      }
    }),
  );
  return existing.filter(Boolean);
}

export async function auditPublicRepository(repoRoot = defaultRepoRoot) {
  const repoPaths = await gitPublicCandidateFiles(repoRoot);
  const issues = auditTrackedFileNames(repoPaths);

  await Promise.all(
    repoPaths.filter(isTextFile).map(async (repoPath) => {
      const content = await readFile(join(repoRoot, repoPath), "utf8");
      issues.push(...auditTextContent(repoPath, content));
    }),
  );
  issues.push(...(await auditDemoAssetManifest(repoRoot)));

  return issues.sort(
    (left, right) => left.path.localeCompare(right.path) || left.code.localeCompare(right.code),
  );
}

export async function auditDemoAssetManifest(repoRoot = defaultRepoRoot) {
  const issues = [];
  let manifest;

  try {
    manifest = JSON.parse(await readFile(join(repoRoot, DEMO_ASSET_MANIFEST_PATH), "utf8"));
  } catch {
    return [
      {
        code: "demo-asset-manifest-unreadable",
        path: DEMO_ASSET_MANIFEST_PATH,
        message: "demo asset manifest is missing or invalid",
      },
    ];
  }

  const assets = Array.isArray(manifest.assets) ? manifest.assets : [];
  if (assets.length === 0) {
    issues.push({
      code: "demo-asset-manifest-empty",
      path: DEMO_ASSET_MANIFEST_PATH,
      message: "demo asset manifest must list synthetic demo assets",
    });
  }

  for (const asset of assets) {
    const assetPath = typeof asset.path === "string" ? asset.path : "";
    const expectedHash = typeof asset.sha256 === "string" ? asset.sha256.toLowerCase() : "";
    const source = typeof asset.source === "string" ? asset.source.toLowerCase() : "";
    if (!assetPath || !/^[a-f0-9]{64}$/.test(expectedHash)) {
      issues.push({
        code: "demo-asset-manifest-entry-invalid",
        path: assetPath || DEMO_ASSET_MANIFEST_PATH,
        message: "demo asset manifest entry must include a repo path and SHA-256 hash",
      });
      continue;
    }

    if (!source.includes("synthetic")) {
      issues.push({
        code: "demo-asset-source-not-synthetic",
        path: assetPath,
        message: "demo asset manifest entry must state synthetic provenance",
      });
    }

    let bytes;
    try {
      bytes = await readFile(join(repoRoot, assetPath));
    } catch {
      issues.push({
        code: "demo-asset-missing",
        path: assetPath,
        message: "demo asset listed in manifest is missing",
      });
      continue;
    }

    const actualHash = createHash("sha256").update(bytes).digest("hex");
    if (actualHash !== expectedHash) {
      issues.push({
        code: "demo-asset-hash-mismatch",
        path: assetPath,
        message: "demo asset bytes do not match the synthetic asset manifest",
      });
    }
  }

  return issues;
}

function formatIssues(issues) {
  return issues.map((issue) => `- ${issue.path}: ${issue.message} (${issue.code})`).join("\n");
}

async function main() {
  const issues = await auditPublicRepository(defaultRepoRoot);
  if (issues.length > 0) {
    console.error("Public repository hygiene check failed:");
    console.error(formatIssues(issues));
    process.exitCode = 1;
    return;
  }

  console.log("Public repository hygiene check passed.");
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main();
}
