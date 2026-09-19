import { copyFile, mkdtemp, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const appDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = await mkdtemp(join(tmpdir(), "whatsvault-icons-"));
try {
  const result = spawnSync(process.execPath, [
    join(appDir, "node_modules/@tauri-apps/cli/tauri.js"),
    "icon", join(appDir, "app-icon.svg"), "--output", output,
  ], { cwd: appDir, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`Icon generation failed (${result.status})`);
  // Tauri puts desktop icons at the root; mobile assets are in subdirectories.
  for (const entry of await readdir(output, { withFileTypes: true })) {
    if (entry.isFile()) await copyFile(join(output, entry.name), join(appDir, "src-tauri/icons", entry.name));
  }
} finally {
  await rm(output, { recursive: true, force: true });
}
