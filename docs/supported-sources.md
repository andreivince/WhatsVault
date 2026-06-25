# Supported Sources

WhatsVault is pre-alpha. This matrix is intentionally conservative and should not overpromise behavior that is not verified.

| Source | macOS | Windows | Chat text | Media preview | HTML export | Status |
| --- | --- | --- | --- | --- | --- | --- |
| WhatsApp export ZIP | Supported in desktop app with bounded latest-message import | Expected through Tauri, pending Windows CI proof | Supported for the loaded latest-message window | Images, stickers, audio, video, and documents when bounded and browser-readable | Supported for the loaded latest-message window | First usable source path |
| Unencrypted iPhone backup | Backup discovery/status, chat-list/import UI path, folder-access fallback, bounded media preview, and bounded selected-chat HTML export covered with synthetic tests and real local smoke | Default Apple Devices/iTunes root construction tested; live Windows proof pending | Real backup chat rendering verified locally without committing private artifacts | Real backup media preview smoke passed locally with bounded browser-readable previews | Real backup bounded HTML export smoke passed locally | Primary roadmap target, macOS pre-alpha |
| Encrypted iPhone backup | Not supported | Not supported | Not supported | Not supported | Not supported | Future research |
| Android WhatsApp backup | Not supported | Not supported | Not supported | Not supported | Not supported | Out of scope for the current roadmap |

## Current Behavior

- The desktop app can import a WhatsApp export ZIP without extracting the archive eagerly, and large transcripts are bounded to a latest-message window before reaching the UI.
- ZIP media is read on demand and bounded before preview or HTML embedding.
- The desktop source screen labels the WhatsApp export ZIP path as available now and the iPhone backup path as preview-ready.
- The desktop app can scan default iPhone backup folders and show safe display metadata, encryption state when known, and WhatsApp `Manifest.db` file-detection status.
- If macOS blocks automatic access to the default backup folder, the desktop app provides a plain "Choose folder" fallback that accepts either the MobileSync Backup folder or one device backup folder.
- Detected backups stay inspectable even when they are encrypted or missing WhatsApp data, so users see plain next steps instead of a disabled dead end.
- Default backup-root construction is tested for macOS, Windows Microsoft Store Apple Devices or iTunes, and Windows legacy iTunes locations.
- A selected ready backup can load chat summaries into the sidebar, search backup chat names through a bounded backend query, open one selected chat through the shared timeline UI path, and search that selected chat through a bounded latest-match backend query.
- Backup media preview can resolve a `ChatStorage.sqlite` media relative path through `Manifest.db` to a hashed backup file and return bounded browser-readable media when present.
- Backup HTML export can re-import a selected chat by id, export a bounded recent-message window, embed bounded media resolved through `Manifest.db`, and write through the same escaped core exporter as ZIP exports. It is available in the desktop UI for ready backups.
- The iPhone backup path has synthetic tests for backup discovery, metadata plists, `Manifest.db` mapping, fileID resolution, `ChatStorage.sqlite` aggregate counts, chat listing, selected-chat import, sender direction, timestamps, media records, and the Tauri command boundary that resolves `ChatStorage.sqlite` from a backup folder.
- A real local iPhone backup has now proven `Manifest.db` discovery, physical `ChatStorage.sqlite` resolution, aggregate count reading, safe chat-list extraction, normalized chat import, desktop chat rendering, bounded media preview, and bounded HTML export without printing or committing private content.

See [proof-evidence.md](proof-evidence.md) for the public-safe evidence boundary behind real-backup proof claims.

## Privacy Boundary

Do not attach real exports, backups, databases, media, or screenshots from private chats to public issues. Use redacted counts and synthetic fixtures.
