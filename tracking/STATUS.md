# Fix status

This file is **the** source of truth for tracking: every defect in the [register](DEFECTS.md) has a box here, ticked in the same commit as its fix. The detail of every change is in the [CHANGELOG](../CHANGELOG.md); commits carry the identifier in brackets (`git log --grep "\[C11\]"`).

Legend: `[x]` fixed and verified · `[ ]` to do · _italic note_ = clarification or remaining work. The progress table is recomputed with `yarn status` (from the repository root).

<!-- progress:start -->
**Overall progress: 103 / 106 defects fixed (97%)**

| Step | Fixed | Progress |
| --- | --- | --- |
| Step 1 — Safety first | 2 / 2 | ██████████ |
| Step 2 — Stop destructive operations | 6 / 6 | ██████████ |
| Step 3 — Secrets and authentication | 4 / 4 | ██████████ |
| Step 4 — Safe Upgrader | 9 / 9 | ██████████ |
| Step 5 — Clean Mixed In Key | 8 / 8 | ██████████ |
| Step 6 — Accurate statistics | 10 / 10 | ██████████ |
| Step 7 — Exact DJ features | 18 / 18 | ██████████ |
| Step 8 — Robust frontend | 17 / 17 | ██████████ |
| Step 9 — Hygiene and tooling | 13 / 14 | █████████░ |
| Step 10 — Visual foundations | 5 / 5 | ██████████ |
| Step 11 — View-by-view compliance | 5 / 7 | ███████░░░ |
| Step 12 — Translation | 6 / 6 | ██████████ |
<!-- progress:end -->

## Owner-only actions

These actions cannot be done on the owner's behalf (personal accounts, decisions).

