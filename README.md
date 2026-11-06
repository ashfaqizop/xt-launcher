# XT Launcher

A clean, fast Minecraft Java launcher. Tauri 2 + Rust backend, Svelte + [`@m3e/web`](https://github.com/matraic/m3e) (Material 3 Expressive) UI, Bricolage Grotesque only.

## Status

Phases 1-3 done: scaffold, CI, signed OTA pipeline, splash with intro, M3E app shell (home card, settings, accounts, profiles).

## Building

There is no local Rust requirement: GitHub Actions builds everything.

- `CI` runs on every push and pull request (type-check, frontend build, `cargo check`, `clippy`).
- `Release` runs when a `v*` tag is pushed (the tag must match `version` in `src-tauri/tauri.conf.json`). It publishes a signed NSIS installer plus `latest.json`, which the in-app updater reads.

### One-time repo secrets

| Secret | Purpose |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Updater signing key (public half is in `tauri.conf.json`) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for that key |

Losing the private key means installed copies can no longer receive updates, so keep a backup outside the repo.

## Frontend development

```
npm install
npm run dev      # Vite on :1420 (the Tauri window needs the Rust side, so use CI builds for the full app)
npm run check
npm run build
```

Brand sources are in `assets/brand/`. Regenerate icons with `npx tauri icon assets/brand/app-icon.png -o src-tauri/icons`.
