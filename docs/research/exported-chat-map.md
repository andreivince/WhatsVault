# WhatsApp Export ZIP Research

Date: June 23, 2026

This note records public-safe structure findings from a local WhatsApp exported chat ZIP. The archive itself is private and must not be committed.

## Observed Archive Shape

- Total ZIP size: about 912 MiB.
- Total entries: 2,165 files.
- Directories: 0.
- Transcript files: 1.
- Transcript filename: `_chat.txt`.
- Transcript size: about 1.1 MiB.
- Media entries: 2,164.

Observed media extensions:

| Extension | Count |
| --- | ---: |
| `.opus` | 680 |
| `.jpg` | 676 |
| `.webp` | 569 |
| `.mp4` | 237 |
| `.mp3` | 2 |

Observed media filename categories:

| Category | Extension | Count |
| --- | --- | ---: |
| `AUDIO` | `.opus` | 680 |
| `PHOTO` | `.jpg` | 676 |
| `STICKER` | `.webp` | 569 |
| `VIDEO` | `.mp4` | 219 |
| `GIF` | `.mp4` | 18 |
| `AUDIO` | `.mp3` | 2 |

Observed transcript shape:

- Transcript lines: 17,735.
- Non-empty transcript lines: 17,716.
- Bracketed timestamp plus sender pattern: 15,180 lines.
- Dash timestamp pattern: 0 lines in this sample.
- Lines with media reference markers: 2,164.
- Some lines include Unicode directional marks before the timestamp; the parser normalizes those marks.

## Parser Implications

The export ZIP importer should:

1. Treat `_chat.txt` as the transcript source.
2. Avoid extracting media eagerly.
3. Classify media entries by filename category and extension.
4. Resolve media references from transcript lines to archive entries.
5. Report missing media as structured import issues instead of failing the whole import.
6. Read bounded attachment payloads for previews on demand.
7. Avoid logging or committing message bodies.

## Verified Parser Behavior

Implemented in `crates/whatsvault-core/src/sources/whatsapp_export_zip.rs`.

Covered by tests:

- bracketed iOS-style timestamp parsing
- dash-style timestamp parsing
- multiline message continuations
- media filename classification
- media reference resolution
- bounded attachment byte lookup by normalized archive path
- oversized attachment preview skip behavior
- missing transcript errors
- continuation-without-message import issues
- private local ZIP validation through ignored env-var test

## Desktop Viewer Behavior

Implemented in `apps/desktop`.

The desktop app can:

1. Open a WhatsApp export ZIP with a native file picker.
2. Import the ZIP through `whatsvault-core`.
3. Render a WhatsApp-like chat list and timeline.
4. Search message sender/body text.
5. Preview bounded images, stickers, audio, video, and documents from the archive without extracting the ZIP.
6. Open image media in an in-app preview modal.
7. Export the chat to one self-contained HTML file with bounded embedded media.
8. Build as a local macOS `.app` and DMG through Tauri.

HTML export is intentionally conservative:

- Message text and metadata are escaped.
- Known browser media types can be embedded as data URLs.
- Media over the per-file or total export size limit is listed but not embedded.
- The export file is written only to a user-selected local path.

## Product Boundary

This source is the first usable viewer path, but it does not satisfy the roadmap gate for reading `ChatStorage.sqlite` from an iPhone backup through `Manifest.db`.
