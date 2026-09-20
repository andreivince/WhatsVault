# WhatsVault

**Browse your WhatsApp history on your own computer.**

WhatsVault is a local desktop viewer for WhatsApp chats and media from unencrypted iPhone backups and exported chat ZIPs. Search conversations, preview attachments, and save a chat as a self-contained HTML file. No account, cloud service, or message upload is required.

**Pre-alpha:** macOS and Windows builds are available for early testing. macOS builds are not Developer ID signed or notarized; Windows builds are not code signed. See [known limitations](#known-limitations) before getting started.

## Demo

https://github.com/user-attachments/assets/2e0507a4-698d-4439-ae8d-60bde8cbd67d

[View screenshot](docs/assets/whatsvault-synthetic-demo.png) · [Download video](docs/assets/whatsvault-readme-demo.mp4) · Fictional conversations and generated media.

## Get started

1. Download the build for your computer from [Releases](https://github.com/andreivince/WhatsVault/releases). Published builds may be behind the latest changes on `main`; check the release date and notes.
2. Open WhatsVault and select a detected iPhone backup. If it is missing, use **Choose folder** to select a local backup folder. You can also open a WhatsApp exported chat ZIP.
3. Select a chat to browse messages and attachments. Use search and the date filter to find messages, then **Export chat to HTML** to save a local copy.

Downloads include SHA-256 checksums. See [release verification](docs/ci-release.md#release-workflow) for checksum and artifact attestation details, or [troubleshooting](docs/troubleshooting.md) if a source will not open.

## What works

- Browse chat lists from local iPhone backups and open individual WhatsApp export ZIPs.
- Search backup chat names and message history, or filter messages loaded from a ZIP.
- Preview supported images, stickers, audio, video, and documents without extracting the entire backup or archive.
- Export a recent chat window to HTML, with supported media embedded when it fits the size limits.
- Keep chats, contacts, backups, and media on your computer.

| Source | Current support |
| --- | --- |
| Unencrypted iPhone backup | Chat browsing, search, media previews, and HTML export; verified locally on macOS |
| WhatsApp export ZIP | Chat text, supported attachments, and HTML export |
| Encrypted iPhone backup | Not supported |
| Android database backup | Not supported |

Both platforms build in CI. Live Windows validation is still pending. See [supported sources](docs/supported-sources.md) and [validation notes](docs/proof-evidence.md) for detailed coverage.

## Known limitations

- **Large histories:** the viewer and HTML exporter load a bounded recent message window. “Show earlier” reveals more of that loaded window; it does not retrieve the entire history. Backup message search can find older matches, but ZIP search covers loaded messages only.
- **Media:** missing, oversized, or unsupported attachments appear as placeholders. HTML exports list attachments that could not be embedded.
- **Installation:** platform signing, macOS notarization, and clean-machine installation checks are still needed before a stable release.

See the [roadmap](ROADMAP.md) for planned work.

## Development

The app uses Tauri, React, TypeScript, and a shared Rust core. Install Node.js 24, Rust stable, and the [Tauri platform prerequisites](https://v2.tauri.app/start/prerequisites/), then run:

```sh
git clone https://github.com/andreivince/WhatsVault.git
cd WhatsVault/apps/desktop
npm ci
npm run tauri dev
```

For a browser preview with fictional data, run `npm run dev` and open [the demo](http://127.0.0.1:1420/?demo=backup-chat). Native file dialogs and local backup access require the desktop app.

See [CONTRIBUTING.md](CONTRIBUTING.md) for tests, browser checks, and contribution guidelines, and [architecture](docs/architecture.md) for module boundaries.

## Help and contributions

Bug reports and focused pull requests are welcome. Start with [troubleshooting](docs/troubleshooting.md), then [open an issue](https://github.com/andreivince/WhatsVault/issues/new/choose) with reproduction steps and your app and operating-system versions. Use fictional examples and remove private information from screenshots or logs.

Please read the [contribution guide](CONTRIBUTING.md) and [code of conduct](CODE_OF_CONDUCT.md). Report security issues through [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE). WhatsVault is an independent project and is not affiliated with WhatsApp or Meta.
