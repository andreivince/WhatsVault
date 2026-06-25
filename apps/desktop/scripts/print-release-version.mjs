import { appendFile } from "node:fs/promises";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appDir = dirname(scriptDir);
const configPath = join(appDir, "src-tauri", "tauri.conf.json");
const config = JSON.parse(await readFile(configPath, "utf8"));
const version = config.version;

if (!version || typeof version !== "string") {
  throw new Error(`Missing Tauri app version in ${configPath}.`);
}

const output = `version=${version}\ntag=v${version}\n`;

if (process.env.GITHUB_OUTPUT) {
  await appendFile(process.env.GITHUB_OUTPUT, output, "utf8");
}

process.stdout.write(output);
