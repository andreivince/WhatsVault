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

Backup discovery/status is available in the desktop app, and real backup chat rendering has passed local smoke. If automatic scanning is blocked by macOS filesystem access, use **Choose folder** in the iPhone backups panel and select either the MobileSync `Backup` folder or one specific device backup folder.

Check:

1. The backup is local on this computer.
2. The backup folder contains `Manifest.db`.
3. The backup is not encrypted.
4. WhatsApp was installed on the backed-up device.

Useful safe evidence:

- operating system
- Finder, Apple Devices, or iTunes backup source
- encrypted, unencrypted, or unknown backup state
- `Manifest.db` present: yes/no
- WhatsApp installed on the backed-up device: yes/no

## Desktop Window Opens Blank

The generated macOS app bundle currently opens to a nonblank source screen in local smoke. If a packaged window opens blank on another machine, report it as a packaging/runtime issue, not as a chat parsing issue.

Do not report private chat data for this issue. Useful safe evidence:

- macOS version
- app version
- whether the browser preview renders
- whether the packaged `.app` creates a window
- a screenshot of the blank window only

The browser visual suite is useful for UI regression checks, but it does not prove packaged Tauri rendering.

## Stable Release Preflight Fails On Code Signing

This is expected until release signing is configured.

Run:

```sh
cd apps/desktop
npm run release:signing
```

The command lists missing macOS and Windows signing inputs by variable or config key name only. Do not paste certificate contents, passwords, private keys, real `.p12` files, or Apple notarization keys into public issues.

## HTML Export Fails

Check:

1. A chat source is loaded before export.
2. The output path is writable.
3. The target file is not open in another app.
4. Available disk space is sufficient for embedded media.

HTML export is conservative: attachments that cannot be embedded should be listed instead of failing the whole export.
