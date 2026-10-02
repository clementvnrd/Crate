# Browser harness

Crate's interface talks to the Tauri backend (`invoke`, plugins, events) from its first frame. Opened in a plain browser it lands on the crash screen. The harness runs the **real app** in a browser with a **fake backend** behind `@tauri-apps/api/mocks`, so design work (light and dark, small windows, contrast, accessibility) can be checked visually and automatically without building the Rust side.

Nothing here ships: the harness is only served by `yarn harness` and is not part of `yarn build`.

## Run it

From the repository root (`~/Coding Projects/crate`):

```bash
yarn harness          # http://localhost:1430/
```

Port 1430 is fixed (`strictPort`); `yarn dev` uses 1420 and 1421, so both can run together. Stop it with Ctrl+C.

The app starts on the library with 16 tracks, in the dark theme, in English, logged in to Beatport.

## URL parameters

| Parameter | Values | Effect |
| --- | --- | --- |
| `theme` | `dark`, `light`, `system` | Theme (default `dark`, so a run never depends on the machine). |
| `accent` | `blue`, `indigo`, `violet`, `purple`, `pink`, `rose`, `orange`, `amber`, `emerald`, `teal` | Accent colour. |
| `lang` | `en`, `fr` (and the other locales) | Interface language. |
| `font` | `open-sans`, `jost`, `dm-sans`, `inter`, `nunito`, `fira-code`, `ibm-plex-mono`, `source-code-pro` | Font. |
| `beatport` | `out` | Start logged out of Beatport (default: logged in). |
| `library` | `empty` | Empty library, empty Discovery, empty Pulse recap and funnel: the empty states. |
| `onboarding` | `1` | Show the onboarding wizard. |
| `playing` | a track id, e.g. `trk-03` | Show that library track, paused, in the player bar. |
| `dev` | `1` | Report a development build (DEV badge, developer tools button). |
| `latency` | milliseconds, e.g. `400` | Delay every command: loading states. |
| `discrepancyReport` | `fail` | Make `get_discrepancy_report` reject (the discrepancy report's error state). |

Example: `http://localhost:1430/?theme=light&lang=fr&accent=amber&playing=trk-03`.

`theme`, `accent`, `lang` and `font` are written to the same `localStorage` keys the app uses (`crate-theme`, `crate-accent`, `crate-language`, `crate-font`), so they persist across reloads until another value is given. Settings changed in the UI live in memory and are lost on reload (the localStorage keys survive).

## Reaching every screen

| Screen | How |
| --- | --- |
| Library | Start page. Sidebar: playlists, folders, smart playlist, tags tab, the USB stick. |
| Player | `Player` in the segmented control. `Album mode` (French `Mode Album`) shows the album grid. |
| Beatport | `Beatport` in the segmented control. Charts, catalogue, purchases, favourites, playlists. |
| Discovery | Globe button of the toolbar. |
| Pulse | Chart button of the toolbar. |
| Settings | The gear button of the toolbar (named `Settings` / `Paramètres`), or `Cmd+,` / `Ctrl+,`. Every tab opens. |
| Duplicate Killer | Overlapping-squares button with the red badge. |
| Beatport Quality Upgrader | Sparkles button with the green badge. |
| Discrepancy report | Settings → Library tab → "Open the discrepancy report". |

## How the fake backend answers

`install.ts` runs before the app code (see `vite.harness.config.ts`) and calls `mockIPC(handler, { shouldMockEvents: true })`, so `invoke()` and `listen()` both work. For every command, `backend.ts` looks, in order, for:

1. **A precise handler** in `handlers/` (`library`, `organise`, `player`, `beatport`, `discovery`, `stats`, `system`, `maintenance`, `plugins`), working on an in-memory copy of the fixtures.
2. **A declared no-op** in `handlers/noops.ts`: native side effects (menu, Now Playing, developer tools) and constant answers.
3. **A pattern default** from `defaults.ts`: `*_count` and `*_size` answer `0`, predicates (`is_*`, `has_*`) `false`, getters of a plural noun `[]`, everything else `null`. The command is reported once with `console.warn('[harness] unmocked command: <name>')` and added to `window.__harness.unmocked`.

So the **unmocked list is the to-do list**: a command in it is answered with a guess, not with data. Start-up, every view and every settings tab currently leave it empty (`e2e/harness.e2e.ts` checks this). Mutations that are not listed above (import, export, cloud sign-in, Spotify, backups…) are unmocked and answer `null`.

In the browser console, `window.__harness` exposes:

| Field | Content |
| --- | --- |
| `params` | The parsed URL parameters. |
| `unmocked` | `Set` of the commands answered by a pattern default. |
| `calls` | The last 300 calls (`command`, `args`, `mocked`). |
| `emit(event, payload)` | Send a backend event to the app, e.g. `__harness.emit('devices-changed', [])` raises the "CDJ-STICK disconnected" toast. |

Native dialogs (open, save, ask) always answer "cancelled", the clipboard and the opener do nothing, there is never an update available.

## Fixtures

Typed with the real types of `shared/types`, deterministic (no `Math.random`, no `Date.now()`: dates derive from `fixtures/reference.ts`, `2026-09-28`). Cover art is generated as inline SVG data URLs, so the harness needs no image files and no network.

| File | Content |
| --- | --- |
| `fixtures/library.ts` | 16 tracks (BPM 90 to 138, all Camelot numbers, energy 2 to 10, every format), stress cases on purpose: a very long title (track 3), a very long artist (4), four tracks without artwork (5, 9, 12, 16), one track with no metadata at all (16). Three tag categories, playlists and folders (one smart, one with a very long name), one USB stick, cues and waveform. |
| `fixtures/player.ts` | 7 albums (two without artwork, one with a very long title) with their tracks; 4 recently opened files. |
| `fixtures/beatport.ts` | 10 genres, 6 charts, 24 catalogue tracks, 3 playlists, favourites, purchases, artist pages, and the 3 Upgrader matches. |
| `fixtures/discovery.ts` | 6 releases (Bandcamp, SoundCloud, YouTube, Discogs, one without metadata), 3 followed sources (one in error), the discovery funnel by source. |
| `fixtures/stats.ts` | Pulse: summary, top tracks and artists, harmonic and BPM stats, heatmap, recent listens, 3 Rekordbox sessions, scaled by the selected time range; the week and year recap (any offset), the timeline of each set, the history export count. |
| `fixtures/maintenance.ts` | Duplicate Killer: two groups (exact hash, metadata match). Discrepancy report: 2 missing files (one `missing`, one `volume_unmounted`), a Mixed In Key comparison with a key/tempo difference and both "missing from" lists, a Rekordbox comparison once an XML path is given. |
| `fixtures/system.ts` | Settings (appearance read from localStorage), app info, audio devices, diagnostics, cloud sync (signed out), Mixed In Key status, backup info. |

## Add a fixture or a handler

1. **A new command the app calls** (`window.__harness.unmocked` names it): add a function to the map of the matching file in `handlers/` (or a new file, registered in `backend.ts`). Arguments arrive as the app sends them (camelCase keys); return what the real command resolves with, or `throw 'message'` to simulate a backend rejection (Tauri rejects with a string).
2. **More or different data**: edit the file in `fixtures/`. Keep it deterministic and typed; use `isoAgo()` for dates. Handlers clone what they return, so the app cannot corrupt the fixtures.
3. **A new URL parameter**: read it in `params.ts`, add it to `HarnessParams` in `types.ts`, use it where the state or a handler needs it, document it above. Appearance parameters (theme, accent, language, font) are handled earlier, by `early.ts`, because they must be in place before the page paints.
4. Run `yarn check:svelte` (the harness is type-checked) and `yarn test:e2e`.

## Automated checks

From the repository root:

```bash
yarn test:e2e                   # 64 cases + the harness checks, about a minute
yarn test:e2e:update-baseline   # rewrite e2e/baseline.json after fixing findings
```

`e2e/views.e2e.ts` opens each view (library, player, beatport, discovery, pulse, settings, duplicates, upgrader) in both themes, at 1000x600 and 1400x900, in English and French. Each case checks that the page has no uncaught error and no `console.error` (allow-list in `e2e/app.ts`, empty), that the view shows, writes `e2e/screenshots/<view>-<theme>-<width>-<lang>.png` (git-ignored), runs the in-page audit of `.claude/skills/crate-visual-check/scripts/ui-audit.js`, and compares its counts with `e2e/baseline.json`.

The baseline is a **ratchet**: the test fails when a category has more findings than the baseline, and prints a message when it has fewer. Design debt can only shrink; when a fix lowers a count, run `yarn test:e2e:update-baseline` and commit the new `e2e/baseline.json` with the fix.

The e2e run blocks the Google Fonts request (answered with an empty stylesheet) so text is always measured in the system sans-serif: results do not depend on the network. The harness in a normal browser still loads the real fonts, so a screenshot taken there can differ from the ones in `e2e/screenshots/`.
