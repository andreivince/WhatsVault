import { describe, expect, it } from "vitest";

import {
  evaluateSigningReadiness,
  formatSigningReadiness,
  signingReadinessExitCode,
} from "./signing-readiness.mjs";

const unsignedConfig = {
  bundle: {},
};

const windowsCertificateConfig = {
  bundle: {
    windows: {
      certificateThumbprint: "A1B2",
      digestAlgorithm: "sha256",
      timestampUrl: "https://timestamp.example.invalid",
    },
  },
};

describe("signing readiness", () => {
  it("reports missing macOS and Windows signing inputs without exposing values", () => {
    const readiness = evaluateSigningReadiness({
      env: {},
      tauriConfig: unsignedConfig,
    });
    const report = formatSigningReadiness(readiness);

    expect(readiness.macos.ready).toBe(false);
    expect(readiness.windows.ready).toBe(false);
    expect(report).toContain("missing APPLE_SIGNING_IDENTITY");
    expect(report).toContain("bundle.windows");
    expect(report).not.toContain("secret-value");
    expect(signingReadinessExitCode(readiness, { strict: false })).toBe(0);
    expect(signingReadinessExitCode(readiness, { strict: true })).toBe(1);
  });

  it("accepts local macOS signing with Apple ID notarization credentials", () => {
    const readiness = evaluateSigningReadiness({
      env: {
        APPLE_SIGNING_IDENTITY: "Developer ID Application: Example",
        APPLE_ID: "developer@example.invalid",
        APPLE_PASSWORD: "secret-value",
        APPLE_TEAM_ID: "TEAMID",
      },
      tauriConfig: unsignedConfig,
    });

    expect(readiness.macos).toMatchObject({
      ready: true,
      activeProfile: "apple-id",
      issues: [],
    });
  });

  it("requires certificate import inputs for macOS GitHub Actions signing", () => {
    const missing = evaluateSigningReadiness({
      env: {
        GITHUB_ACTIONS: "true",
        APPLE_SIGNING_IDENTITY: "Developer ID Application: Example",
        APPLE_API_ISSUER: "issuer",
        APPLE_API_KEY: "key",
        APPLE_API_KEY_PATH: "/tmp/AuthKey.p8",
      },
      tauriConfig: unsignedConfig,
    });
    const ready = evaluateSigningReadiness({
      env: {
        GITHUB_ACTIONS: "true",
        APPLE_SIGNING_IDENTITY: "Developer ID Application: Example",
        APPLE_API_ISSUER: "issuer",
        APPLE_API_KEY: "key",
        APPLE_API_KEY_PATH: "/tmp/AuthKey.p8",
        APPLE_CERTIFICATE: "base64",
        APPLE_CERTIFICATE_PASSWORD: "secret-value",
        KEYCHAIN_PASSWORD: "secret-value",
      },
      tauriConfig: unsignedConfig,
    });

    expect(missing.macos.ready).toBe(false);
    expect(missing.macos.issues).toEqual([
      "missing APPLE_CERTIFICATE",
      "missing APPLE_CERTIFICATE_PASSWORD",
      "missing KEYCHAIN_PASSWORD",
    ]);
    expect(ready.macos.ready).toBe(true);
  });

  it("requires Windows signing config and CI certificate import inputs when relevant", () => {
    const localReady = evaluateSigningReadiness({
      env: {},
      tauriConfig: windowsCertificateConfig,
    });
    const ciMissing = evaluateSigningReadiness({
      env: { GITHUB_ACTIONS: "true" },
      tauriConfig: windowsCertificateConfig,
    });
    const ciReady = evaluateSigningReadiness({
      env: {
        GITHUB_ACTIONS: "true",
        WINDOWS_CERTIFICATE: "base64",
        WINDOWS_CERTIFICATE_PASSWORD: "secret-value",
      },
      tauriConfig: windowsCertificateConfig,
    });

    expect(localReady.windows.ready).toBe(true);
    expect(ciMissing.windows.ready).toBe(false);
    expect(ciMissing.windows.issues).toEqual([
      "missing WINDOWS_CERTIFICATE",
      "missing WINDOWS_CERTIFICATE_PASSWORD",
    ]);
    expect(ciReady.windows.ready).toBe(true);
  });

  it("derives Windows signing config from runtime environment values", () => {
    const readiness = evaluateSigningReadiness({
      env: {
        WINDOWS_CERTIFICATE_THUMBPRINT: "A1B2",
        WINDOWS_DIGEST_ALGORITHM: "sha256",
        WINDOWS_TIMESTAMP_URL: "https://timestamp.example.invalid",
      },
      tauriConfig: unsignedConfig,
    });

    expect(readiness.windows).toMatchObject({
      ready: true,
      activeProfile: "certificate-thumbprint",
      issues: [],
    });
  });
});
