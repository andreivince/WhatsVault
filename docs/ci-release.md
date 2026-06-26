# CI and Release

WhatsVault uses GitHub Actions to keep the desktop app portable across macOS and Windows.

Dependabot version updates are configured in `.github/dependabot.yml` for Rust crates, desktop npm packages, and GitHub Actions. Routine minor and patch updates are grouped by ecosystem, major version bumps are ignored by default, and dependency-update pull requests still need the normal CI, hygiene, and release-readiness checks before merging.

## CI Workflow

`.github/workflows/ci.yml` runs on pull requests, pushes to `main`, and manual dispatch.

The quality job checks:

- Rust formatting
- Rust Clippy warnings
- Rust workspace tests
- frontend unit tests
- public repository hygiene guard
- release readiness honesty guard
- frontend production build
- desktop visual checks
- npm dependency audit

The bundle smoke job builds Tauri bundles on:

- macOS Apple Silicon with `--target aarch64-apple-darwin`
- macOS Intel with `--target x86_64-apple-darwin`
- Windows with the default Windows runner target

## Release Workflow

`.github/workflows/release.yml` runs on version tags matching `v*` and manual dispatch.

It builds draft pre-release artifacts by default for:

- macOS Apple Silicon
- macOS Intel
- Windows

The workflow uses Tauri's official GitHub release action pinned to a published release tag with `projectPath: apps/desktop` so the monorepo layout stays explicit.

The CI smoke workflow uses the same official Tauri action and the same target matrix, but without release metadata. That keeps platform build drift visible before a tagged release is created.

The release workflow reads the app version from `apps/desktop/src-tauri/tauri.conf.json` and fails early if a pushed tag does not match `v<app version>`.

Manual release dispatch has two explicit inputs:

- `publish_release`: when disabled, the GitHub Release remains a draft. When enabled, the workflow publishes the release.
- `stable_release`: when disabled, the published release is marked as a pre-release. When enabled, the stable signing preflight must pass before any matrix build can publish a non-prerelease.

Use these release modes:

- Draft validation: `publish_release=false`, `stable_release=false`
- Public unsigned pre-release: `publish_release=true`, `stable_release=false`
- Public stable release: `publish_release=true`, `stable_release=true`

Do not use the public stable mode until macOS signing, notarization, Windows signing, and clean-machine install checks have passed. Until then, public artifacts must remain pre-release and must keep the unsigned-bundle warning in the release body.

Unsigned pre-release macOS builds use Tauri's ad-hoc signing identity (`-`) when no Developer ID signing identity is configured. That avoids an empty-keychain identity failure in CI, but it is not notarization and does not satisfy the stable-release gate.

Each bundle smoke job also runs `npm run release:checksums` after the Tauri build. This proves the release checksum generator can find the platform bundle outputs before a tagged release is attempted.

When `stable_release` is enabled, a dedicated `stable-signing-preflight` job runs once before the build matrix. That job prepares temporary signing inputs, generates runtime-only Windows Tauri signing config, and runs `npm run release:preflight`. Matrix builds then import platform-specific certificates and build artifacts only after the once-per-release signing gate passes.

Tagged releases upload target-specific checksum manifests next to the Tauri bundles:

- `WhatsVault_macos-aarch64_SHA256SUMS.txt`
- `WhatsVault_macos-x86_64_SHA256SUMS.txt`
- `WhatsVault_windows-x86_64_SHA256SUMS.txt`

The checksum command scans `target/release/bundle` plus target-specific Tauri roots such as `target/<target-triple>/release/bundle` by default. Use `WHATSVAULT_BUNDLE_DIR` only when checking one explicit bundle directory. It writes ignored release metadata to `target/release/release-metadata`.

After checksum generation, the release workflow creates GitHub artifact attestations with `actions/attest@v4` using the same target-specific checksum manifest as the subject list. The checksum manifest remains the single source of truth for downloadable artifact names and digests; the attestation step must not maintain a separate artifact glob.

Users with the GitHub CLI can verify a downloaded artifact against the release provenance:

