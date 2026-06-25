# WhatsVault Roadmap

Start date: June 23, 2026

Core rule: this is open-source material, but it must feel product-quality. The goal is not to build another WhatsApp parser. The goal is to make the easiest local Mac viewer for iPhone WhatsApp backups.

## North Star

A user downloads WhatsVault, opens it, picks an iPhone backup, and sees their WhatsApp chats with media. No terminal. No manual backup ID hunting. No copying SQLite files by hand.

## Success Criteria

Ship only if WhatsVault can do these things:

1. Auto-detect local iPhone Finder or iTunes backups on macOS.
2. Read the backup `Manifest.db` and locate WhatsApp files.
3. Load `ChatStorage.sqlite` from the backup.
4. Render at least one real chat thread.
5. Resolve sender names enough that chats are readable.
6. Display image media from the backup.
7. Search messages.
8. Export one chat to self-contained HTML.
9. Ship a downloadable macOS release.
10. README proves privacy: all local, no upload, no account, no cloud.

## Public Product Wedges

WhatsVault should win by making the iPhone-backup path obvious for nontechnical users while staying useful to technical users who want local, inspectable tooling.

The public roadmap should prioritize:

1. Automatic backup discovery instead of asking users to find hidden backup folders.
2. Clear backup labels from local metadata instead of opaque backup IDs.
3. Local-only processing with no account, no cloud upload, and no hosted parser.
4. A downloadable desktop app instead of a terminal-first workflow.
5. Readable support states: backup found, WhatsApp found, media found, encrypted or unsupported state detected.
6. Graceful failure reports when a backup, schema, or media file cannot be read.
7. Virtualized chat rendering so large conversations stay usable.
8. Media-first browsing because memories are often photos, videos, audio, documents, and stickers, not only text.
9. Self-contained export for one selected chat before broad bulk export.
10. Documentation that explains supported backup types and known limits without overpromising.

The core positioning is:

> Open WhatsVault, pick a local iPhone backup, browse WhatsApp chats and media privately on your computer.

## Public Release Quality Gates

Before a release is presented as public-ready, it should satisfy these gates:

1. The README opens with a synthetic demo GIF or screenshot that shows chat text, media preview, search, and export in the first viewport.
2. The README links to a full MP4 demo generated from synthetic English data.
3. The supported-source matrix is current and conservative.
4. The app clearly separates supported export-ZIP viewing from in-progress iPhone-backup browsing.
5. CI passes for Rust formatting, Rust linting, Rust tests, frontend tests, frontend build, and desktop bundle smoke checks.
6. Release artifacts are attached for macOS Apple Silicon, macOS Intel, and Windows when available.
7. Each release includes checksums and known limitations.
8. Unsigned or unnotarized builds are labeled clearly until signing is configured.
9. No real chats, backups, databases, media, local paths, backup IDs, file IDs, names, or phone numbers appear in committed files, screenshots, videos, examples, logs, or test fixtures.
10. The issue templates continue to block users from pasting private data by default.
11. Large chats remain usable through bounded rendering or virtualization.
12. Media preview failures degrade into readable placeholders instead of failed imports.
13. HTML export escapes message text, sender names, titles, and filenames.
14. Accessibility basics are covered: keyboard focus, readable labels, color contrast, and no essential icon-only controls without labels.
15. The demo-video command is reproducible from a clean checkout after installing dependencies.
16. The app can be opened from a fresh install path without requiring a terminal workflow. Browser visual checks alone are not enough; packaged Tauri rendering must show a nonblank source screen unless an upstream platform blocker is explicitly documented.
17. Troubleshooting docs cover missing backups, missing media, parser failures, macOS permissions, Windows install status, and privacy-safe bug reports.
18. The project keeps one parser/model source of truth in the Rust core, with no duplicate parser logic in React.

## Kill Gates

Kill or pause the project if any of these happen:

1. End of Day 3: cannot locate WhatsApp files from a real iPhone backup through `Manifest.db`.
2. End of Week 1: cannot render a real chat from a local backup.
3. End of Week 2: still no downloadable app or no screenshot-worthy UI.
4. Any point: the project becomes a parser-first CLI instead of a local Mac viewer.

## Phase 0: Name, Repo, Positioning

Goal: make the project understandable before code.

Tasks:

1. Create GitHub repo: WhatsVault.
2. Add one-line positioning: Open-source local viewer for iPhone WhatsApp backups.
3. Add privacy promise: your backup stays on your Mac.
4. Add initial README sections: Problem, Demo target, Privacy, Roadmap, Status.
5. Add license, probably MIT unless a dependency forces otherwise.
6. Add `docs/architecture.md` with the iPhone backup flow.

First README headline:

> WhatsVault is a local-first macOS app for browsing WhatsApp chats and media from iPhone backups. It runs entirely on your Mac and never uploads your messages.

## Phase 1: Backup Archaeology

Goal: prove the hard technical path before building UI.

Tasks:

1. Find macOS backup directory:
   `~/Library/Application Support/MobileSync/Backup`
2. List available device backup folders.
3. Open `Manifest.db` for one backup.
4. Understand `fileID`, `domain`, `relativePath` mapping.
5. Locate WhatsApp app domain entries.
6. Find `ChatStorage.sqlite`.
7. Find `ContactsV2.sqlite` if present.
8. Find media attachment paths.
9. Write a small internal proof script that prints:
   - backup name or ID
   - WhatsApp database path
   - number of messages
   - number of chats
   - sample latest messages

Output needed before moving on:

