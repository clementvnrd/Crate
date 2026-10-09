# Update channels

Crate's in-app updater reads `channels/<channel>/latest.json` on this branch. It holds no code and is never merged. Only `scripts/release/publish.mjs` changes it (see `docs/RELEASING.md` on `develop`).
