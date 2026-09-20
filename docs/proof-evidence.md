# Public Proof Evidence

This page records public-safe evidence for WhatsVault proof claims. It intentionally avoids private backup details. Do not add real local paths, backup IDs, file IDs, contact names, phone numbers, message bodies, media filenames, screenshots from private chats, raw SQLite rows, or exported HTML from real chats.

Public proof evidence does not include private backup content, private identifiers, local paths, screenshots from private chats, or generated exports from real conversations.

## Earlier Local Validation

Earlier local validation recorded the following results. They are historical observations, not a guarantee that every backup format works on the current revision.

| Area | Public-safe status | Evidence boundary |
| --- | --- | --- |
| Real local iPhone backup discovery | Passed locally | `Manifest.db` was found through the local backup discovery flow without recording the backup path or identifier. |
| WhatsApp database resolution | Passed locally | `ChatStorage.sqlite` was resolved from `Manifest.db` to its physical backup file without recording the file ID or hashed backup path. |
| Aggregate database reading | Passed locally | The proof CLI read nonzero aggregate counts for chats and messages without printing contacts or message bodies. |
| Desktop chat rendering | Passed locally | The packaged desktop app opened a real backup chat through the same normalized timeline used by synthetic tests. |
| Bounded media preview | Passed locally | Real backup media was resolved through `Manifest.db` and previewed only through bounded browser-readable payloads. |
| Bounded HTML export | Passed locally | One selected real backup chat exported to escaped, self-contained HTML with bounded media embedding. The generated private export was not committed. |
| Packaged app smoke | Passed locally | The generated macOS app bundle opened to a nonblank source screen outside the dev server. |

These proof notes are deliberately weaker than public fixtures because they cannot expose the user's real backup. Synthetic fixtures and automated tests cover the public, repeatable shape of the same behavior.

## September 19, 2026 Quality Review

The current review used synthetic data. No real local iPhone backup was available to repeat the earlier private validation.

- The Rust workspace passed 106 tests; the optional private-fixture test remained ignored.
- Frontend unit and script checks passed 90 tests. All 48 browser checks passed in Chromium and WebKit, including source transitions, stale requests, media changes, mixed-height timelines, narrow layouts, keyboard focus, and automated WCAG A/AA checks on four screens.
- The packaged macOS app imported a synthetic ZIP with 100,001 messages, displayed the latest 2,000 messages, opened an image preview, and exported that loaded window. The generated HTML contained exactly 2,000 messages and one embedded image.
- A separate synthetic iPhone backup with 301 chats and 100,300 messages opened in the packaged app. The largest chat loaded a 2,000-message window, and full-backup search found its oldest message outside that window.
- The README screenshot and video were regenerated from fictional data. Their hashes are recorded in the demo asset manifest.

Automated accessibility checks do not replace manual assistive-technology testing. Windows package builds do not establish successful installation or runtime behavior on a Windows machine.

## Repeatable Public Evidence

Public CI and local checks cover the source-neutral behavior without private data:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/desktop
npm test
npm run build
npm run visual:check
npm run hygiene:public
npm run release:readiness
```

The repeatable tests cover:

- default backup-root construction for macOS and Windows
- selected backup folder fallback behavior
- `Manifest.db` file mapping
- `ChatStorage.sqlite` summary, bounded proof CLI sampling, chat listing, selected-chat import, selected-chat search, and bounded chat-list search
- media path resolution from WhatsApp relative paths to iPhone backup files
- bounded media preview behavior
- bounded HTML export and HTML escaping
- privacy-safe Tauri command boundaries that return opaque handles instead of local paths
- source-screen, chat-list, media-preview, mobile-layout, accessibility, and fake-action visual checks

## Private Proof Rules

Private proof is allowed only when the evidence stays local and redacted:

1. Run `cargo run -p whatsvault-proof` against local backups.
2. Record only booleans, aggregate counts, sampled counts, sample limits, command names, dates, and whether the app reached the expected screen.
3. Keep generated screenshots, exported HTML, backup files, databases, and media in ignored local paths.
4. Before committing, run `cd apps/desktop && npm run hygiene:public`.

Acceptable proof notes:

- "Real local backup proof passed on macOS: `Manifest.db` found, `ChatStorage.sqlite` resolved, nonzero aggregate message count."
- "Packaged macOS app opened one real backup chat; no screenshots or exported chat files committed."

Unacceptable proof notes:

- full local backup paths
- backup folder names or device IDs
- `fileID` values
- contact names, phone numbers, or message text
- private media filenames
- raw proof command dumps from real backups

## Release Interpretation

This page does not make WhatsVault stable-release ready. It only documents the private-proof boundary behind the current pre-alpha claims. Stable release still requires signed and notarized macOS output, Windows signing, clean-machine install/open proof, and current release notes with known limitations.
