import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { afterEach, describe, expect, it } from "vitest";

import { findReleaseBundles, writeChecksumManifest } from "./release-assets.mjs";

const tempRoots = [];

afterEach(async () => {
  await Promise.all(tempRoots.map((root) => rm(root, { force: true, recursive: true })));
  tempRoots.length = 0;
});

async function tempRoot() {
  const root = await mkdtemp(join(tmpdir(), "whatsvault-release-assets-"));
  tempRoots.push(root);
  return root;
}

describe("release asset helpers", () => {
  it("finds only release bundle files in deterministic order", async () => {
    const root = await tempRoot();
    const bundleDir = join(root, "bundle");
    await mkdir(join(bundleDir, "dmg"), { recursive: true });
    await mkdir(join(bundleDir, "nsis"), { recursive: true });
    await mkdir(join(bundleDir, "share"), { recursive: true });
    await writeFile(join(bundleDir, "dmg", "WhatsVault_0.1.0_aarch64.dmg"), "dmg");
    await writeFile(join(bundleDir, "nsis", "WhatsVault_0.1.0_x64-setup.exe"), "exe");
    await writeFile(join(bundleDir, "share", "bundle_dmg.sh"), "ignored");

    await expect(findReleaseBundles(bundleDir)).resolves.toEqual([
      join(bundleDir, "dmg", "WhatsVault_0.1.0_aarch64.dmg"),
      join(bundleDir, "nsis", "WhatsVault_0.1.0_x64-setup.exe"),
    ]);
  });

  it("writes a SHA-256 manifest for discovered release bundles", async () => {
    const root = await tempRoot();
    const bundleDir = join(root, "bundle", "dmg");
    const outputDir = join(root, "release-metadata");
    await mkdir(bundleDir, { recursive: true });
    await writeFile(join(bundleDir, "WhatsVault_0.1.0_aarch64.dmg"), "abc");

    const result = await writeChecksumManifest({
      bundleDir: join(root, "bundle"),
      outputDir,
    });

    await expect(readFile(result.manifestPath, "utf8")).resolves.toBe(
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  WhatsVault_0.1.0_aarch64.dmg\n",
    );
    expect(result.bundleCount).toBe(1);
  });
});