A terminal proof that WhatsVault can find and read real WhatsApp data from an iPhone backup without manual SQLite copying.

## Phase 2: Data Model

Goal: convert raw WhatsApp SQLite into stable app data.

Tasks:

1. Define internal models:
   - `Backup`
   - `Chat`
   - `Message`
   - `Contact`
   - `Attachment`
2. Map `ChatStorage.sqlite` tables.
3. Extract chat list sorted by latest message.
4. Extract messages for one chat.
5. Resolve sender display names.
6. Resolve timestamps correctly.
7. Detect message types: text, image, video, audio, document, sticker, call event, system event.
8. Build media path resolver from `Manifest.db`.
9. Add fixture-free tests around parser functions using a small synthetic SQLite database.

Output needed:

A clean internal API that the UI can call without knowing WhatsApp SQLite table weirdness.

## Phase 3: Minimal Viewer

Goal: get the real-chat-visible moment.

Tasks:

1. Choose app stack.
   Recommended default: Tauri + React + TypeScript if speed matters.
   Alternative: SwiftUI if native polish matters more than iteration speed.
2. Build first screen: detected backups list.
3. Build chat sidebar.
4. Build message timeline.
5. Render text bubbles.
6. Render timestamps.
7. Render sender names in group chats.
8. Add empty states and error states.
9. Add one demo screenshot to README.

Output needed by end of Week 1:

Open WhatsVault, select backup, see a real WhatsApp chat.

## Phase 4: Media Reconstruction

Goal: make the viewer emotionally useful, not just technically correct.

Tasks:

1. Resolve image attachments.
2. Render image thumbnails in chat.
3. Open image preview.
4. Resolve video attachments.
5. Resolve audio attachments.
6. Resolve PDFs and documents.
7. Handle missing media gracefully.
8. Add per-chat media count.
9. Add README screenshots with media visible.

Output needed:

A chat with real media visible inside the app.

## Phase 5: Search and Export

Goal: make it useful enough to install and keep.

Tasks:

1. Add full-text search across messages.
2. Add search inside selected chat.
3. Add date jump or simple date filtering.
4. Export selected chat to self-contained HTML.
5. Include text and media in export when available.
6. Add JSON export only if cheap.
7. Add export progress and success state.
8. Add README export example.

Output needed:

A user can find a memory and export it without touching terminal.

## Phase 6: Open-Source Polish

Goal: make GitHub visitors trust it in 10 seconds.

Tasks:

1. README with screenshots above the fold.
2. Add a 20-second GIF or short screen recording generated from synthetic English demo data.
   - Use a code-first Playwright walkthrough as the source of truth.
   - Render the polished README video with `playwright-recast`.
   - Keep generated private/local video output ignored until a small public asset is intentionally selected.
3. Add clear install instructions.
4. Add security and privacy section.
5. Add supported backup matrix:
   - unencrypted iPhone backup
   - encrypted iPhone backup
   - media support status
   - WhatsApp version tested
6. Add troubleshooting page.
7. Add `CONTRIBUTING.md`.
8. Add issue templates:
   - backup not detected
   - media missing
   - parse failure
   - feature request
9. Add architecture diagram.
10. Add GitHub topics:
    `whatsapp`, `ios-backup`, `macos`, `sqlite`, `local-first`, `privacy`, `archive`, `tauri`.

Workflow note:

- CI should keep Rust core checks, frontend checks, and macOS/Windows Tauri bundle smoke tests visible for contributors.
- Release automation should create draft pre-release artifacts first, because code signing and notarization are not configured yet.

Output needed:

A repo that looks like a serious open-source utility, not a class project.

## Phase 7: Release

Goal: make it downloadable.

Tasks:

1. Build macOS release artifact.
2. Create GitHub Release `v0.1.0`.
3. Attach `.dmg` or zipped app.
4. Add checksums.
5. Add release notes with known limitations.
6. Confirm app launches on a clean path.
7. Confirm README install path works.
8. Pin release in repo.

Output needed:

Someone can download WhatsVault from GitHub and open it.

## Phase 8: Community Launch

Goal: share the project only after it is useful, credible, and easy for new users to evaluate.

Tasks:

1. Post to Hacker News Show HN only after screenshots and release exist.
2. Post to relevant Reddit communities carefully:
   - `r/selfhosted` if local-first angle fits
   - `r/DataHoarder` for archive angle
   - `r/macapps` if release is polished
   - `r/privacy` if privacy story is strong
3. Submit to open-source directories and awesome lists.
4. Write a technical blog post:
   How iPhone backups store WhatsApp data and how WhatsVault reconstructs chats locally.
5. Post LinkedIn with engineering angle, not startup pitch.
6. Ask early users to open issues so feedback becomes searchable and useful to future contributors.
7. Track project health signals such as issues, downloads, release regressions, and contributor activity.

Output needed:

Public launch materials that help users understand, install, trust, and contribute to the project.

## First 48 Hours

Do only these first:

1. Create repo and README skeleton.
2. Choose stack.
3. Inspect local iPhone backup folder.
4. Open `Manifest.db`.
5. Locate WhatsApp domain files.
6. Extract message count from `ChatStorage.sqlite`.
7. Commit a proof note to `docs/research/ios-backup-map.md`.

Do not build UI until step 6 works.

## Final Judgment

Build WhatsVault if the first week proves real backup-to-chat rendering. Kill it fast if the hard path does not work. The win condition is not clever parsing. The win condition is becoming the obvious open-source Mac app for viewing iPhone WhatsApp backups locally.
