import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";

import { applyRuntimeSigningConfig } from "./prepare-tauri-signing-config.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appDir = dirname(scriptDir);
const tauriConfigPath = join(appDir, "src-tauri", "tauri.conf.json");

export const SIGNING_REQUIREMENTS = {
  macos: {
    signingIdentity: ["APPLE_SIGNING_IDENTITY"],
    ciCertificate: ["APPLE_CERTIFICATE", "APPLE_CERTIFICATE_PASSWORD", "KEYCHAIN_PASSWORD"],
    notarizationProfiles: [
      {
        id: "app-store-connect-api",
        label: "App Store Connect API notarization",
        required: ["APPLE_API_ISSUER", "APPLE_API_KEY", "APPLE_API_KEY_PATH"],
      },
      {
        id: "apple-id",
        label: "Apple ID notarization",
        required: ["APPLE_ID", "APPLE_PASSWORD", "APPLE_TEAM_ID"],
      },
    ],
  },
  windows: {
    ciCertificate: ["WINDOWS_CERTIFICATE", "WINDOWS_CERTIFICATE_PASSWORD"],
    azureTrustedSigning: [
      "AZURE_CLIENT_ID",
      "AZURE_CLIENT_SECRET",
      "AZURE_TENANT_ID",
      "AZURE_TRUSTED_SIGNING_ENDPOINT",
      "AZURE_TRUSTED_SIGNING_ACCOUNT",
      "AZURE_TRUSTED_SIGNING_CERTIFICATE_PROFILE",
    ],
  },
};

export async function loadTauriConfig(configPath = tauriConfigPath) {
  return JSON.parse(await readFile(configPath, "utf8"));
}

export function evaluateSigningReadiness({ env = process.env, tauriConfig = {} } = {}) {
  const effectiveConfig = applyRuntimeSigningConfig(tauriConfig, env);

  return {
    macos: evaluateMacosSigning(env, effectiveConfig),
    windows: evaluateWindowsSigning(env, effectiveConfig),
  };
}

export function formatSigningReadiness(readiness) {
  const lines = ["WhatsVault signing readiness"];
  for (const platform of ["macos", "windows"]) {
    const result = readiness[platform];
    lines.push("", `${platform}: ${result.ready ? "ready" : "blocked"}`);
    if (result.activeProfile) {
      lines.push(`profile: ${result.activeProfile}`);
    }
    for (const issue of result.issues) {
      lines.push(`- ${issue}`);
    }
  }

  return `${lines.join("\n")}\n`;
}

export function signingReadinessExitCode(readiness, { strict = false } = {}) {
  if (!strict) {
    return 0;
  }
  return readiness.macos.ready && readiness.windows.ready ? 0 : 1;
}

function evaluateMacosSigning(env, tauriConfig) {
  const signingIdentityMissing = missingEnv(SIGNING_REQUIREMENTS.macos.signingIdentity, env);
  const configuredNotarizationProfile = firstCompleteProfile(
    SIGNING_REQUIREMENTS.macos.notarizationProfiles,
    env,
  );
  const notarizationIssues = configuredNotarizationProfile
    ? []
    : [
        `configure one notarization profile: ${SIGNING_REQUIREMENTS.macos.notarizationProfiles
          .map((profile) => `${profile.label} (${profile.required.join(", ")})`)
          .join(" or ")}`,
      ];
  const ciCertificateMissing = isGithubActions(env)
    ? missingEnv(SIGNING_REQUIREMENTS.macos.ciCertificate, env)
    : [];
  const configIdentity = nonEmptyString(tauriConfig.bundle?.macOS?.signingIdentity);
  const hasIdentity = signingIdentityMissing.length === 0 || configIdentity;
  const issues = [
    ...(!hasIdentity ? missingMessage(signingIdentityMissing) : []),
    ...notarizationIssues,
    ...missingMessage(ciCertificateMissing),
  ];

  return {
    ready: issues.length === 0,
    activeProfile: configuredNotarizationProfile?.id ?? null,
    issues,
  };
}

function evaluateWindowsSigning(env, tauriConfig) {
  const windowsConfig = tauriConfig.bundle?.windows ?? {};
  const certificateConfigReady = [
    windowsConfig.certificateThumbprint,
    windowsConfig.digestAlgorithm,
    windowsConfig.timestampUrl,
  ].every(nonEmptyString);
  const customSignCommand = nonEmptyString(windowsConfig.signCommand);
  const azureProfileReady = SIGNING_REQUIREMENTS.windows.azureTrustedSigning.every((name) =>
    hasEnv(name, env),
  );
  const ciCertificateMissing = isGithubActions(env) && certificateConfigReady
    ? missingEnv(SIGNING_REQUIREMENTS.windows.ciCertificate, env)
    : [];
  const configIssues = [];
  if (!certificateConfigReady && !customSignCommand) {
    configIssues.push(
      "configure bundle.windows certificateThumbprint/digestAlgorithm/timestampUrl or bundle.windows.signCommand",
    );
  }
  if (customSignCommand && customSignCommand.includes("artifact-signing-cli") && !azureProfileReady) {
    configIssues.push(
      `configure Azure Trusted Signing env: ${SIGNING_REQUIREMENTS.windows.azureTrustedSigning.join(", ")}`,
    );
  }
  const issues = [...configIssues, ...missingMessage(ciCertificateMissing)];

  return {
    ready: issues.length === 0,
    activeProfile: customSignCommand
      ? "custom-sign-command"
      : certificateConfigReady
        ? "certificate-thumbprint"
        : null,
    issues,
  };
}

function firstCompleteProfile(profiles, env) {
  return profiles.find((profile) => profile.required.every((name) => hasEnv(name, env)));
}

function missingEnv(names, env) {
  return names.filter((name) => !hasEnv(name, env));
}

function missingMessage(names) {
  return names.map((name) => `missing ${name}`);
}

function hasEnv(name, env) {
  return nonEmptyString(env[name]);
}

function nonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function isGithubActions(env) {
  return env.GITHUB_ACTIONS === "true";
}

async function main() {
  const strict = process.argv.includes("--strict");
  const readiness = evaluateSigningReadiness({
    env: process.env,
    tauriConfig: await loadTauriConfig(),
  });
  process.stdout.write(formatSigningReadiness(readiness));
  process.exitCode = signingReadinessExitCode(readiness, { strict });
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main();
}
