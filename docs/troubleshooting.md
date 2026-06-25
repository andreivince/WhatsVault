# Troubleshooting

This page keeps user-facing failure states consistent. Do not add private paths, message bodies, contact identifiers, backup IDs, file IDs, or raw database rows to reports.

## WhatsApp Export ZIP Does Not Open

Check:

1. The selected file is a `.zip` exported from WhatsApp.
2. The archive contains one transcript such as `_chat.txt`.
3. The file is stored locally and is readable by the current user.

Useful safe evidence:

- app version
- operating system
- ZIP size range
- transcript present: yes/no
- media extension counts
- redacted error text

## Media Is Missing

Media may show as a placeholder when:

- the archive does not contain the referenced file
- the media file is over the preview or export size limit
- the media type is not browser-readable
- the attachment reference in the transcript does not match a ZIP entry

Useful safe evidence:

- affected extension categories such as `.jpg`, `.webp`, `.opus`, `.mp4`, or `.pdf`
- approximate counts
- whether text messages still render
- whether export lists the media as skipped

## iPhone Backup Is Not Detected

Backup discovery/status is available in the desktop app, but backup-to-chat browsing is still in proof work. Current synthetic tests cover path discovery, metadata plists, and `Manifest.db` mapping; real backup-to-chat browsing is not complete.

Useful safe evidence:

- operating system
- Finder, Apple Devices, or iTunes backup source
- encrypted, unencrypted, or unknown backup state
- `Manifest.db` present: yes/no
- WhatsApp installed on the backed-up device: yes/no

## Desktop Window Opens Blank

On macOS 26, Tauri 2.11.x with Tao 0.35.3 can open a real application window while the WebView surface stays blank. This is tracked upstream at [tauri-apps/tauri#15517](https://github.com/tauri-apps/tauri/issues/15517).

Do not report private chat data for this issue. Useful safe evidence:

- macOS version
- Tauri, Wry, and Tao versions
- whether the browser preview renders
- whether the packaged `.app` creates a window
- a screenshot of the blank window only

The browser visual suite is useful for UI regression checks, but it does not prove packaged Tauri rendering.

## HTML Export Fails

Check:

1. A chat source is loaded before export.
2. The output path is writable.
3. The target file is not open in another app.
4. Available disk space is sufficient for embedded media.

HTML export is conservative: attachments that cannot be embedded should be listed instead of failing the whole export.
