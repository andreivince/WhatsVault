# WhatsVault

WhatsVault is a local-first desktop app for browsing WhatsApp chats and media from iPhone backups. It runs on your computer and does not upload your messages, backups, contacts, or media.

Status: pre-alpha desktop viewer. WhatsApp export ZIP viewing works as the first local source path. The desktop app can scan default iPhone backup folders, show local backup/WhatsApp status, and route a selected ready backup into the shared chat-list/import UI path. Real local iPhone-backup proof has passed for `Manifest.db`, `ChatStorage.sqlite`, desktop chat rendering, bounded media preview, and bounded HTML export without committing private artifacts. Stable release remains blocked by signing/notarization and release hardening.

![Synthetic WhatsVault desktop demo showing local backup chats, search, image media preview, date filtering, and export controls](docs/assets/whatsvault-synthetic-demo.png)

Screenshot and video use synthetic demo data only.

[Watch the synthetic 18-second demo video](docs/assets/whatsvault-readme-demo.mp4)

## Goal

The intended product path is simple:

1. Open WhatsVault.
2. Pick a local iPhone Finder, iTunes, or Apple Devices backup.
3. Browse WhatsApp chats and media locally.
4. Search messages.
5. Export a selected chat to self-contained HTML.

No terminal workflow, manual backup-ID hunting, or copying SQLite files by hand should be required in the finished app.

## Privacy

WhatsVault is designed around local-only processing:

- No account.
- No cloud sync.
- No message upload.
- No hosted parsing service.
- No analytics on private chat content.

Real backups, exported chats, SQLite databases, media, and private fixtures must stay out of the repository. See `.gitignore` before adding any sample data.

See [SECURITY.md](SECURITY.md) for the security reporting policy.

## Downloads

Pre-alpha desktop builds are published on [GitHub Releases](https://github.com/andreivince/WhatsVault/releases) when release automation passes. Current bundles are not notarized or Developer ID signed on macOS and are not code signed on Windows, so they should be treated as early tester builds, not stable end-user releases.

## Current Direction

The selected app direction is Tauri v2 with React and TypeScript:

- Tauri keeps the app cross-platform for macOS and Windows while allowing a Rust core for local filesystem and SQLite work.
- React and TypeScript keep the interface portable, testable, and fast to iterate.
- Shared parser/model logic should sit behind stable internal APIs so the UI never depends directly on WhatsApp SQLite or export-file quirks.

The first supported source target remains iPhone local backups. A WhatsApp exported chat ZIP can be useful as a development source, but it must not replace the iPhone-backup roadmap.

The current desktop app uses the exported chat ZIP path first so the viewer, layout, search, media-preview, and HTML-export flow can be exercised without requiring a local iPhone backup.

## Roadmap

See [ROADMAP.md](ROADMAP.md).

## Architecture

See [docs/architecture.md](docs/architecture.md).

## Supported Sources

See [docs/supported-sources.md](docs/supported-sources.md) for the current source matrix.

## Proof Evidence

See [docs/proof-evidence.md](docs/proof-evidence.md) for the public-safe evidence boundary behind local real-backup proof claims.

## Demo Video

README demo videos must be generated from synthetic English demo data, not private chats or backups.

See [docs/demo-video.md](docs/demo-video.md) for the Playwright plus `playwright-recast` workflow that produces the committed synthetic MP4.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md), [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md), [CHANGELOG.md](CHANGELOG.md), [docs/troubleshooting.md](docs/troubleshooting.md), and [docs/ci-release.md](docs/ci-release.md).

## Verified So Far

- Public-safe repository foundation is in place.
- Shared Rust core crate exists at `crates/whatsvault-core`.
- WhatsApp export ZIP import is tested with synthetic fixtures.
- A private ignored test hook validates a local export ZIP without printing chat content.
- A Tauri desktop app exists at `apps/desktop`.
- The desktop app can open a WhatsApp export ZIP through a native file picker, parse the transcript in Rust into a bounded latest-message window, render the chat timeline, search loaded messages, preview bounded images, stickers, audio, video, and documents from the archive, open image previews, and export the loaded window to self-contained HTML.
- The desktop app can scan default iPhone backup folders and show safe display metadata plus WhatsApp file-detection status without showing local backup paths in the UI.
- Default iPhone backup root construction is tested for macOS, Windows Microsoft Store Apple Devices or iTunes, and Windows legacy iTunes locations.
- iPhone backup discovery and `Manifest.db` mapping are tested with synthetic fixtures.
- Modern iPhone backup `fileID` values are resolved to physical backup files through the centralized core resolver.
- Synthetic `ChatStorage.sqlite` files can be summarized for message, chat, and media-item counts through the shared core crate.
- Synthetic `ChatStorage.sqlite` files can list chats by latest message and import one selected chat into the same normalized `ChatImport` model used by the desktop timeline.
- The Tauri command layer can resolve a selected backup's `ChatStorage.sqlite` through `Manifest.db` and call the core chat-list/import API.
- The React app can select a ready backup, show discovered chats in the sidebar and backup panel, and open one selected backup chat through the same normalized timeline used by export ZIP imports. This path is covered with synthetic tests and has passed local real-backup chat-rendering smoke without committing private artifacts.
- When macOS blocks automatic access to the default MobileSync backup folder, the desktop app shows a plain "Choose folder" fallback that accepts either the Backup folder or one specific device backup folder.
- Backup media preview can resolve `ChatStorage.sqlite` media paths through `Manifest.db` to hashed backup files and return bounded browser-readable previews. This is covered by synthetic tests and real local backup UI smoke.
- Backup HTML export has a bounded implementation for selected backup chats, including media resolved through `Manifest.db`; it is enabled in the desktop UI after real local backup export smoke.
- The desktop source screen separates the available WhatsApp export ZIP viewer from iPhone-backup proof work.
- The desktop search field filters WhatsApp export ZIP messages through shared frontend domain helpers, searches iPhone-backup chat names through a bounded backend query, and searches selected iPhone-backup chats through a bounded backend query for latest matching messages.
- The desktop timeline renders a bounded recent-message window with a tested "show earlier" path for already-loaded chats, while source importers avoid returning unbounded message vectors for huge backup or ZIP histories.
- The public synthetic demo renders a safe inline image preview and image-preview modal without using private media files.
- The README links to a committed synthetic MP4 generated through the Playwright plus `playwright-recast` workflow.
- The desktop visual suite covers keyboard focus visibility, accessible names for icon-only controls, contrast floors for core text, and removal of fake or unsupported action chrome.
- A private-safe proof CLI exists at `crates/whatsvault-proof`.
- The private-safe proof CLI has located WhatsApp `ChatStorage.sqlite` through a real local iPhone backup `Manifest.db`, confirmed the physical backup file exists, read nonzero aggregate database counts, read a bounded real chat-list sample, and imported a bounded real chat sample into the normalized model without printing paths, identifiers, names, message bodies, or filenames.
- A macOS `.app` bundle and DMG can be built locally with Tauri, and release checksum manifests can be generated from the bundle outputs.
- Visible packaged-window smoke from the generated macOS app bundle opens to a nonblank source screen locally.
- CI and draft-release workflows are configured for macOS Apple Silicon, macOS Intel, and Windows Tauri builds.
- Release checksum manifests can be generated from Tauri bundle outputs and are uploaded by the draft-release workflow.
- Real local backup media preview and bounded HTML export smoke have passed from the packaged macOS app without committing private artifacts.

