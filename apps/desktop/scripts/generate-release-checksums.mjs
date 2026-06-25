import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { writeChecksumManifest } from "./release-assets.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appDir = dirname(scriptDir);
const repoRoot = resolve(appDir, "../..");
const bundleDir = process.env.WHATSVAULT_BUNDLE_DIR ?? join(repoRoot, "target", "release", "bundle");
const outputDir = process.env.WHATSVAULT_RELEASE_METADATA_DIR
  ?? join(repoRoot, "target", "release", "release-metadata");
const manifestName = process.env.WHATSVAULT_CHECKSUM_MANIFEST ?? "SHA256SUMS.txt";

try {
  const result = await writeChecksumManifest({ bundleDir, manifestName, outputDir });
  console.log(`Wrote ${result.bundleCount} release checksum(s): ${result.manifestPath}`);
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
