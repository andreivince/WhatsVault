import { readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appDir = dirname(scriptDir);
const tauriConfigPath = join(appDir, "src-tauri", "tauri.conf.json");

export function applyRuntimeSigningConfig(config, env = process.env) {
  const nextConfig = structuredClone(config);
  nextConfig.bundle = nextConfig.bundle ?? {};

  const windowsCertificateProfile = windowsCertificateProfileFromEnv(env);
  const windowsSignCommand = stringEnv(env, "WINDOWS_SIGN_COMMAND");
  if (windowsCertificateProfile || windowsSignCommand) {
    nextConfig.bundle.windows = {
      ...(nextConfig.bundle.windows ?? {}),
    };
    if (windowsCertificateProfile) {
      Object.assign(nextConfig.bundle.windows, windowsCertificateProfile);
      delete nextConfig.bundle.windows.signCommand;
    } else if (windowsSignCommand) {
      nextConfig.bundle.windows.signCommand = windowsSignCommand;
      delete nextConfig.bundle.windows.certificateThumbprint;
      delete nextConfig.bundle.windows.digestAlgorithm;
      delete nextConfig.bundle.windows.timestampUrl;
    }
  }

  return nextConfig;
}

export function runtimeSigningConfigSummary(config, env = process.env) {
  const windowsBefore = config.bundle?.windows ?? {};
  const windowsAfter = applyRuntimeSigningConfig(config, env).bundle?.windows ?? {};
  const windowsProfile = windowsAfter.signCommand
    ? "custom sign command"
    : windowsAfter.certificateThumbprint && windowsAfter.digestAlgorithm && windowsAfter.timestampUrl
      ? "certificate thumbprint"
      : "not configured";

  return {
    changed: JSON.stringify(windowsBefore) !== JSON.stringify(windowsAfter),
    windowsProfile,
  };
}

async function main() {
  const config = JSON.parse(await readFile(tauriConfigPath, "utf8"));
  const nextConfig = applyRuntimeSigningConfig(config, process.env);
  const summary = runtimeSigningConfigSummary(config, process.env);
  await writeFile(tauriConfigPath, `${JSON.stringify(nextConfig, null, 2)}\n`);
  process.stdout.write(
    [
      "WhatsVault runtime signing config",
      `windows: ${summary.windowsProfile}`,
      `changed: ${summary.changed ? "yes" : "no"}`,
      "",
    ].join("\n"),
  );
}

function windowsCertificateProfileFromEnv(env) {
  const certificateThumbprint = stringEnv(env, "WINDOWS_CERTIFICATE_THUMBPRINT");
  const digestAlgorithm = stringEnv(env, "WINDOWS_DIGEST_ALGORITHM");
  const timestampUrl = stringEnv(env, "WINDOWS_TIMESTAMP_URL");
  if (!certificateThumbprint && !digestAlgorithm && !timestampUrl) {
    return null;
  }
  if (!certificateThumbprint || !digestAlgorithm || !timestampUrl) {
    return null;
  }

  return {
    certificateThumbprint,
    digestAlgorithm,
    timestampUrl,
  };
}

function stringEnv(env, name) {
  const value = env[name];
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main();
}
