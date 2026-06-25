# Supported Sources

WhatsVault is pre-alpha. This matrix is intentionally conservative and should not overpromise behavior that is not verified.

| Source | macOS | Windows | Chat text | Media preview | HTML export | Status |
| --- | --- | --- | --- | --- | --- | --- |
| WhatsApp export ZIP | Supported in desktop app | Expected through Tauri, pending Windows CI proof | Supported | Images, stickers, audio, video, and documents when bounded and browser-readable | Supported | First usable source path |
| Unencrypted iPhone backup | Backup discovery/status, chat-list/import UI path, media-preview resolver, and selected-chat HTML export implemented with synthetic coverage; real backup proof pending | Default Apple Devices/iTunes root construction tested; live Windows proof pending | UI path implemented with synthetic coverage; real backup proof pending | Preview resolver implemented with synthetic coverage; real backup proof pending | Implemented with synthetic coverage; real backup proof pending | Primary roadmap target |
| Encrypted iPhone backup | Not supported | Not supported | Not supported | Not supported | Not supported | Future research |
| Android WhatsApp backup | Not supported | Not supported | Not supported | Not supported | Not supported | Out of scope for the current roadmap |

## Current Behavior

- The desktop app can import a WhatsApp export ZIP without extracting the archive eagerly.
- ZIP media is read on demand and bounded before preview or HTML embedding.
- The desktop source screen labels the WhatsApp export ZIP path as available now and the iPhone backup path as real-backup proof pending.
- The desktop app can scan default iPhone backup folders and show safe display metadata, encryption state when known, and WhatsApp `Manifest.db` file-detection status.
- Default backup-root construction is tested for macOS, Windows Microsoft Store Apple Devices or iTunes, and Windows legacy iTunes locations.
- A selected ready backup can load chat summaries into the sidebar and open one selected chat through the shared timeline UI path.
- Backup media preview can resolve a `ChatStorage.sqlite` media relative path through `Manifest.db` to a hashed backup file and return bounded browser-readable media when present.
- Backup HTML export can re-import a selected chat by id, embed bounded media resolved through `Manifest.db`, and write through the same escaped core exporter as ZIP exports.
- The iPhone backup path has synthetic tests for backup discovery, metadata plists, `Manifest.db` mapping, fileID resolution, `ChatStorage.sqlite` aggregate counts, chat listing, selected-chat import, sender direction, timestamps, media records, and the Tauri command boundary that resolves `ChatStorage.sqlite` from a backup folder.
- A real local iPhone backup is still required before backup browsing can be marked complete.

## Privacy Boundary

Do not attach real exports, backups, databases, media, or screenshots from private chats to public issues. Use redacted counts and synthetic fixtures.