- [ ] **Change the Beatport account password**: it was in plain text in the code (C2). The code is clean, but the old password must be considered compromised.
- [x] **Confirm that Mixed In Key stays read-only** (CRA-110, confirmed 2026-09-30). The step 2 fixes are built on that principle.
- [ ] **Answer the open questions** in the [report](AUDIT-REPORT.md#decisions-to-make): other machines than this Mac (answered: macOS only, CRA-122) · Crate as the default player (answered: yes, CRA-124) · _the skewed Spotify history: owner wants it emptied (CRA-123); the in-app "Reset Spotify history" button is now ready in Crate Pulse (CRA-144), waiting for the owner to press it_

## Outside the register

- [x] Backup of `crate.db`, `db.key`, the automatic backup, the artwork and `Collection11.mikdb` in `~/Documents/Crate-sauvegardes/2026-09-26-avant-corrections/` (checksums verified)
- [x] Permissions of `~/.config/beatportdl/beatportdl-config.yml` reduced to `600` (it contains the old password)
- [x] Private personal GitHub repository `Crate`, remote `origin`; upstream `blackboxaudio/crate` becomes `upstream`
- [x] Upstream workflows (macOS/Windows builds on every push) switched to manual trigger so as not to consume Actions minutes; lightweight CI `ci.fork.yml` (Vitest)
- [x] Tracking documents: this file, [report](AUDIT-REPORT.md), [register](DEFECTS.md), [history](history/DISCUSSION-SUMMARY.md), `CLAUDE.md`
- [x] New way of working (2026-09-30): everything written in the repository is in English, `suivi/` became `tracking/`, `yarn suivi` became `yarn status`, standing rules in `CLAUDE.md`
- [x] Linear adopted as the live board (2026-09-30): team Crate App (`CRA`), 3 projects, 17 milestones, the 107 register defects as issues, decisions and scenario B ideas, rules in `CLAUDE.md` §1 — _optional GitHub integration left to the owner (CRA-136)_

## Register defects, by repair step

Minor defects and smells (about 90, unnumbered) are handled along the way in the step that touches the same file, and noted in the CHANGELOG.

### Step 1 — Safety first

_Exit criterion: Backups, immediate commit, secret removed before any push._

- [x] **C1** — 27,800 lines of work in no commit, branch or stash
- [x] **C2** — Beatport username and password in plain text in the code, rewritten into… — _Code cleaned (password rotation to be done by the owner)_

### Step 2 — Stop destructive operations

_Exit criterion: No automatic deletion, no test on real databases._

- [x] **C3** — Automatic strict purge: every Crate track missing from Mixed In Key is deleted, with cloud propagation
- [x] **C4** — Destructive writes to `Collection11.mikdb` (cascading DELETE on the Core Data tables,…
- [x] **C5** — Two `cargo test` tests open your real Crate (with the key) and Mixed In Key databases and write to them
- [x] **B5** — `prune_missing_tracks` deletes at startup the tracks of a folder that was merely renamed — _cleaning up missing files is still available, but only as a manual action_
- [x] **F6** — Full Mixed In Key sync and reload of the whole library on every window focus
- [x] **Q1** — `tauri.prod.conf.json` modified for a local build (`targets: ["app"]`, update artefacts… — _personal build: `yarn build:local`_

### Step 3 — Secrets and authentication

_Exit criterion: No more plain-text credentials, a single source of truth._

- [x] **B19** — Beatport tokens in plain text in `~/.config/crate/beatport_auth.json`, copied into the `beatportdl` folder… — _`beatportdl-credentials.json` is still required by beatportdl: written with `600` and deleted on sign-out_
- [x] **B20** — Retrieval of the token stored by DJ.Studio in its local configuration
- [x] **B21** — `validate_token` validates nothing and persists a fake "authenticated" state
- [x] **I6** — Duplicate `spotify_set_client_id` and `spotify_set_client_secret` commands; the Spotify secret is…

### Step 4 — Safe Upgrader

_Exit criterion: No neighbouring file touched, cues and tags preserved._

- [x] **C6** — The upgrader empties subfolders and deletes the .jpg, .m3u, .txt files of the destination folder
- [x] **C7** — FLAC files already present in the folder are taken as the download, and the MP3 is wrongly sent to the Trash
- [x] **C8** — The upgrade deletes the track and re-imports it with a new identifier
- [x] **B22** — Insufficient FLAC validation: a truncated file larger than 3 MB passes
- [x] **B23** — Scoring: a radio edit can replace an extended mix; fake Beatport default values (240,000 ms,…
- [x] **B24** — Artist splitting by regex without word boundary: "Daft Punk" becomes "Da"; panic on byte index… — _the byte-index panic was not found in the current code_
- [x] **B25** — No negative cache nor 429 handling: every opening triggers a full network scan
- [x] **B26** — `execute_upgrade_replacements`: unsafe order, ignored errors, several minutes without progress or… — _path re-read from the database, safe order, errors reported, progress; cancellation not implemented_
- [x] **I3** — No progress event for the Beatport download and the upgrader: the UI stays frozen for several… — _done for the upgrader (`upgrade-progress`); Beatport cart download without progress_

### Step 5 — Clean Mixed In Key

_Exit criterion: A single sync, incremental, read-only._

- [x] **B1** — The sync retriggers in bursts: 4 passes in 14 s observed at startup, each one rewrites the 276… — _verified on a copy of the real database: 2nd pass "0 updated" (276 tracks rewritten on every pass before)_
- [x] **B2** — Matching by title and artist, then rewriting another track's `file_path` — _title/artist matching limited to a single match whose file has disappeared; hash matching not done_
- [x] **B3** — MIK cues are recreated with new UUIDs on every sync, without tombstones; user cues… — _verified on a copy: 2,033 cues converted with no loss or duplicate_
- [x] **B4** — `get_track_cues` scans the whole MIK database and resolves every bookmark on each read, under the global lock
- [x] **B6** — macOS bookmarks resolved without `WithoutMounting` or `WithoutUI`, `CFError` leak
- [x] **B7** — `file_hash` is no longer stored on import, and re-importing an existing file fails on a constraint…
- [x] **B8** — Serato Markers2 parser off by one byte (index, position, name)
- [x] **B9** — The new `energy` column is neither serialised nor merged by the cloud sync

### Step 6 — Accurate statistics

_Exit criterion: One listen = one row, pauses excluded._

- [x] **C9** — The Spotify "repair" rewrites every listen shorter than 30 s as a full listen, on every startup — _listens already "repaired" by previous versions cannot be restored (see the open question on the Spotify history)_
- [x] **C10** — Spotify double counting: the live poller and the "recently played" sync record the same listen, and… — _the Spotify history already duplicated is not cleaned automatically (owner's decision)_
- [x] **B11** — Mixed In Key tracker: "file open in `lsof`" is counted as a listen; minutes capped… — _tracking disabled by default, can be enabled in Crate Pulse; no reliable detection of MIK playback_
- [x] **B12** — Local tracker: pauses not handled, wall-clock time including pauses, seek counted as a listen
- [x] **B13** — Rekordbox import: timestamp = time of import, full re-import on every sync, `master.db`… — _Rekordbox 6/7 `master.db` remains unreadable (encrypted): XML import is the supported path_
- [x] **B14** — Stats queries: comparisons of heterogeneous date strings, heatmap failing on a single row…
- [x] **B15** — Spotify JSON import: whole file passed as a string over IPC, no transaction, deduplication by scan… — _the JSON file still goes over IPC as text (acceptable for personal use)_
- [x] **B16** — Lock-order inversion between `spotify_disconnect` and the poller: possible deadlock freezing the whole…
- [x] **B17** — Spotify token refresh: silent failure, stale token returned, race between poller and commands
- [x] **B18** — Permanent OAuth server on 127.0.0.1:8888, `state` not enforced, parameters reflected in the response HTML

### Step 7 — Exact DJ features

_Exit criterion: Key 3 = cue 3, search for "You'll", valid XML._

- [x] **C11** — Hot cues shifted by one: the backend indexes from 1, the frontend accepts index, index−1 and index+1; the… — _existing cues are converted to 0-based automatically on the next Mixed In Key sync_
- [x] **C12** — Rekordbox XML export invalid as soon as a path contains `&`, `"` or `<`; URL not encoded for…
- [x] **C13** — Regression: `loadTracks()` without argument reuses the current filter
- [x] **C14** — Regression: dragging a tag onto a track no longer does anything (`data-track-id` removed)
- [x] **C15** — The Rust engine and the HTML preview can play at the same time
- [x] **B27** — No code writes `waveform_data`: the displayed waveform is always the fake 64-bar pattern — _computed on demand (86 ms for a 2.4 MB MP3), cached_
- [x] **B28** — FTS5: sanitisation makes tracks with an apostrophe, dash or slash unfindable, and `AND`, `OR`, `NOT`…
- [x] **B29** — Playback position = wall clock × speed, stale at the end of the track (no CoreAudio clock… — _exact rodio position at normal speed; clock × speed estimate kept when the tempo is changed_
- [x] **B30** — File associations with `rank: Default` and the `public.audio` umbrella; bundle `com.crate.app`… — _Crate appears in "Open With" without becoming the default player (open question in the report)_
- [x] **B31** — `StartupFile`: race at startup, several opened files overwrite each other
- [x] **B32** — `delete_tracks_and_files`: Trash through `osascript` per file, silent failures, deletion…
- [x] **B33** — Blocking I/O, subprocesses (`beatportdl`, `lsof`, `osascript`) and rusqlite under `std::sync::Mutex`… — _every heavy command and poller now runs on the blocking pool (`run_blocking`); the lock no longer spans file reads in `resync_mixed_in_key_tracks` and the XML export; the Mixed In Key startup, watcher and context-menu sync release the lock every 50 ms instead of holding it for the whole pass (CRA-142); the cloud-sync manifest is built once in SQL; ghost-track pruning and the bulk playlist and tag writes are transactions; the Keychain, diagnostics, `get_tracks`, Beatport post-download and Mixed In Key probing work runs on the blocking pool; also fixed: audio replies paired with the wrong request, unbounded analysis. Left: a shorter library lock for `get_tracks`, which needs a separate read connection (CRA-143)_
- [x] **B34** — Services instantiated twice: the state managed by Tauri is not the one used by the background tasks
- [x] **B35** — `get_duplicate_count` runs a full duplicate scan on every `duplicates-updated` event
- [x] **F1** — Keys 1 to 8 captured everywhere, with no view or modifier condition
- [x] **F2** — Space in the Player view starts a recent track instead of pausing the preview; duplicated logic
- [x] **F3** — Space or Enter on a focused row triggers playback and the global shortcut
- [x] **F11** — Cues and waveform never loaded for an external file: a UUID is sent to `get_track_cues`

### Step 8 — Robust frontend

_Exit criterion: Backend errors visible, no superfluous IPC call._

- [x] **F4** — `withTimeout`: an initialisation longer than 2.5 s loses the cleanup functions (Tauri listeners…
- [x] **F5** — Splash closed by a module timer at 1.5 s: race with settings loading, possible flash of…
- [x] **F7** — The Toolbar effect triggers a `get_duplicate_count` call on every library mutation
- [x] **F8** — Beatport: favourites and playlists "created" only locally with a misleading success toast; cart… — _favourites and playlists stay local, but are announced as such_
- [x] **F9** — Side effects on module import in `shared/` (3-minute Beatport timer, splash timer) and…
- [x] **F10** — Mutation of a `$derived` value (`activeHeroTrack.is_in_library = true`)
- [x] **F12** — Race of stale responses in position tracking (async interval calling `getPlaybackState`…
- [x] **F13** — "Sync with Mixed In Key" in the context menu ignores the selection and resyncs the whole database
- [x] **F14** — Upgrader: confidence displayed as "0.96%" instead of "96%"; the replace button stays active… — _the "0.96%" seen during the audit came from the harness's fake data, not from the app; button disabled without a Beatport session_
- [x] **F15** — The Mixed In Key badge tooltip stays visible after the pointer leaves — _fixed in the shared `Tooltip` component (all tooltips)_
- [x] **I1** — Backend error messages lost: Tauri rejects with a string, and the new stores test… — _helper applied to all stores and components (upstream ones included)_
- [x] **I2** — Two Beatport settings with no effect: the downloader forces `lossless` and always runs the Mixed In Key sync — _ineffective settings removed from the interface (FLAC format enforced by the verification, MIK sync handled by the watcher)_
- [x] **I4** — `BeatportAuthState` carries two conventions (camelCase on the TS side, snake\_case on the Rust side); consistency…
- [x] **I5** — `record_listen_event` accepts a full event from the webview but is never called
- [x] **I7** — `library-updated` emitted without a listener; `searchType` parameter of the Beatport search ignored
- [x] **I8** — About twenty Rust `Option<T>` fields typed `?: T` on the TS side although the actual value is `null` — _Beatport types; album types already accepted `null`_
- [x] **I9** — Display options stored only in `localStorage`: neither backed up nor synced

### Step 9 — Hygiene and tooling

_Exit criterion: Green CI, clean fmt/lint, cross-platform builds._

- [x] **Q3** — Windows/Linux build broken: `RunEvent::Opened` only exists on macOS, iOS and Android — _code fixed; Linux compilation verified by the CI Rust job (Windows not verified)_
- [x] **Q4** — Clippy with `-D warnings`: 37 errors (16 dead code); cargo fmt: 35 files
- [x] **Q5** — ESLint: 38 errors; Prettier: 60 files
- [x] **Q6** — No workflow runs `yarn test` or `cargo test` — _Linux jobs: frontend (format, lint, types, Vitest) and Rust (clippy, tests); the upstream macOS/Windows matrix stays manual_
- [x] **Q7** — No tests on the risky paths: file replacement, purge, pollers, OAuth; 3 tests read… — _tests added for file replacement, sync/purge, listen trackers, import, export; tests on the real MIK database removed_
- [x] **Q8** — Vitest runs on Vite 8.2 while the app uses Vite 7.3; `test:coverage` with no coverage provider…
- [x] **Q9** — Dev, staging and prod icons have become byte-for-byte identical; `.ico`, Windows, iOS and Android icons… — _upstream already shipped the three identical; the dev and staging icons now carry a blue `DEV` / purple `STG` band (all formats regenerated with `tauri icon`), prod untouched_
- [x] **Q10** — CHANGELOG, version, README and documentation site not updated; the shortcuts doc contradicts the… — _CHANGELOG, README and shortcuts page up to date; version 0.3.0 to be set at the first personal build_
- [x] **Q11** — Files that must not be committed: `SYNTHESE_DISCUSSION.md` (personal paths),… — _Swift script deleted; the summary is archived in `tracking/history/` (private repository); fonts handled with D8_
- [x] **Q12** — No `CLAUDE.md`: every assistant rediscovers the rules (`desktop` feature, CI clippy flags,…
- [x] **Q13** — `yarn dev` compiles Rust in `--release`: every change costs several minutes (upstream)
- [ ] **Q14** — 87 `yarn audit` alerts (61 high) in the transitive tooling; `cargo audit` not installed locally — _partial: `yarn audit` is at 0 (it was 122 advisories, 80 high) after re-resolving the lockfile and moving the exact Vite pin to 7.3.6; `cargo audit` is still not installed and needs the owner's go (CRA-88)_
- [x] **Q15** — No design guard rails for assistants: the strict rules only exist in the register,… — _`DESIGN.md`, `design` agent and 5 skills, `yarn design:scan` (1,220 offending lines at the start); automatic visual verification is now `yarn test:e2e` on the versioned harness (CRA-134)_
- [x] **B10** — Migration numbering diverging from upstream (15 entries, labelled 7 to 16, 6 skipped) — _labels fixed (6 to 15), order unchanged: the local database has already applied these migrations_

### Step 10 — Visual foundations

_Exit criterion: No invisible element in light or dark._

- [x] **D1** — `dark:` follows the OS theme, not the app's: the build 40 "light mode fixes" are… — _`dark:` variant rewired to `[data-theme]` (one CSS line); cleaning up the 43 usages will be done view by view (step 11)_
- [x] **D2** — About 70 classes reference non-existent tokens (`surface-3`, `surface-4`, `stroke-strong`,…
- [x] **D6** — Modals taller or wider than the window: the Duplicate Killer footer and its "Delete… button
- [x] **D8** — Jost and DM Sans fonts offered in the settings but not wired (`[data-font='jost']` missing); 15… — _OFL licences referenced in `static/fonts/LICENSES.md` (full text via the official link)_
- [x] **D9** — Six icons in use do not exist in `Icon.svelte`; the Beatport SVG logo loaded as `<img>` with… — _icon name not typed, but a Vitest test fails on any unknown icon_

### Step 11 — View-by-view compliance

_Exit criterion: Strict design rules respected._

- [ ] **D3** — Broken light theme: Pulse integration cards dark at the top and light at the bottom; header badges… — _partial: family colour tokens with light text values, `--brand-on` on accent fills, Beatport follows the theme, the four CRA-141 touches, dark theme unchanged otherwise; left: the energy badge in light, the toolbar count badges and the Mixed In Key logo label, Beatport's "Connected" text, the palette classes listed in DESIGN.md "Known deviations"_
- [x] **D4** — Header at minimum width (1000 px): the icons overlap the segmented control… — _verified by measurement in the harness at 1000, 1280, 1440 and 1600 px: no overlap, all tools visible_
- [x] **D5** — Player hero with a fixed height (`h-[225px]` for about 260 px of content): at 1000×640 the transport is… — _transport never covered (verified at 1000×640); at that size the recents list keeps ~2 rows_
- [x] **D7** — Beatport table at 1000 px: the title column shrinks to one character — _title column 52 to 288 px at 1000 px; date and genre hide below 832 and 720 px; identical from a 1400 px window; the Player recents overflow is fixed too_
- [ ] **D10** — Accessibility: about 25 icon buttons without a name (transport, segments, MIK badge, recents actions);… — _partial: names, waveform slider, recents grid, switches, focus traps, reduced motion and the keyboard focus outline are done; left: the global shortcuts block Enter and Space on focused buttons (owner decision, CRA-100), 17 `svelte-ignore a11y`, the heatmap is not focusable_
- [x] **D11** — Components reinvented instead of the shared ones: 4 segmented controls, checkbox, select, spinner, tooltip and… — _shared SegmentedControl, KeyBadge, EnergyBadge, Button tones, `Modal` for the Spotify panel, `Checkbox appearance="native"` and `Spinner` everywhere, dark look unchanged; the Beatport search scope stays a native `<select>`_
- [x] **D12** — Permanent "Build 57" badge next to the logo and "PRO" labels on third-party brands

### Step 12 — Translation

_Exit criterion: App in English: no French string._

- [x] **L1** — 17 new components with no translation at all, about 220 strings — _423 new keys in `en.json` and `fr.json` (1259 each, in parity); hard-coded visible strings in `apps/desktop/src` go from 139 to 20 and text attributes from 51 to 4; Pulse, Player, Beatport, Upgrader, Duplicate Killer, tags, discovery and the store toasts are translated; French uses "morceau" for a track and keeps "Cover" (owner's wording, CRA-103); what remains is brand names, code and data values, plus the Camelot key names in French_
- [x] **L2** — 13 locales without the 49 to 53 keys added by the fork — _decided (CRA-114): French and English complete, the other 13 fall back to English, documented in the README; Vitest guards keep EN and FR aligned; one key was missing in French and is added_
- [x] **L3** — Numbers and units formatted the English way in French ("2,310"), "plays" and "écoutes" mixed in Pulse — _numbers and dates follow the app language (`formatNumber`, `formatDate`); Pulse says "plays" in English and "écoutes" in French throughout; Player dates and remaining counts follow the app language_
- [x] **L4** — Rust error messages in French — _all backend messages are English; the frequent actionable ones (Beatport sign-in, invalid album folder) are translated by the interface through `localizeBackendError`; the Spotify sign-in pages follow the app language_
- [x] **L5** — `register('en')` and `register('fr')` kept after `addMessages`: initialisation goes back through the… — _removed; two Vitest tests pin the synchronous switch_
- [x] **L6** — The README advertises 11 languages while the repository contains 15

## Session log

| Date | Work | Steps |
| --- | --- | --- |
| 2026-09-25 | Full audit: report, register of ~180 defects, browser harness | — |
| 2026-09-26 | Backups, secrets removed, baseline commit, GitHub repository, tracking documents | 1 |
| 2026-09-26 | No more automatic deletion nor writes to Mixed In Key; prod config restored | 2 |
| 2026-09-26 | Beatport session in the Keychain, end of DJ.Studio scraping, Spotify secret never sent back to the webview | 3 |
| 2026-09-26 | Safe Upgrader: isolated folder, in-place replacement, FLAC fully decoded, scoring fixed | 4 |
| 2026-09-26 | Incremental and idempotent Mixed In Key sync, stable cues, import/Serato/energy fixed | 5 |
| 2026-09-26 | Accurate statistics: one Spotify listen = one row, pauses excluded, Rekordbox sets dated | 6 |
| 2026-09-26 | Exact DJ features: hot cues, XML, real waveform, search, position, regressions, Trash | 7 |
| 2026-09-26 | Robust frontend: visible errors, leak-free init, reliable cart, display options saved | 8 |
| 2026-09-26 | Hygiene: fmt/clippy/ESLint at zero, Linux CI, git hooks restored | 9 |
| 2026-09-26 | Visual foundations: tokens, `dark:` tied to the theme, bounded modals, fonts, icons | 10 |
| 2026-09-30 | `design` agent: `DESIGN.md`, skills (system, audit, build, image to code, Playwright verification), `yarn design:scan` scanner | 9 |
| 2026-09-30 | **Clean break — new way of working**: everything in the repository in English, `suivi/` → `tracking/`, `AVANCEMENT.md` → `STATUS.md`, `yarn status`, all earlier documents translated, standing rules in `CLAUDE.md` | — |
| 2026-09-30 | **Linear adopted as the live board**: team Crate App (`CRA`), 3 projects and 17 milestones, 107 register defects loaded as issues (94 Done), 5 owner actions and decisions, 10 scenario B ideas, beginner's guide, rules in `CLAUDE.md` §1 | — |
| 2026-09-30 | **Answers in Linear + hourly watch**: owner answers by commenting or closing issues, assistant comments prefixed `🤖 Claude:`, scheduled task `crate-linear-hourly-check` (highest autonomy, one defect per run, ends at In Review) | — |
| 2026-09-30 | **Owner decisions read from Linear**: scenario A then B, French and English complete, macOS only, Mixed In Key read-only; Spotify cleanup, default player, visual language and Beatport still open | — |
| 2026-09-30 | **Owner decisions read from Linear (2nd pass)**: Beatport download stays while the repository is private (CRA-113), colour families are kept and a graphic charter is written (CRA-115 → CRA-140); Spotify history backed up and verified, deletion awaiting the owner's go (CRA-123) | — |
| 2026-09-30 | Dev and staging icons recognisable in the Dock (blue `DEV` and purple `STG` bands), all formats regenerated | 9 |
| 2026-09-30 | Synchronous English/French language switch (two redundant `register` calls removed) | 12 |
| 2026-09-30 | Language policy applied: EN/FR aligned and guarded by tests, one missing French key added, fallback documented | 12 |
| 2026-09-30 | Numbers and dates follow the app language (`formatNumber`, `formatDate`) — Pulse wording left for L1 | 12 |
| 2026-09-30 | Backend errors in English, frequent ones translated by the interface, Spotify sign-in pages follow the app language | 12 |
| 2026-09-30 | **Graphic charter** written in `DESIGN.md` (four colour families, scope rules, computed contrast, checklist, deviations); five small choices put to the owner | 11 |
| 2026-09-30 | Backend work moved off the async runtime; audio reply pairing, analysis cap and USB poller fixed | 7 |
| 2026-09-30 | Scenario B started: listening-history export to CSV/JSON (backend) | — |
| 2026-09-30 | Scenario B: smart-playlist listening criteria (backend, migration 16) | — |
| 2026-09-30 | Scenario B: "Your week / Your year" recap (backend) and exact statistics windows | — |
| 2026-09-30 | Scenario B: next-track suggestion from real Rekordbox transitions (backend) and harmonic key module | — |
| 2026-09-30 | Scenario B: discovery funnel (backend, migration 17) | — |
| 2026-09-30 | **Browser harness and end-to-end checks** versioned (`yarn harness`, `yarn test:e2e`, ratchet baseline) | 9 |
| 2026-09-30 | Scenario B: Rekordbox set timeline (backend) | — |
| 2026-09-30 | Scenario B: Crate / Mixed In Key / Rekordbox discrepancy report (backend, read only) | — |
| 2026-09-30 | Scenario B: assisted physical organisation with preview, journal and undo (backend, migration 18) | — |
| 2026-09-30 | Scenario B: Set mode analysis, bridge tracks and proposed order (backend) | — |
| 2026-09-30 | **Owner decisions read from Linear (3rd pass)**: Q2 cancelled (macOS only), visual language reopened after "sienne" became "cyan", Spotify reset waiting for a chat approval or the in-app button | — |
| 2026-09-30 | Shared SegmentedControl, KeyBadge, EnergyBadge and family Button props, dark theme unchanged (D11 partial) | 11 |
| 2026-09-30 | Bulk playlist and tag writes made atomic (one transaction each, 10 tests) | 7 |
| 2026-09-30 | Cloud-sync manifest built once, shard chosen in SQL, off the async workers (about 3.5 times faster at 10,000 tracks) | 7 |
| 2026-09-30 | Ghost-track pruning made atomic and lock-free for disk checks; Keychain, diagnostics and get_tracks commands moved to the blocking pool | 7 |
| 2026-09-30 | Beatport post-download steps and Mixed In Key process probing moved to the blocking pool | 7 |
| 2026-09-30 | Mixed In Key sync releases the database lock every 50 ms instead of holding it for the whole pass | 7 |
| 2026-09-30 | Beatport and Player track titles readable at 1000 px (D7) | 11 |
| 2026-09-30 | Accessible names, keyboard waveform, recents grid, switches, focus traps, reduced motion and focus outline (D10, partial) | 11 |
| 2026-09-30 | `yarn audit` from 122 advisories to 0: Vite pin 7.3.6, transitive packages re-resolved, svelte-i18n esbuild override (Q14, partial) | 9 |
| 2026-09-30 | Typed frontend API and contract test for the nine scenario B backends | — |
| 2026-10-01 | Interface strings extracted to 423 new en/fr keys, plays/écoutes unified, numbers and dates localised (L1 partial, L3) | 52 |
| 2026-10-01 | Scenario B screens in Pulse: recap, discovery funnel, Rekordbox set timeline, history export (CRA-125 to CRA-128) | — |
| 2026-10-01 | Energy column in the Rekordbox set timeline (CRA-127; jump marker left for a backend change) | — |
| 2026-10-01 | French says "morceau" for a track and keeps "Cover", per the owner's answer in CRA-103 (L1 closed) | 12 |
| 2026-10-01 | "Reset Spotify history" button in Crate Pulse: backup then delete, confirmation dialog (CRA-144) | — |
| 2026-10-01 | Discovery funnel renamed "Crate to booth" / "Du bac à la cabine", per the owner's answer in CRA-126 | — |
| 2026-10-01 | CRA-141 rollout: family colour tokens, black text on accent fills, Beatport follows the theme, the four touches (D3, partial); Spotify modal, Rekordbox and add-to-library buttons, native checkboxes and spinners on the shared components (D11) | 11 |
