## Summary

- What changed and why:

## Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cd apps/desktop && npm test`
- [ ] `cd apps/desktop && npm run hygiene:public`
- [ ] `cd apps/desktop && npm run release:readiness`
- [ ] `cd apps/desktop && npm run build`
- [ ] `cd apps/desktop && npm run visual:check`

## Privacy

- [ ] No real chats, backups, SQLite databases, plist files, media, raw proof dumps, names, phone numbers, file IDs, or private paths were committed.
- [ ] Any screenshots, videos, or fixtures are synthetic or fully redacted.