Public-safe proof details live in [docs/proof-evidence.md](docs/proof-evidence.md). Do not add private proof dumps, paths, screenshots, media, or exported chats to the repository.

## Development

Install desktop app dependencies:

```sh
cd apps/desktop
npm install
```

Run all Rust tests from the repository root:

```sh
cargo test --workspace
```

Run desktop frontend tests:

```sh
cd apps/desktop
npm test
```

Run the public repository hygiene guard before publishing or opening a pull request:

```sh
cd apps/desktop
npm run hygiene:public
```

The guard scans Git-tracked files plus local non-ignored new files for private backups, transcripts, databases, media, local paths, personal roadmap-only material, and unexpected changes to the committed synthetic demo assets.

Run the release-readiness status guard:

```sh
cd apps/desktop
npm run release:readiness
```

This command passes when the current pre-alpha blockers are documented honestly. Before a stable public release, `npm run release:preflight` must pass; today it is expected to fail until macOS notarized signing and Windows code signing are configured.

Inspect local signing inputs without printing secret values:

```sh
cd apps/desktop
npm run release:signing
```

The release workflow can generate platform signing config from GitHub secrets at runtime; certificate material and passwords must stay in GitHub Secrets or local ignored files.

Run desktop visual layout checks:

```sh
cd apps/desktop
npm run visual:check
```

Run Rust formatting and lint checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

Record and render the synthetic README demo video:

```sh
cd apps/desktop
npm run demo:video
```

Run the desktop web preview:

```sh
cd apps/desktop
npm run dev
```

Run the Tauri desktop app:

```sh
cd apps/desktop
npm run tauri dev
```

Build the macOS app and DMG:

```sh
cd apps/desktop
npm run tauri build
```

Generate checksum manifests for the local Tauri bundle outputs:

```sh
cd apps/desktop
npm run release:checksums
```

The checksum command scans `target/release/bundle` plus target-specific Tauri roots such as `target/<target-triple>/release/bundle`. Set `WHATSVAULT_BUNDLE_DIR` only when checking one explicit bundle directory.

Current HTML export behavior:

- Exports the loaded WhatsApp export ZIP chat or selected iPhone-backup chat to one `.html` file.
- The WhatsApp export ZIP path imports the latest bounded message window for large transcripts instead of returning an unbounded in-memory message list.
- The iPhone-backup export path runs behind the Tauri command boundary with bounded recent-message export, synthetic tests, and real local backup smoke.
- Escapes message text, sender names, filenames, and titles.
- Embeds media as data URLs when the attachment has a known browser media type and fits within the local per-file and total export size limits.
- Lists media that cannot be embedded instead of failing the export.

Run the private-safe backup proof command against default backup roots:

```sh
cargo run -p whatsvault-proof
```

Run the proof command against a specific backup root:

```sh
cargo run -p whatsvault-proof -- "/path/to/MobileSync/Backup"
```

The proof command reports aggregate counts, sampled counts, sample limits, and booleans only. It does not print device IDs, backup paths, file IDs, contacts, message bodies, or media filenames.

Optionally validate a private local WhatsApp export ZIP without printing chat content:

```sh
WHATSVAULT_PRIVATE_EXPORT_ZIP="/path/to/private-export.zip" \
  cargo test -p whatsvault-core --test whatsapp_export_zip -- \
  --ignored imports_private_export_zip_without_printing_chat_content
```

## Development Notes

The exported ZIP viewer was the first fully proven desktop path. The first hard iPhone-backup gates now pass: WhatsVault can locate WhatsApp data from a real local iPhone backup through `Manifest.db`, read `ChatStorage.sqlite`, render a real backup chat, preview bounded media, and export a bounded chat HTML file from the packaged desktop app without committing private artifacts. The next hard gate is release hardening: signed/notarized macOS output, Windows signing, and clean-machine install/open proof.

Real private exports and backups must stay in ignored local paths. Committed tests use synthetic fixtures or ignored private samples.
