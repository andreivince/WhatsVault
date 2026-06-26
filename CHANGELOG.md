# Changelog

All notable public changes to WhatsVault should be recorded here.

WhatsVault is pre-alpha. Until the first stable release, entries should stay honest about proof boundaries and must not include private backup paths, contact identifiers, message text, media filenames, screenshots from private chats, or generated exports from real conversations.

## Unreleased

- Public GitHub repository foundation with README, roadmap, architecture docs, contribution guide, security policy, issue templates, pull request template, and release documentation.
- Tauri v2 desktop app with React and TypeScript plus a Rust core crate for source-neutral import, media preview, search, and HTML export behavior.
- WhatsApp export ZIP viewer using bounded latest-message import, on-demand bounded media preview, and self-contained HTML export.
- iPhone backup discovery, `Manifest.db` mapping, `ChatStorage.sqlite` summary/list/import/search, media resolution, and bounded HTML export covered by synthetic tests.
- Public-safe local proof notes for real iPhone backup discovery, real backup chat rendering, bounded media preview, bounded HTML export, and packaged macOS app smoke without committing private artifacts.
- Public hygiene guard for tracked files and non-ignored new files to catch private backups, transcripts, databases, media, local paths, personal roadmap-only material, and unexpected demo asset changes.
- CI and draft-release workflows for macOS Apple Silicon, macOS Intel, and Windows Tauri builds, including bundle checksum generation.
- Stable-release preflight that remains blocked until macOS signing/notarization and Windows code signing are configured and verified.
