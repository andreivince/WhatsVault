import { existsSync, mkdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appDir = dirname(scriptDir);
const inputDir = join(appDir, "target", "readme-demo", "test-results");
const outputDir = join(appDir, "target", "readme-demo", "rendered");
const outputPath = join(outputDir, "whatsvault-readme-demo.mp4");
const subtitlePath = join(appDir, "demo", "readme-demo.srt");
const burnSubtitles = process.env.WHATSVAULT_DEMO_BURN_SUBS === "1";

if (!existsSync(inputDir)) {
  console.error("Missing Playwright demo trace output. Run `npm run demo:record` first.");
  process.exit(1);
}

if (!existsSync(subtitlePath)) {
  console.error(`Missing subtitle file: ${subtitlePath}`);
  process.exit(1);
}

mkdirSync(outputDir, { recursive: true });

const npx = process.platform === "win32" ? "npx.cmd" : "npx";
const args = [
  "playwright-recast",
  "-i",
  inputDir,
  "-o",
  outputPath,
  "--speed-idle",
  "1.0",
  "--speed-action",
  "1.0",
];

if (burnSubtitles) {
  args.push("--srt", subtitlePath, "--burn-subs");
}

const result = spawnSync(
  npx,
  args,
  { cwd: appDir, stdio: "inherit" },
);

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

console.log(`Rendered README demo video: ${outputPath}`);
