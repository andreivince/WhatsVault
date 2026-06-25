import { describe, expect, it } from "vitest";

import {
  applyRuntimeSigningConfig,
  runtimeSigningConfigSummary,
} from "./prepare-tauri-signing-config.mjs";

const baseConfig = {
  bundle: {
    active: true,
  },
};

describe("runtime Tauri signing config", () => {
  it("leaves config unchanged when no signing profile env is present", () => {
    const next = applyRuntimeSigningConfig(baseConfig, {});

    expect(next).toEqual(baseConfig);
    expect(next).not.toBe(baseConfig);
  });

  it("applies a Windows certificate-thumbprint profile from environment", () => {
    const next = applyRuntimeSigningConfig(baseConfig, {
      WINDOWS_CERTIFICATE_THUMBPRINT: "A1B2",
      WINDOWS_DIGEST_ALGORITHM: "sha256",
      WINDOWS_TIMESTAMP_URL: "https://timestamp.example.invalid",
      WINDOWS_CERTIFICATE_PASSWORD: "secret-value",
    });
    const summary = runtimeSigningConfigSummary(baseConfig, {
      WINDOWS_CERTIFICATE_THUMBPRINT: "A1B2",
      WINDOWS_DIGEST_ALGORITHM: "sha256",
      WINDOWS_TIMESTAMP_URL: "https://timestamp.example.invalid",
      WINDOWS_CERTIFICATE_PASSWORD: "secret-value",
    });

    expect(next.bundle.windows).toEqual({
      certificateThumbprint: "A1B2",
      digestAlgorithm: "sha256",
      timestampUrl: "https://timestamp.example.invalid",
    });
    expect(summary).toEqual({
      changed: true,
      windowsProfile: "certificate thumbprint",
    });
    expect(JSON.stringify(next)).not.toContain("secret-value");
  });

  it("does not write partial Windows certificate profile config", () => {
    const next = applyRuntimeSigningConfig(baseConfig, {
      WINDOWS_CERTIFICATE_THUMBPRINT: "A1B2",
      WINDOWS_TIMESTAMP_URL: "https://timestamp.example.invalid",
    });

    expect(next).toEqual(baseConfig);
  });

  it("prefers a complete Windows certificate profile over a custom sign command", () => {
    const next = applyRuntimeSigningConfig(
      {
        bundle: {
          windows: {
            signCommand: "old signer %1",
          },
        },
      },
      {
        WINDOWS_CERTIFICATE_THUMBPRINT: "A1B2",
        WINDOWS_DIGEST_ALGORITHM: "sha256",
        WINDOWS_TIMESTAMP_URL: "https://timestamp.example.invalid",
        WINDOWS_SIGN_COMMAND: "custom signer %1",
      },
    );

    expect(next.bundle.windows).toEqual({
      certificateThumbprint: "A1B2",
      digestAlgorithm: "sha256",
      timestampUrl: "https://timestamp.example.invalid",
    });
  });

  it("applies a Windows custom sign command when no certificate profile is present", () => {
    const next = applyRuntimeSigningConfig(baseConfig, {
      WINDOWS_SIGN_COMMAND: "custom signer %1",
    });

    expect(next.bundle.windows).toEqual({
      signCommand: "custom signer %1",
    });
  });
});
