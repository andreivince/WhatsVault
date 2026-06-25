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

## Current Gate

No real MobileSync backup is currently available in the local development environment, so the `Manifest.db` proof cannot be verified against a real iPhone backup yet.

Do not mark the iPhone-backup phase complete until a real local backup proves:

1. available backup folders can be listed;
2. `Manifest.db` can be opened;
3. WhatsApp files can be located from `domain` and `relativePath`;
4. the `ChatStorage.sqlite` `fileID` resolves to an existing physical backup file;
5. `ChatStorage.sqlite` can be read;
6. message, chat, and media-item counts can be produced without manually copying SQLite files.
