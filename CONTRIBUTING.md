# Contributing

WhatsVault is a local-first desktop app for browsing WhatsApp chats and media from local iPhone backups. Contributions should keep the app private by design, cross-platform, and maintainable.

## Privacy Rules

Never commit or paste:

- real WhatsApp exports
- real iPhone backups
- SQLite databases
- plist files from real devices
- private media
- message bodies
- contact names, phone numbers, or identifiers
- full local paths
- raw proof dumps

Use synthetic fixtures and redacted counts. If a bug requires private data to reproduce, reduce it to the smallest synthetic fixture that exercises the same parser behavior.

## Development Setup

Install desktop dependencies:

```sh
cd apps/desktop
npm install
```

Run the main checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/desktop
npm test
npm run hygiene:public
npm run release:readiness
npm run build
npm run visual:check
```

`npm run hygiene:public` scans Git-tracked files plus local non-ignored new files. It is meant to catch private backups, transcripts, databases, media, local paths, and personal roadmap-only material before a pull request.

`npm run release:readiness` confirms the public docs still describe known release blockers honestly. `npm run release:preflight` is the stricter stable-release gate; it is expected to fail while signing and notarization are incomplete.

Build the desktop app locally:

```sh
cd apps/desktop
npm run tauri build
```

## Architecture Expectations

- Keep parser behavior in `crates/whatsvault-core`.
- Keep Tauri command wiring in `apps/desktop/src-tauri`.
- Keep frontend source handling behind `apps/desktop/src/services/desktop.ts` and `apps/desktop/src/domain/source.ts`.
- Do not create parallel ZIP, backup, or export logic in React components.
- Add tests for parsing, normalization, export, media resolution, and error handling.

## Demo Assets

README demos must use synthetic English data only.

```sh
cd apps/desktop
npm run demo:video
```

Generated demo output is ignored under `apps/desktop/target/readme-demo/`.

## Pull Requests

Before opening a pull request:

1. Run the verification commands above.
2. Confirm no private files or identifiers are included.
3. Update docs when the user workflow, supported-source status, or architecture boundary changes.
4. Keep changes focused on one coherent improvement.

Contributors must follow [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Privacy mistakes should be treated as project-safety issues and fixed before broader review.
