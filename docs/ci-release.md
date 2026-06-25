# CI and Release

WhatsVault uses GitHub Actions to keep the desktop app portable across macOS and Windows.

## CI Workflow

`.github/workflows/ci.yml` runs on pull requests, pushes to `main`, and manual dispatch.

The quality job checks:

- Rust formatting
- Rust Clippy warnings
- Rust workspace tests
- frontend unit tests
- public repository hygiene guard
- frontend production build
- npm dependency audit

The bundle smoke job builds Tauri bundles on:

- macOS Apple Silicon with `--target aarch64-apple-darwin`
- macOS Intel with `--target x86_64-apple-darwin`
- Windows with the default Windows runner target

## Release Workflow

`.github/workflows/release.yml` runs on version tags matching `v*` and manual dispatch.

It builds draft pre-release artifacts for:

- macOS Apple Silicon
- macOS Intel
- Windows

The workflow uses Tauri's official GitHub release action pinned to a published release tag with `projectPath: apps/desktop` so the monorepo layout stays explicit.

The CI smoke workflow uses the same official Tauri action and the same target matrix, but without release metadata. That keeps platform build drift visible before a tagged release is created.

The release workflow reads the app version from `apps/desktop/src-tauri/tauri.conf.json` and fails early if a pushed tag does not match `v<app version>`.

Each bundle smoke job also runs `npm run release:checksums` after the Tauri build. This proves the release checksum generator can find the platform bundle outputs before a tagged release is attempted.

Tagged releases upload target-specific checksum manifests next to the Tauri bundles:

- `WhatsVault_macos-aarch64_SHA256SUMS.txt`
- `WhatsVault_macos-x86_64_SHA256SUMS.txt`
- `WhatsVault_windows-x86_64_SHA256SUMS.txt`

The checksum command reads bundle output from `target/release/bundle` by default and writes ignored release metadata to `target/release/release-metadata`.

## Local Package Smoke

Local macOS package checks should verify:

- `npm run tauri build` creates `WhatsVault.app` and a DMG under `target/release/bundle`.
- `npm run release:checksums` writes checksum metadata under `target/release/release-metadata`.
- The packaged `.app` opens to the source screen from the generated bundle, not only from the Vite dev server.

Current local note: on this macOS 26.5.1 machine, Tauri 2.11.3 with Tao 0.35.3 opens a real app window but the WebView surface captures as blank. This matches the upstream Tauri issue [tauri-apps/tauri#15517](https://github.com/tauri-apps/tauri/issues/15517). Until Tauri/Tao ships a stable fix or the project pins an upstream-reviewed workaround, local package smoke on macOS 26 must be recorded as blocked, not passed. Browser visual checks still prove the React UI, but they do not prove packaged app rendering.

## Signing Status

Current release artifacts are unsigned. The macOS Tauri config intentionally leaves `bundle.macOS.signingIdentity` unset until Developer ID signing and notarization are configured. Before a public stable release:

1. Configure macOS signing and notarization.
2. Configure Windows code signing.
3. Update release notes with exact install warnings and known limits.
4. Re-test install/open behavior on clean macOS and Windows machines.
