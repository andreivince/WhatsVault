# Demo Video Workflow

The README demo video should be generated from code, not recorded manually from private chats.

Current approach:

1. Use the synthetic English demo route at `/?demo=1`.
2. Record a deterministic Playwright walkthrough with trace and video enabled.
3. Render the trace into a polished MP4 with `playwright-recast`.
4. Publish only synthetic demo output in README assets.

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

To refresh it, run `npm run demo:video`, inspect the generated MP4, then copy the approved synthetic output from `apps/desktop/target/readme-demo/rendered/whatsvault-readme-demo.mp4` to the committed asset path above.

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
- Commit only small, intentional README assets generated from synthetic data.
