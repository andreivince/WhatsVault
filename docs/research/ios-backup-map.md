# iPhone Backup Research

Date: June 23, 2026

This note records the public-safe iPhone backup path and current implementation assumptions. It intentionally does not include any real device identifiers, backup paths, contact names, message bodies, or media filenames.

## Backup Location

Apple documents the macOS local backup location as:

```text
~/Library/Application Support/MobileSync/Backup/
```

On macOS Catalina 10.15 or later, Finder can create local iPhone backups. On Windows, Apple documents backups through the Apple Devices app or iTunes.

Current centralized backup roots:

- macOS: `~/Library/Application Support/MobileSync/Backup/`
- Windows, Apple Devices or Microsoft Store iTunes family: `%USERPROFILE%\Apple\MobileSync\Backup`
- Windows, legacy iTunes family: `%APPDATA%\Apple Computer\MobileSync\Backup`

## Expected Backup Files

A local iPhone backup folder commonly includes:

- `Info.plist`
- `Manifest.db`
- `Manifest.plist`
- `Status.plist`
- hashed data files split into two-character subdirectories

`Manifest.db` is expected to be a SQLite database with a `Files` table mapping backup `fileID` values to logical `domain` and `relativePath` values.

Modern backup files are stored under a two-character shard directory based on the first two characters of `fileID`:

```text
<backup-root>/<first-two-fileID-characters>/<fileID>
```

## WhatsApp Targets

The first iPhone-backup proof should locate:

- WhatsApp app domain entries
- `ChatStorage.sqlite`
- `ContactsV2.sqlite` when present
- media attachment paths

Current centralized WhatsApp domain constant:

```text
AppDomainGroup-group.net.whatsapp.WhatsApp.shared
```

## Verified Synthetic Behavior

Implemented in `crates/whatsvault-core/src/sources/iphone_backup.rs`.

Covered by tests:

- missing backup root returns no candidates
- candidate folders require `Manifest.db`
- optional `Info.plist`, `Status.plist`, and `Manifest.plist` paths are surfaced when present
- safe backup display metadata can be read from plist files without exposing backup folder names in the UI
- synthetic `Manifest.db` rows can be read
- WhatsApp `ChatStorage.sqlite`, `ContactsV2.sqlite`, and media paths can be located from synthetic manifest rows
- manifest `fileID` values can be resolved to modern two-character-sharded backup file paths
- macOS and Windows backup-root suffixes are centralized and tested
- synthetic `ChatStorage.sqlite` files can be summarized for message, chat, and media-item counts
- synthetic `ChatStorage.sqlite` files can list chats sorted by latest message and import one selected chat as normalized messages and attachments
- `cargo run -p whatsvault-proof` provides a private-safe report for real backup checks, including aggregate WhatsApp database counts when readable

## Real Backup Proof

On June 25, 2026, the private-safe proof command passed against a real local iPhone backup without manual SQLite copying.

Sanitized result:

- local backup root checked
- one backup candidate found
- `Info.plist` and `Status.plist` present
- WhatsApp `ChatStorage.sqlite` found through `Manifest.db`
- resolved physical `ChatStorage.sqlite` backup file exists
- WhatsApp `ContactsV2.sqlite` manifest entry present
- WhatsApp media manifest entries are present
- aggregate message, chat, and media-item counts are nonzero
- real chat list can be read
- a real chat can be imported into the normalized app model
- chat import proof exposes aggregate message/media counts only

The proof output intentionally does not record device identifiers, backup paths, file IDs, contact names, chat titles, message bodies, media filenames, or raw SQLite rows.

## Current Gate

The first real MobileSync backup proof gate is complete: WhatsVault can locate WhatsApp files through `Manifest.db`, resolve `ChatStorage.sqlite` to the physical backup file, read aggregate counts, read the chat list, import a real chat into the normalized app model, render that chat in the packaged desktop UI, preview bounded media from the backup, and export a bounded self-contained HTML file without manual SQLite copying.

Do not commit the private proof artifacts. Public documentation should mention only sanitized aggregate evidence and the remaining release-hardening work: signed/notarized macOS output, Windows signing, and clean-machine install/open proof.