```sh
gh attestation verify "/path/to/WhatsVault_0.1.0_aarch64.dmg" \
  --repo andreivince/WhatsVault
```

Attestations improve provenance for published artifacts, but they do not replace code signing, notarization, or clean-machine install/open proof.

## Release Readiness Guard

`npm run release:readiness` is an honesty guard. It exits successfully when the repository documents the current release blockers accurately.

`npm run release:preflight` is the stable-release gate. It exits nonzero while any stable-release blocker remains:

1. macOS or Windows signing is not configured

Current pre-alpha state is allowed only when the remaining signing blocker stays visible in README, supported-source, architecture, and release docs.

## Local Package Smoke

Local macOS package checks should verify:

- `npm run tauri build` creates `WhatsVault.app` and a DMG under `target/release/bundle`, or under `target/<target-triple>/release/bundle` for targeted builds.
- `npm run release:checksums` writes checksum metadata under `target/release/release-metadata`.
- The packaged `.app` opens to the source screen from the generated bundle, not only from the Vite dev server.

Current local note: the generated macOS `.app` opens to a nonblank source screen from the bundle. Real local iPhone-backup chat rendering, bounded media preview, and bounded HTML export have passed through the packaged app. On this machine, default MobileSync scanning can still be blocked by macOS filesystem access, so the app exposes a native "Choose folder" fallback instead of requiring terminal launch.

Public-safe proof notes for these local checks live in [proof-evidence.md](proof-evidence.md). Do not commit private screenshots, exported HTML, backup paths, file IDs, contact details, message text, or media filenames as proof artifacts.

## Signing Status

Current release artifacts are not trusted stable builds. The macOS Tauri config intentionally leaves `bundle.macOS.signingIdentity` unset because Tauri can read the signing identity from `APPLE_SIGNING_IDENTITY`, and the repository must not commit certificate-specific values. The release workflow supplies ad-hoc macOS signing for unsigned pre-releases and a real Developer ID identity only when signing secrets are configured. The Windows Tauri signing config is absent until a concrete signing profile is selected.

Run the local signing readiness check:

```sh
cd apps/desktop
npm run release:signing
```

Run the strict stable-release check:

```sh
cd apps/desktop
npm run release:signing:strict
```

The signing checker reports missing variable names only. It must never print certificate contents, passwords, private key material, or local keychain paths.

For macOS direct distribution, follow Tauri's macOS signing and notarization guide: https://v2.tauri.app/distribute/sign/macos/

Stable macOS signing requires:

- `APPLE_SIGNING_IDENTITY`
- one notarization profile:
  - App Store Connect API: `APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`
  - Apple ID app-specific password: `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`
- in GitHub Actions, exported certificate import inputs: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `KEYCHAIN_PASSWORD`
- in GitHub Actions with App Store Connect API notarization, `APPLE_API_KEY_PRIVATE_KEY` is written to a temporary key file and exported as `APPLE_API_KEY_PATH`

For Windows distribution, follow Tauri's Windows signing guide: https://v2.tauri.app/distribute/sign/windows/

Stable Windows signing requires one Tauri Windows signing profile:

- certificate thumbprint profile: `bundle.windows.certificateThumbprint`, `bundle.windows.digestAlgorithm`, and `bundle.windows.timestampUrl`
- or custom signing command profile: `bundle.windows.signCommand`

Do not commit certificate-specific Windows config unless the value is intentionally public for the project. The release workflow can generate the Windows Tauri signing config at runtime from `WINDOWS_CERTIFICATE_THUMBPRINT`, `WINDOWS_DIGEST_ALGORITHM`, and `WINDOWS_TIMESTAMP_URL`.

In GitHub Actions, a certificate-thumbprint profile also needs `WINDOWS_CERTIFICATE` and `WINDOWS_CERTIFICATE_PASSWORD` so the runner can import the certificate before building.

Before a public stable release:

1. Configure macOS signing and notarization.
2. Configure Windows code signing.
3. Run `npm run release:signing:strict`.
4. Update release notes with exact install warnings and known limits.
5. Re-test install/open behavior on clean macOS and Windows machines.
