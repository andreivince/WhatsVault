import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { access, mkdir, readdir, writeFile } from "node:fs/promises";
import { basename, join, relative, resolve } from "node:path";

const bundleExtensions = [
  ".AppImage",
  ".app.tar.gz",
  ".deb",
  ".dmg",
  ".exe",
  ".msi",
  ".rpm",
  ".tar.gz",
  ".zip",
];

export async function findReleaseBundles(bundleDir) {
  const root = resolve(bundleDir);
  const entries = await collectFiles(root);

  return entries
    .filter((filePath) => isReleaseBundle(filePath))
    .sort((left, right) => relative(root, left).localeCompare(relative(root, right)));
}

export async function findReleaseBundlesFromRoots(bundleDirs) {
  const roots = [...new Set(bundleDirs.map((bundleDir) => resolve(bundleDir)))];
  const bundleGroups = await Promise.all(
    roots.map(async (root) => {
      if (!(await pathExists(root))) {
        return [];
      }

      return findReleaseBundles(root);
    }),
  );

  return bundleGroups
    .flat()
    .sort((left, right) => basename(left).localeCompare(basename(right)) || left.localeCompare(right));
}

export async function writeChecksumManifest({
  bundleDir,
  bundleDirs,
  outputDir,
  manifestName = "SHA256SUMS.txt",
} = {}) {
  const roots = bundleDirs ?? (bundleDir ? [bundleDir] : []);
  if (roots.length === 0) {
    throw new Error("bundleDir or bundleDirs is required.");
  }

  if (!outputDir) {
    throw new Error("outputDir is required.");
  }

  const bundles = await findReleaseBundlesFromRoots(roots);
  if (bundles.length === 0) {
    throw new Error(`No release bundle files found in ${roots.join(", ")}.`);
  }

  const lines = [];
  for (const bundlePath of bundles) {
    lines.push(`${await sha256File(bundlePath)}  ${basename(bundlePath)}`);
  }

  await mkdir(outputDir, { recursive: true });
  const manifestPath = join(outputDir, manifestName);
  await writeFile(manifestPath, `${lines.join("\n")}\n`, "utf8");

  return {
    bundleCount: bundles.length,
    bundles,
    manifestPath,
  };
}

async function collectFiles(root) {
  const dirents = await readdir(root, { withFileTypes: true });
  const files = [];

  for (const dirent of dirents) {
    const entryPath = join(root, dirent.name);

    if (dirent.isDirectory()) {
      files.push(...await collectFiles(entryPath));
      continue;
    }

    if (dirent.isFile()) {
      files.push(entryPath);
    }
  }

  return files;
}

async function pathExists(path) {
  try {
    await access(path);
    return true;
  } catch (error) {
    if (error?.code === "ENOENT") {
      return false;
    }
    throw error;
  }
}

function isReleaseBundle(filePath) {
  const name = basename(filePath);
  return bundleExtensions.some((extension) => name.endsWith(extension));
}

async function sha256File(filePath) {
  const hash = createHash("sha256");
  const stream = createReadStream(filePath);

  for await (const chunk of stream) {
    hash.update(chunk);
  }

  return hash.digest("hex");
}
