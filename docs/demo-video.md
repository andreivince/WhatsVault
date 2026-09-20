# Demo Video Workflow

The README demo video should be generated from code, not recorded manually from private chats.

Current approach:

1. Use the synthetic English demo route at `/?demo=1`.
2. Record a deterministic Playwright walkthrough with trace and video enabled.
3. Render the trace into a polished MP4 with `playwright-recast`.
4. Publish only synthetic demo output in README assets.

The recording viewport and video canvas share a 1920 × 1080 size, matching the renderer's explicit 1080p output. Keep the 16:9 aspect ratio so avatars, icons, and text are not stretched during rendering; the walkthrough checks this before recording.

Commands:

```sh
cd apps/desktop
npm run demo:record
npm run demo:render
```

Or run both steps:

```sh
cd apps/desktop
npm run demo:video
```

Generated files are written under `apps/desktop/target/readme-demo/`, which is ignored by Git.

The current committed public demo asset is:

- `docs/assets/whatsvault-readme-demo.mp4`

To refresh it, run `npm run demo:video`, inspect the generated MP4, then copy the approved synthetic output from `apps/desktop/target/readme-demo/rendered/whatsvault-readme-demo.mp4` to the committed asset path above. Update `docs/assets/demo-assets-manifest.json` with the new SHA-256 hash only after confirming the asset uses synthetic data.

## Publish the inline player

GitHub does not render a playable video on the repository MP4 file page. The README uses a GitHub video attachment for its inline player and keeps the committed MP4 as a download fallback.

After inspecting the generated video, attach it to the demo-update pull request with a current GitHub CLI:

```sh
gh pr edit <pr-number> --attach docs/assets/whatsvault-readme-demo.mp4
```

Copy the resulting `https://github.com/user-attachments/assets/...` URL into the README on its own line. Verify that the README renders a player and that playback starts. Update the committed MP4, its manifest hash, and the attachment URL together so the two video copies stay consistent. See [GitHub attachment documentation](https://docs.github.com/en/github-cli/github-cli/attaching-files-with-github-cli).

## README Screenshot

The committed README screenshot lives at `docs/assets/whatsvault-synthetic-demo.png` and must be generated from the synthetic `/?demo=backup-chat` route.

Regenerate it from a running local preview:

```sh
cd apps/desktop
npm run dev
```

In another terminal:

```sh
cd apps/desktop
npm run demo:screenshot
```

The capture waits for the synthetic image and fonts, then frames the latest messages. It fails if either media attachment is clipped, so changes to the title bar or timeline cannot silently crop the demo. Inspect the generated PNG, then update its SHA-256 entry in `docs/assets/demo-assets-manifest.json` before running `npm run hygiene:public`.

Subtitle source lives at `apps/desktop/demo/readme-demo.srt`. Burned-in subtitles are opt-in because ffmpeg subtitle-filter path handling must be verified on both macOS and Windows:

```sh
cd apps/desktop
WHATSVAULT_DEMO_BURN_SUBS=1 npm run demo:render
```

Requirements:

- Node.js for the existing desktop build.
- Playwright browsers installed for the Chromium demo run.
- `ffmpeg` and `ffprobe` on `PATH` for `playwright-recast`.

Public-safety rules:

- Do not use real exported chats, real backups, real names, or real media in README videos.
- Keep narration, subtitles, and demo messages in English.
- Keep private-looking text detection centralized in `apps/desktop/scripts/privacy-rules.mjs`; the README screenshot capture, demo walkthrough, and public repository hygiene guard use the same evaluator.
- Keep committed README media listed in `docs/assets/demo-assets-manifest.json`; `npm run hygiene:public` verifies the asset hashes to catch accidental private replacements.
- Commit only small, intentional README assets generated from synthetic data.
