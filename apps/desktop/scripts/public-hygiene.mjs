import { execFile } from "node:child_process";
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

  return issues.sort(
    (left, right) => left.path.localeCompare(right.path) || left.code.localeCompare(right.code),
  );
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
