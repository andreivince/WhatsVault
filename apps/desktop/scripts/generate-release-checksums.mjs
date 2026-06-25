import { readdir } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { writeChecksumManifest } from "./release-assets.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appDir = dirname(scriptDir);
const repoRoot = resolve(appDir, "../..");
const explicitBundleDir = process.env.WHATSVAULT_BUNDLE_DIR;
const outputDir = process.env.WHATSVAULT_RELEASE_METADATA_DIR
  ?? join(repoRoot, "target", "release", "release-metadata");
const manifestName = process.env.WHATSVAULT_CHECKSUM_MANIFEST ?? "SHA256SUMS.txt";

try {
  const bundleDirs = explicitBundleDir ? [explicitBundleDir] : await defaultBundleDirs(repoRoot);
  const result = await writeChecksumManifest({ bundleDirs, manifestName, outputDir });
  console.log(`Wrote ${result.bundleCount} release checksum(s): ${result.manifestPath}`);
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}

async function defaultBundleDirs(root) {
  const targetRoot = join(root, "target");
  const dirs = [join(targetRoot, "release", "bundle")];

  let entries = [];
  try {
    entries = await readdir(targetRoot, { withFileTypes: true });
  } catch (error) {
    if (error?.code === "ENOENT") {
      return dirs;
    }
    throw error;
  }

  for (const entry of entries) {
    if (entry.isDirectory()) {
      dirs.push(join(targetRoot, entry.name, "release", "bundle"));
    }
  }

  return dirs;
}
