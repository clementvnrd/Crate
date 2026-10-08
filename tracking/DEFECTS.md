> **Frozen snapshot of the audit of 25 September 2026.** Each defect keeps its identifier (C1, B12, D4…): that is what you find in [STATUS.md](STATUS.md), in the [CHANGELOG](../CHANGELOG.md) and in commit messages (`git log --grep "\[C2\]"`). Editable version: [Crate — Defect register & fixes](https://claude.ai/code/artifact/5f3fd54e-ebb9-457f-bc3a-b584ad01a41c).

# Crate — Defect register & fixes


## How to read this register

This register lists Crate's defects, sorted by severity and then by area, each with its fix and its effort. It complements the report "Crate — Full audit & vision".

| Severity | Meaning for a personal project |
| --- | --- |
| Critical | Data loss, leaked secret, a DJ feature that gives a wrong result, unsaved work |
| Major | Bug visible day to day, degraded performance or reliability, broken tooling |
| Minor | Limited or rare defect, debt that is easy to pay off |
| Smell | Code quality, with no direct user impact |

| Effort | Indicative duration |
| --- | --- |
| XS | less than an hour |
| S | half a day |
| M | one to two days |
| L | three days or more |

**Method.** Six parallel audits read the code in full: Rust for the new features, core Rust (database, library, audio, export), frontend, design and accessibility, IPC contract, repository and CI. They produced about 250 overlapping raw findings; this register deduplicates them.

**Verified by execution**: Vitest, svelte-check, cargo check, cargo test, clippy with the CI flags, ESLint, Prettier, cargo fmt, mobile build on an isolated copy, FTS5 queries in sqlite3, two reproduced panics, the real app run for 25 minutes with logs, and every view captured in a browser with a fake backend. The three most serious defects (password, purge at startup, tests on real databases) were re-checked by hand.

**Verified by reading only**: the Windows build, the release pipeline and the behaviour of `beatportdl`. File paths cited start from `src-tauri/src` for Rust and from the repository root for everything else.

## Summary table

After deduplication the register keeps about 180 defects, including 15 critical and 72 major. The backend holds two thirds of the critical ones; design and i18n weigh mostly in volume of work.

| Area | Critical | Major | Minor and smells | Prefix |
| --- | --- | --- | --- | --- |
| Rust backend | 10 | 35 | about 50 | C, B |
| Svelte frontend | 4 | 15 | about 25 | C, F |
| Backend ↔ frontend consistency | 0 | 3 | 6 | I |
| Design, responsive, accessibility | 0 | 10 | 2 | D |
| Internationalisation | 0 | 2 | 4 | L |
| Quality, tooling, CI | 1 | 7 | 7 | C, Q |
| **Total** | **15** | **72** | **about 94** |  |

Estimated total effort to fix everything: about 5 to 6 weeks of work, of which 3 weeks for the critical defects, the functional major defects and hygiene, and 2 to 3 weeks for visual unification and translation.

## Critical defects

Fifteen unique critical defects: nine can destroy or corrupt your data, one exposes a secret, four make a DJ feature wrong, and the last one is the complete absence of commits. C1 to C5 must be dealt with before using the app on your library again.

| # | Defect | Where | Impact | Fix | Effort |
| --- | --- | --- | --- | --- | --- |
| C1 | 27,800 lines of work in no commit, branch or stash | whole repository | One unlucky `git checkout` or `git clean` wipes out weeks of work | Immediate local branch with a snapshot commit, then clean splitting in phase 3 | XS |
| C2 | Beatport username and password in plain text in the code, rewritten into `~/.config/beatportdl/beatportdl-config.yml` | `services/beatport/downloader.rs:108-109`, `client.rs:409` | Account leak on the first push; the file on disk already contains the secret | Change the password now; remove the literals; read credentials from the macOS Keychain; add gitleaks to pre-commit | XS |
| C3 | Automatic strict purge: every Crate track missing from Mixed In Key is deleted, with cloud propagation | `lib.rs:805-806`, `services/library/mik_db.rs` | At startup, on every window focus and on every change to the MIK database; an unmounted volume is enough to trigger deletions | Remove the call at startup; replace it with a "tracks missing from MIK" report with explicit confirmation | S |
| C4 | Destructive writes to `Collection11.mikdb` (cascading DELETE on the Core Data tables, `wal_checkpoint(TRUNCATE)`) | `mik_db.rs:114-189`, `upgrader.rs:884`, `update.rs:378` | Possible corruption of the Mixed In Key library, with no backup | Open MIK strictly read-only; if a write is ever wanted, a `.mikdb.bak` backup and opt-in | S |
| C5 | Two `cargo test` tests open your real Crate (with the key) and Mixed In Key databases and write to them | `mik_db.rs:1022-1046` | Every `cargo test` alters your library | Delete these tests; fixtures on temporary databases; forbid `~/Library` in tests | XS |
| C6 | The upgrader empties subfolders and deletes .jpg, .m3u, .txt files from the destination folder | `downloader.rs:151-198` (`flatten_and_clean_destination`) | The default folder is your music folder: artwork, playlists and notes deleted | Download into a dedicated temporary folder, move only the validated FLAC, never clean a user folder | S |
| C7 | FLAC files already present in the folder are taken for the download, and the MP3 is wrongly sent to the Trash | `downloader.rs`, `upgrader.rs` | Replacement with the wrong file | Identify the downloaded file by its exact path in the temporary folder, check duration and metadata | S |
| C8 | The upgrade deletes the track and re-imports it with a new identifier | `upgrader.rs` | Cues, tags, playlists, rating, colour, play count and cloud link lost | Update `file_path`, format and bitrate of the existing track in place, inside a transaction | M |
| C9 | The Spotify "repair" rewrites every listen shorter than 30 s into a full listen, at every startup | `services/stats/recorder.rs` (`repair_spotify_historical_durations`) | Inflated minutes and streams, visible in the log ("Repaired 5 Spotify historical listen events") | Remove the repair; if needed, a single idempotent migration | XS |
| C10 | Spotify double counting: the live poller and the "recently played" sync record the same listen, and a pause followed by a resume creates a new listen | `services/stats/spotify.rs` | Wrong tops and totals | Deduplication key (Spotify identifier + rounded start timestamp) with a UNIQUE constraint; a single recording source | S |
| C11 | Hot cues off by one: the backend indexes from 1, the front end accepts index, index−1 and index+1; the memory cue takes up a pad | `shared/stores/player.ts:1120-1130`, `PlayerView.svelte:151-162` | Key 3 jumps to cue 2; the pads lie | Zero-based everywhere like upstream (ANLZ, PCOB); a single conversion at MIK import; pads filtered on `cue_type = hot` | S |
| C12 | Rekordbox XML export invalid as soon as a path contains `&`, `"` or `<`; URL not encoded for non-ASCII characters; memory cues exported as hot cue A, loops lost | `services/export/rekordbox_xml.rs` | Rekordbox rejects the file or imports the wrong cues | Escape every attribute; `file://localhost/` + percent-encoding; `Num=-1` for memory cues, 0 to 7 for hot cues, `End` for loops | S |
| C13 | Regression: `loadTracks()` with no argument reuses the current filter | `apps/desktop/src/lib/stores/library.ts:51-65` | Removing the last tag no longer clears the filter; the "Mix harmonique" (harmonic mix) filter becomes sticky | Restore the upstream semantics; add an explicit `reloadWithCurrentFilter()` method | XS |
| C14 | Regression: dragging a tag onto a track no longer does anything (`data-track-id` removed) | `TrackRow.svelte`, `useDragDropCoordination.ts:159` | Upstream feature broken | Restore the attribute and the hover state; component test | XS |
| C15 | The Rust engine and the HTML preview can play at the same time | `shared/stores/player.ts:481-492`, `555-562` | Two overlapping sounds in standalone player mode | A single entry point "stopAllEngines()" called before any playback, whatever the source | S |

## Rust backend

The backend has 35 major defects besides the critical ones, concentrated in the Mixed In Key integration, the statistics trackers and the Beatport client. The recurring pattern is always the same: heavy or blocking work run under the global database lock, on the async runtime, with no tests.

**Mixed In Key and library**

| # | Defect | Where | Fix | Effort |
| --- | --- | --- | --- | --- |
| B1 | The sync re-triggers in bursts: 4 runs in 14 s observed at startup, each rewriting the 276 tracks without detecting any change | `lib.rs` (watcher), `mik_db.rs` | A single trigger (mtime + size), real debounce, comparison before writing, no sync on focus | M |
| B2 | Matching by title and artist, then rewriting the `file_path` of another track | `mik_db.rs` (`sync_all_from_mik_db`) | Match by NFC-normalised path, then by hash; never rewrite a path without confirmation | S |
| B3 | MIK cues are recreated with new UUIDs on every sync, with no tombstones; user cues are erased | `mik_db.rs` | Deterministic identifier (hash of track + position + index) and upsert; only touch cues that originate from MIK | S |
| B4 | `get_track_cues` scans the whole MIK database and resolves every bookmark on each playback, under the global lock | `commands/library.rs`, `mik_db.rs` | Read the cues from the Crate database (already synchronised) | S |
| B5 | `prune_missing_tracks` deletes at startup the tracks of a folder that was merely renamed | `services/library` | Mark as "missing" (upstream already does this) instead of deleting | XS |
| B6 | macOS bookmarks resolved without `WithoutMounting` or `WithoutUI`, `CFError` leak | `services/library/macos_bookmark.rs` | Add the resolution options, release the CF objects, remove the dead `create_bookmark` | S |
| B7 | `file_hash` is no longer recorded on import, and re-importing an existing file fails on a foreign key constraint | `services/library/import.rs` | Restore the hash write; insert cues after resolving the final identifier | S |
| B8 | Serato Markers2 parser off by one byte (index, position, name) | `services/library` (Serato parser) | Fix the offset and add a test on an anonymised real file | S |
| B9 | The new `energy` column is neither serialised nor merged by the cloud sync | `services/cloud_sync` | Add the column to the rows and to the merge | XS |
| B10 | Migration numbering diverges from upstream (15 entries, labelled 7 to 16, 6 skipped) | `db/schema.rs:385-534` | Renumber now, before any database depends on the current order; one comment per migration | S |

**Statistics (Crate Pulse)**

| # | Defect | Where | Fix | Effort |
| --- | --- | --- | --- | --- |
| B11 | Mixed In Key tracker: "file open in `lsof`" is counted as a listen; minutes capped at around 30 s | `services/stats/mik.rs:56-101` | Only count when MIK is actually playing (player state), or disable by default | M |
| B12 | Local tracker: pauses not handled, wall-clock time including pauses, a seek counted as a listen | `services/player.rs` | Accumulate only the intervals spent playing; re-arm after resuming | S |
| B13 | Rekordbox import: timestamp = time of the import, full re-import on every sync, `master.db` unreadable without the key | `services/stats/rekordbox.rs` | Read the session date from the XML, deduplicate by session, document the `master.db` limitation | S |
| B14 | Stats queries: comparisons between heterogeneous date strings, a heatmap that fails on a single invalid row, aggregation in Rust over the whole table | `services/stats/recorder.rs` | Store `played_at` as UTC epoch milliseconds, aggregate in SQL, tolerate invalid rows | M |
| B15 | Spotify JSON import: whole file passed as a string over IPC, no transaction, deduplication by full scan; panic on a malformed date | `services/stats/spotify.rs` | Pass the path, parse as a stream, single transaction, UNIQUE constraint; parse the date without byte slicing | S |
| B16 | Lock order inversion between `spotify_disconnect` and the poller: possible deadlock that freezes the whole database | `services/stats/spotify.rs` | A single documented locking order, or a single lock | S |
| B17 | Spotify token refresh: silent failure, stale token returned, race between the poller and the commands | `services/stats/spotify.rs` | Centralised refresh behind an async mutex, error surfaced to the UI | S |
| B18 | Permanent OAuth server on 127.0.0.1:8888, `state` not enforced, parameters reflected in the response HTML | `services/stats/spotify.rs:1070-1076` | Start the server only during a sign-in, with a timeout; require `state`; escape the response | S |

**Beatport and upgrader** (in addition to C2, C6, C7, C8)

| # | Defect | Where | Fix | Effort |
| --- | --- | --- | --- | --- |
| B19 | Beatport tokens in plain text in `~/.config/crate/beatport_auth.json`, copied into the `beatportdl` folder and into `localStorage` | `services/beatport/client.rs:358-416` | macOS Keychain or encrypted database, a single source of truth | S |
| B20 | Retrieval of the token that DJ.Studio stores in its local configuration | `client.rs:281-321` | Remove | XS |
| B21 | `validate_token` validates nothing and persists a fictitious "authenticated" state | `client.rs` | A real API call (profile) or removal | XS |
| B22 | Insufficient FLAC validation: a truncated file larger than 3 MB passes | `downloader.rs:20-45` | Decode the whole file with symphonia and compare its duration with the expected one | S |
| B23 | Scoring: a radio edit can replace an extended mix; fictitious Beatport default values (240,000 ms, 2026-01-01) fed into the score | `upgrader.rs` | Penalise duration and mix mismatches; never score a missing value | S |
| B24 | Artist splitting by regex without word boundaries: "Daft Punk" becomes "Da"; panic from byte indexing on lower-cased text | `upgrader.rs` (`split_artists`) | Word boundaries, separators surrounded by spaces, indices computed on the same string | S |
| B25 | No negative cache and no handling of 429s: every opening restarts a full network scan | `upgrader.rs` (`find_upgrade_matches`) | Cache "no result" outcomes with expiry, backoff on 429 | S |
| B26 | `execute_upgrade_replacements`: unsafe ordering, ignored errors, several minutes with no progress or cancellation, `file_path` supplied by the webview | `commands/upgrader.rs:48-64` | Re-read the path from the database, transactional steps, progress events and cancellation | M |

**Audio, search and export** (in addition to C12)

| # | Defect | Where | Fix | Effort |
| --- | --- | --- | --- | --- |
| B27 | No code writes `waveform_data`: the waveform displayed is always the fake 64-bar pattern | `services/analysis.rs`, `PlayerView.svelte:138` | Generate the peaks during analysis (symphonia) or read MIK's `ZWAVEFORM` read-only; hide the waveform when absent | M |
| B28 | FTS5: sanitisation makes tracks with an apostrophe, hyphen or slash unfindable, and upper-case `AND`, `OR`, `NOT` make `get_tracks` fail | `services/library/query.rs` | Wrap each token in double quotes with a `*` suffix; fall back to `LIKE` on error; tests on "You'll", "Jay-Z", "AC/DC" | S |
| B29 | Playback position = wall clock × speed, stale at the end of a track (no CoreAudio clock, contrary to what the summary claimed) | `services/audio/mod.rs:347` | Use rodio's `Sink::get_pos()` | S |
| B30 | File associations with `rank: Default` and the `public.audio` umbrella; bundle `com.crate.app` does not exist in the native code | `tauri.conf.json:38-61`, `commands/standalone.rs:101-150` | `rank: Alternate`, remove `public.audio`, delete the FFI and the Swift script | XS |
| B31 | `StartupFile`: race at startup, several opened files overwrite each other | `lib.rs` | Queue of paths, drained when the frontend is ready | S |
| B32 | `delete_tracks_and_files`: Trash via `osascript` per file, silent failures, permanent deletion outside macOS | `services/library/update.rs` | `trash` crate, errors surfaced | S |
| B33 | Blocking I/O, subprocesses (`beatportdl`, `lsof`, `osascript`) and rusqlite under `std::sync::Mutex` run on the tokio runtime | several services | `spawn_blocking` (not synchronous Tauri commands: in Tauri 2 a sync command runs on the main thread); never hold the lock during disk or network I/O | M |
| B34 | Services instantiated twice: the state managed by Tauri is not the one used by the background tasks | `lib.rs:594-599` | A single shared `Arc` instance | XS |
| B35 | `get_duplicate_count` launches a full duplicate scan on every `duplicates-updated` event | `services/duplicate.rs` | Cached counter, invalidated by mutations | S |
| B36 | Playback silently stops at the end of a library track: the end callback fires only from the client interpolation, so when a backend sync tick lands after the engine finished (`is_playing: false`, position clamped at the duration) the end is never signalled (found after the audit, CRA-148) | `shared/stores/player.ts` | Treat "engine not playing with the position at the duration while the store believed it was playing" as the end of the track; fire the end callback exactly once per track end (also across the HTML preview events) | XS |

**Backend minor defects and smells** (about 50, grouped): Beatport network errors turned into empty lists; French error messages hard-coded in `CrateError`; tables without foreign keys and with unbounded growth (`listen_events`, `upgrade_matches_cache`, `pkce_verifier_*`); background loops with no cancellation or backoff; eight copies of the `Track` mapper; lofty/symphonia readers duplicated four times; `Regex::new` recompiled on every call; base64 reimplemented; 330 lines of HTML duplicated in the OAuth handler; personal paths in the code (`downloader.rs:76`, `mik_db.rs:1005`). Each is XS to S and is dealt with along the way in phase 3.

## Svelte frontend

The frontend compiles without type errors, but 15 major defects affect shortcuts, initialisation, performance and the honesty of the interface. Most of them come from the same habit: effects and timers added without thinking about their clean-up or about the other views.

| # | Defect | Where | Fix | Effort |
| --- | --- | --- | --- | --- |
| F1 | Keys 1 to 8 captured everywhere, with no view or modifier condition | `hooks/useKeyboardShortcuts.ts` | Active only in the Player view and outside input fields, or behind a modifier | XS |
| F2 | Space in the Player view starts a recent track instead of pausing the preview; duplicated logic | `useKeyboardShortcuts.ts`, `PlayerView.svelte` | A single "play/pause" handler that acts on the active source | S |
| F3 | Space or Enter on a focused row triggers both playback and the global shortcut | `TrackRow.svelte`, shortcuts | `stopPropagation` on the row, or ignore shortcuts when a row has focus | XS |
| F4 | `withTimeout`: an initialisation longer than 2.5 s loses the clean-up functions (Tauri listeners never detached) and hides errors | `hooks/useAppSetup.ts:752-798` | Remove the timeout, or keep the promise and clean up when it resolves | S |
| F5 | Splash closed by a module-level timer at 1.5 s: race with settings loading, possible flash of the onboarding | `stores/splash.ts:15` | Close the splash once the settings are loaded | XS |
| F6 | Full Mixed In Key sync and reload of the whole library on every window focus | `routes/+layout.svelte:341-347` | Remove; the backend watcher is enough | XS |
| F7 | The Toolbar effect triggers a `get_duplicate_count` call on every library mutation | `layout/Toolbar.svelte:55-60` | Subscribe to the backend event only | XS |
| F8 | Beatport: favourites and playlists "created" only locally with a misleading success toast; cart emptied even if downloads fail | `shared/stores/beatport.ts` | Call the real API or hide these actions; only remove successful items from the cart | S |
| F9 | Side effects on module import in `shared/` (3-minute Beatport timer, splash timer) and desktop dependencies in shared code | `shared/stores/beatport.ts`, `splash.ts` | Explicit initialisation from `useAppSetup` | S |
| F10 | Mutation of a `$derived` value (`activeHeroTrack.is_in_library = true`) | `PlayerView.svelte` | Update the source store | XS |
| F11 | Cues and waveform never loaded for an external file: a UUID is sent to `get_track_cues` | `shared/stores/player.ts` | Load by path for files outside the library | S |
| F12 | Race with stale responses in position tracking (async interval calling `getPlaybackState` every second) | `shared/stores/player.ts` | Request number and discarding of older responses, or a position event pushed by the backend | S |
| F13 | "Synchroniser avec Mixed In Key" (Sync with Mixed In Key) in the context menu ignores the selection and re-syncs the whole database | `library/trackContextMenuItems.ts` | Pass the selected identifiers | XS |
| F14 | Upgrader: confidence is shown as "0.96%" instead of "96%"; the replace button stays active when Beatport is not connected | `components/upgrader/` | Multiply by 100 with `Intl.NumberFormat`; disable when not connected | XS |
| F15 | The Mixed In Key badge tooltip stays visible after the pointer leaves | `layout/Toolbar.svelte` | Use the common `Tooltip` component | XS |
| F16 | Arrow keys, Home and End on a focused segmented control (view switcher, Player toggles, Pulse period) also seek or change the volume through the global shortcuts; the arrows of an open select change the volume too (found after the audit, CRA-196) | `common/SegmentedControl.svelte`, `common/Select.svelte`, `hooks/useKeyboardShortcuts.ts` | Stop the keys the control handles; the global arrow shortcuts ignore a key a focused widget already used (`defaultPrevented`) | XS |

**Frontend minor defects and smells** (about 25): `{#each}` without keys on mutable lists; native `window.confirm()` to delete an album; file size column always "-"; two display preference stores that are not synchronised; `formatBitrate` that assumes 24-bit PCM; continuous playback in standalone mode that moves on through the history; 16 explicit `any`; around thirty dead exports; a forgotten `console.log`; giant components (`PlayerView.svelte` is 938 lines long); logic copied between `PlayerView` and `Player`.

## Backend ↔ frontend consistency

The wiring is the healthiest part of the fork: no missing command, no orphan call, no argument mismatch. The defects lie in the error contract, in settings that have no effect and in the lack of progress reporting for long operations.

| Measure | Value |
| --- | --- |
| `#[tauri::command]` commands | 230 names, all registered |
| Names invoked from the frontend | 226, all resolved |
| New commands in the fork | 75 |
| Commands never invoked | 4 (2 new duplicates, 2 upstream) |
| Argument mismatches | 0 |
| Events emitted without a listener | 1 (`library-updated`) |

| # | Defect | Where | Fix | Effort |
| --- | --- | --- | --- | --- |
| I1 | Backend error messages lost: Tauri rejects with a string, and the new stores test `instanceof Error`, which is always false; the Upgrader's authentication heuristic can never detect the error | `shared/stores/{upgrader,duplicate,stats,player,albums}.ts` | Shared `toErrorMessage(e)` helper; eventually, serialise `CrateError` as `{ code, message }` and translate the code on the TS side | S |
| I2 | Two Beatport settings with no effect: the downloader forces `lossless` and always runs the Mixed In Key sync | `services/beatport/downloader.rs:110,284` | Read `AppSettings` in the command and pass quality and sync through | S |
| I3 | No progress events for the Beatport download and the upgrader: the UI stays frozen for several minutes | `commands/beatport.rs:122-160`, `commands/upgrader.rs` | Progress `tauri::ipc::Channel`; the TS types already exist but are unused | M |
| I4 | `BeatportAuthState` carries two conventions (camelCase on the TS side, snake\_case on the Rust side); consistency relies on 6 sites in the store that duplicate both | `shared/types/beatport.ts:89-100`, `client.rs:82-90` | A single snake\_case schema aligned with Rust | S |
| I5 | `record_listen_event` accepts a complete event from the webview but is called nowhere | `commands/stats.rs:75` | Remove | XS |
| I6 | Duplicate `spotify_set_client_id` and `spotify_set_client_secret` commands; the Spotify secret can be read back by the webview | `commands/stats.rs:95-131` | Remove the duplicates and the command that reads the secret back | XS |
| I7 | `library-updated` emitted without a listener; `searchType` parameter of the Beatport search ignored | `commands/upgrader.rs:62`, `commands/beatport.rs:92-97` | Listen to the event in the layout or remove it; implement or remove the parameter | XS |
| I8 | About twenty Rust `Option<T>` fields typed `?: T` on the TS side although the actual value is `null` | `shared/types/beatport.ts`, `album.ts`, `index.ts` | Type them `T \| null` | XS |
| I9 | Display options stored only in `localStorage`: neither backed up nor synchronised | `shared/stores/displaySettings.ts:34` | Persist them in the `settings` table like the other preferences | S |

## Design, responsive and accessibility

Upstream has a real design system that the fork almost entirely ignores: three visual languages coexist (upstream's accent-driven one, the cyan and amber of the Player and Pulse, Beatport's neon green), and no new view follows the accent colour chosen in the settings. Bringing it into compliance is estimated at 10 to 14 days.

| Indicator | Upstream | New code |
| --- | --- | --- |
| Tailwind palette classes instead of tokens | 88 | 584 |
| Hexadecimal colours in components | 143 (mostly in `style.css`) | 299 |
| Arbitrary font sizes `text-[8–12px]` | 20 | 195 |
| Radii from `rounded-xl` to `3xl` | 1 | 88 |
| `backdrop-blur` | 1 | 26 |
| `dark:` variants | 0 | 43 |
| Ad hoc buttons versus the `Button` component | — | 102 versus 8 |

| # | Defect | Where | Fix | Effort |
| --- | --- | --- | --- | --- |
| D1 | `dark:` follows the OS theme, not the app theme: build 40's "light-mode fixes" are wrong as soon as the OS and the app differ | 43 occurrences | Remove `dark:` and use the tokens, or declare `@custom-variant dark` on `[data-theme='dark']` | S |
| D2 | About 70 classes reference tokens that do not exist (`surface-3`, `surface-4`, `stroke-strong`, `text-disabled`, `brand-primary/NN`): loading skeletons and progress tracks invisible | new components | Create the missing tokens or replace them with existing ones | S |
| D3 | Broken light theme: Pulse integration cards dark at the top and light at the bottom; dark header badges; contrast measured from 1.04:1 to 2.78:1 on the Player tab, the badges, the heatmap and the energy badge | `components/stats/`, `Toolbar.svelte`, `EnergyBadge.svelte` | Tokens only; target 4.5:1 for text and 3:1 for interface elements | M |
| D4 | Header at the minimum width (1000 px): the icons overlap the Player/Library/Beatport segmented control, a click on "duplicates" lands on Beatport; even at 1400 px, "Beatport" is truncated in the Discover view | `layout/Toolbar.svelte` | Group the tools into a "Tools" menu, segmented control as `flex-shrink-0`, MIK badge reduced to a dot | S |
| D5 | Player hero with a fixed height (`h-[225px]` for about 260 px of content): at 1000×640 the transport is covered and only 2 rows of recent tracks remain; at 1920×1080 large empty areas | `player/PlayerView.svelte` | Intrinsic height, artwork with `clamp()`, recent tracks list as `flex-1 min-h-0` | M |
| D6 | Modals taller or wider than the window: the Duplicate Killer footer and its "Supprimer" (Delete) button are cut off at 1400×900; the `4xl` modals (1152 px) exceed the minimum width | `components/duplicates/`, `upgrader/`, `common/Modal.svelte` | `max-h-[calc(100vh-32px)]` with internal scrolling and a fixed footer; width `min(1152px, 100vw - 32px)` | S |
| D7 | Beatport table at 1000 px: the title column shrinks to a single character | `components/beatport/` | `minmax(0, …)` grid with a minimum width on the title, secondary columns hidden | S |
| D8 | Jost and DM Sans fonts offered in the settings but not wired (`[data-font='jost']` missing); 15 of 20 files unused (896 KB); OFL licences missing; double loading via Google Fonts | `style.css`, `static/fonts/` | Wire up or remove the option, keep 5 files, add the OFL, remove the Google import | XS |
| D9 | Six icons in use do not exist in `Icon.svelte`; the Beatport SVG logo loaded as an `<img>` with `currentColor` comes out black on a dark background | `common/Icon.svelte`, `components/beatport/` | Add the icons and type the name (union) so that `svelte-check` fails on an unknown name; inline logo | S |
| D10 | Accessibility: about 25 icon buttons without a name (transport, segments, MIK badge, recent track actions); clickable rows as `div` hidden behind `svelte-ignore a11y`; mouse-only waveform; heatmap not focusable; 3 overlays without a focus trap; `ToggleSwitch` nests two controls; 19 infinite animations without `motion-reduce`; upstream removes the focus outline globally | many | `aria-label` everywhere, `<button>` for every clickable element, arrow keys on the waveform, common `Modal`, `focus-visible:ring` | M |
| D11 | Components reinvented instead of the common ones: 4 segmented controls, home-made checkbox, select, spinner, tooltip and confirm; Camelot badge copied 8 times | new components | Extract `SegmentedControl`, `KeyBadge`, `EnergyBadge` and use `Button`, `Checkbox`, `Select`, `Tooltip`, `Spinner` | M |
| D12 | Permanent "Build 57" badge next to the logo and "PRO" labels on third-party brands | `Toolbar.svelte`, `+layout.svelte` | Show the version in "About" only | XS |
| D13 | Light theme, after D3: state colours used as text (`text-danger`, `text-warning`, `text-success`, `text-info`, about 40 places) keep one value for both themes; `ToggleSwitch` sky fill at 2.5 to 2.7:1; cloud-sync "offline" amber (`text-amber-500`) at about 2:1 | `style.css` state tokens, `ToggleSwitch.svelte`, `CloudSyncTab.svelte`, `SyncStatusIndicator.svelte` | Light values for the state tokens (as D3 did for the family tokens); switch fill decided with the owner (sky or accent, D11 family scope) | S |

**Proposed strict design rules**, derived from upstream's conventions:

1. Colours through tokens only, no palette class or hex in a component.
2. Every new semantic colour becomes a token declared for both themes, with verified contrast; three at most, for cue, live and brand.
3. Never `dark:`: the theme is `[data-theme]`, not the OS.
4. Typography through the `Text` component: no `text-[Npx]`, minimum body size of 12 px for a data value.
5. Radii `rounded` or `rounded-md` for controls, `rounded-lg` for cards and modals, `rounded-full` for pills.
6. Elevation limited to `shadow-sm`, `shadow-lg`, `shadow-xl`; no gradient, glow or backdrop blur.
7. Mandatory common components: `Button`, `IconButton`, `Modal`, `Checkbox`, `Select`, `Tooltip`, `Spinner`, `Icon`.
8. Every clickable element is a button with an accessible name; no `svelte-ignore a11y`.
9. Zero hard-coded strings: the translation key is added in the same change.
10. No fixed pixel height on a content area; every view and every modal usable at 1000×600.
11. Transitions of 150 to 200 ms on colour, opacity or transform; infinite animations reserved for spinners, with `motion-reduce`.
12. Definition of done: light and dark screenshots, two accents, two window sizes, and a CI scan that rejects palette classes, hex, `dark:` and arbitrary sizes.

## Internationalisation

All the new views are written in hard-coded French: with the app in English, the library is translated but Crate Pulse, the Upgrader, the Duplicate Killer, the Player and the Beatport tab stay in French. The Beatport view even mixes both languages ("Purchased tracks" next to "Ajouter tout au panier").

| Area | Translation calls | Hard-coded strings found |
| --- | --- | --- |
| `components/stats/` | 0 | 64 |
| `player/PlayerView.svelte` | 0 | 44 |
| `components/beatport/` | 0 | 27 |
| `components/upgrader/` | 0 | 26 |
| `settings/tabs/BeatportTab.svelte` | 0 | 25 |
| `components/duplicates/` | 0 | 13 |
| `settings/tabs/DisplayOptionsTab.svelte` | 30 | 0 |
| `shared/` stores (toasts) | 0 | 18 |
| Rust errors (`CrateError`, Beatport client) | — | hard-coded French messages |

| Locale | Missing keys out of 792 | Coverage |
| --- | --- | --- |
| en | 0 | 100% |
| fr | 1 | 99.9% |
| ja | 49 | 93.8% |
| de, es, it, ko, nl, pt, sv, zh | 50 | 93.7% |
| pl, ro, tr, uk | 53 | 93.3% |

| # | Defect | Fix | Effort |
| --- | --- | --- | --- |
| L1 | 17 new components with no translation at all, about 220 strings | Extract the strings into `stats.*`, `player.*`, `beatport.*`, `upgrader.*`, `duplicates.*` keys in `en.json` and `fr.json` | M |
| L2 | 13 locales missing the 49 to 53 keys added by the fork | Decide: translate, or keep svelte-i18n's English fallback and document it | S |
| L3 | Numbers and units formatted English-style in French ("2,310"), "plays" and "écoutes" mixed in Pulse | `Intl.NumberFormat` with the active locale, ICU plurals | XS |
| L4 | Rust error messages in French | English error codes translated on the frontend side (see I1) | S |
| L5 | `register('en')` and `register('fr')` kept after `addMessages`: initialisation goes back through the asynchronous path | Remove the two redundant `register` calls | XS |
| L6 | The README advertises 11 languages whereas the repository contains 15 | Update the README | XS |

## Quality, tests, tooling and CI

If the tree were committed as is, 5 of the 8 upstream CI jobs would fail, and no workflow runs the tests. For personal use on this single Mac, the Windows and mobile builds matter little; the release configuration and the tests, on the other hand, protect your data.

| # | Defect | Where | Fix | Effort |
| --- | --- | --- | --- | --- |
| Q1 | `tauri.prod.conf.json` modified for a local build (`targets: ["app"]`, update artefacts disabled): a release would publish a `latest.json` with an empty signature and break auto-update | `src-tauri/tauri.prod.conf.json:5-6` | Restore; local build with `--config '{"bundle":{"targets":["app"],"createUpdaterArtifacts":false}}'` | XS |
| Q2 | **Cancelled 2026-09-30 (CRA-76): Crate targets macOS only.** iOS/Android build broken: `services::beatport`, `services::stats` and two library commands are not gated by the `desktop` feature | `lib.rs:193-194`, `services/mod.rs`, `commands/mod.rs` | `#[cfg(feature = "desktop")]` on modules, registrations and `.manage()` | S |
| Q3 | Windows/Linux build broken: `RunEvent::Opened` only exists on macOS, iOS and Android | `lib.rs` | `#[cfg(any(target_os = "macos", target_os = "ios"))]` around the arm; read `argv` elsewhere | XS |
| Q4 | Clippy with `-D warnings`: 37 errors (16 of dead code); cargo fmt: 35 files | Rust | `cargo fmt`, removal of dead code, clippy fixes | S |
| Q5 | ESLint: 38 errors; Prettier: 60 files | TypeScript and Svelte | `yarn format:fix && yarn lint:fix`, then manual fixing of keyless `each` blocks and `any` | S |
| Q6 | No workflow runs `yarn test` or `cargo test` | `.github/workflows/` | Add a test job (Vitest and cargo test on a temporary database) | XS |
| Q7 | No tests on the risky paths: file replacement, purge, pollers, OAuth; 3 tests read the real MIK database and silently pass if it is absent | `services/library/mik_db.rs:982-1010` | Integration tests on `tempdir`; anonymised MIK fixtures | M |
| Q8 | Vitest runs on Vite 8.2 whereas the app uses Vite 7.3; `test:coverage` without a coverage provider; test dependencies with a caret while everything else is pinned | `package.json`, `yarn.lock` | Pin vitest to a Vite 7-compatible version; add `@vitest/coverage-v8` | XS |
| Q9 | dev, staging and prod icons now byte-for-byte identical; `.ico`, Windows, iOS and Android icons not regenerated | `src-tauri/icons/` | `yarn tauri icon` per environment, with a visible variant for dev | S |
| Q10 | CHANGELOG, version, README and documentation site not updated; the shortcuts documentation contradicts the new Shift+arrow behaviour | `CHANGELOG.md`, `README.md`, `docs/` | `[Unreleased]` entries per feature; version 0.3.0; docs pages updated | S |
| Q11 | Files that must not be committed: `SYNTHESE_DISCUSSION.md` (now `tracking/history/DISCUSSION-SUMMARY.md`; personal paths), `scripts/set_default_player.swift`, unused fonts, third-party brand logos | root, `static/` | Exclude or move; `.gitignore` | XS |
| Q12 | No `CLAUDE.md`: every assistant rediscovers the rules (`desktop` feature, CI clippy flags, i18n, never real databases in tests) | root | Create a 15-line `CLAUDE.md` from the rules in this register | XS |
| Q13 | `yarn dev` compiles the Rust in `--release`: every change costs several minutes (upstream) | `package.json:11` | Debug profile for development | XS |
| Q15 | _Added after the audit (30 September)._ No design guardrails for assistants: the "strict design rules" exist only in this register, each session reinvents its own style (the origin of the three visual languages of builds 40 to 57) and nothing measures the drift | `.claude/`, root | Crate's `DESIGN.md`, a `design` agent with dedicated skills (system, audit, build, image to code, visual check), `yarn design:scan` scanner | S |
| Q14 | 87 `yarn audit` alerts (61 high) in transitive tooling; `cargo audit` not installed locally | dependencies | Targeted `yarn upgrade`; install `cargo-audit` | S |

## Recommended repair order

Repair in this order: each step makes the next one safer, and each row corresponds to one or two verifiable commits. Steps 1 to 9 make up scenario A of the main report.

| Step | Content | Defects addressed | Effort | Verification |
| --- | --- | --- | --- | --- |
| 1 | Change the Beatport password; back up `crate.db`, `db.key` and `Collection11.mikdb`; local branch and snapshot commit of the current work | C1, C2 (rotation) | 1 h | `git log` shows the commit; backups present |
| 2 | Stop the destructive operations: purge at startup, MIK writes, sync on focus, tests on real databases; restore the prod config | C3, C4, C5, B5, F6, Q1 | 2 h | Restart the app: no "prune" in the logs; `cargo test` no longer touches `~/Library` |
| 3 | Remove the credentials from the code, Beatport tokens and Spotify secret in the Keychain, remove the DJ.Studio retrieval | C2, B19, B20, I6 | 0.5 d | `grep` returns nothing; gitleaks in pre-commit |
| 4 | Safe upgrader: temporary folder, in-place replacement, validation by decoding, progress and cancellation (on a private branch) | C6, C7, C8, B22, B26, I3 | 2 to 3 d | Tests on `tempdir`: cues and tags kept, no neighbouring file touched |
| 5 | Clean Mixed In Key: read-only, single incremental sync, deterministic cues, matching by path | B1, B2, B3, B4, B6 | 1.5 d | Startup log: a single sync, "0 updated" on the second launch |
| 6 | Accurate statistics: remove the repair, Spotify deduplication, pauses, MIK tracker, epoch dates, locks | C9, C10, B11 to B18 | 2 d | Test: one Spotify listen produces only one row; a pause does not count |
| 7 | Exact DJ features: zero-based hot cues, real waveform, FTS5, XML export, audio position, three regressions, two audio engines | C11 to C15, B27 to B29, F1 to F3, F11 | 3 d | Key 3 = cue 3; search for "You'll" finds it; XML with `&` opens in Rekordbox |
| 8 | Robust frontend: `withTimeout`, splash, costly effects, IPC errors, Beatport settings taking effect | F4 to F10, F12 to F15, I1, I2, I4 | 1.5 d | No IPC call per keystroke; backend error messages visible |
| 9 | Hygiene: fmt, lint, dead code, `desktop` feature, Windows, tests in CI, icons, fonts, CHANGELOG, `CLAUDE.md`, splitting into thematic commits | Q2 to Q14, B10 | 2 to 3 d | Green CI; installation of a personal 0.3.0 |
| 10 | Visual foundations: missing tokens, `dark:`, fonts, icons, visible focus, modal template | D1, D2, D6, D8, D9 | 1 d | Light and dark screenshots with no invisible element |
| 11 | View-by-view compliance: Player, Pulse, Beatport, Upgrader, Duplicates, header | D3, D4, D5, D7, D10, D11, D12 | 8 to 12 d | Definition of done from the design rules |
| 12 | Full French and English translation, documented fallback for the other languages | L1 to L6 | 2 to 3 d | App in English: no French string |

The minor defects and smells (about 90) are dealt with along the way, in the step that touches the same file. The browser harness built for this audit (fake Tauri backend, 16 tracks, theme and language parameters) can serve as the basis for the end-to-end tests of steps 10 to 12.
