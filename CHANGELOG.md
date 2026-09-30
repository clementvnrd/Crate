# Changelog

All notable changes to Crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Personal fork — change log

> Log kept live since the audit of 25 September 2026. Each entry cites the identifier of the fixed defect (see [tracking/DEFECTS.md](tracking/DEFECTS.md)); overall progress is in [tracking/STATUS.md](tracking/STATUS.md). The most recent entries are at the top of each section.

#### Process

- **2026-09-30 — Owner decisions read from Linear (second pass).** The Beatport FLAC download stays as it is while `origin` is private (CRA-113). The colour families of Player, Pulse, Beatport and the original accent-driven look are all kept, and the missing piece is a precise graphic charter saying where each applies, tracked as CRA-140 and blocking D3, D7, D10 and D11 (CRA-115). Recorded in `CLAUDE.md` §2. For the Spotify history (CRA-123) the owner asked to empty it: `crate.db` was backed up and the backup verified (`quick_check` ok, 1,560 Spotify listens out of 1,715), but the irreversible deletion was refused by the session's permission layer and is left to the owner's go.
- **2026-09-30 — Owner decisions read from Linear.** Scenario "A then B" (CRA-112), interface languages French and English complete with the other 13 locales falling back to English (CRA-114), macOS only (CRA-122), Mixed In Key stays read-only (CRA-110). Recorded in `CLAUDE.md` §2. Still open: Spotify history cleanup (CRA-123, needs a safe procedure), default audio player (CRA-124, follow-up issue), visual language (CRA-115) and Beatport download (CRA-113), both awaiting a simpler explanation.
- **2026-09-30 — Answers in Linear and hourly watch.** The owner answers decisions and questions by commenting on the Linear issue or closing it; assistants check Linear for that feedback at the start of every reply. Because the Linear connection is signed in as the owner, every assistant comment starts with `🤖 Claude:` so the two can be told apart. A scheduled task, `crate-linear-hourly-check`, repeats the check every hour and does nothing when nothing is new; at the owner's choice it may also implement one register defect per run, only on a clean `develop`, ending at In Review. Statuses now say who has the ball: Todo waits for the owner, In Progress is the assistant's turn, In Review waits for the owner's check. Rules are in `CLAUDE.md` §1.
- **2026-09-30 — Two new Linear statuses: Needs input and On Hold.** `In Review` no longer means "back to the owner": it is now only for pushed code to verify. `Needs input` (between Todo and In Progress) marks an issue waiting for the owner, so the progress ring steps back when the assistant returns a question; `On Hold` marks work the owner keeps aside on purpose, which assistants never touch. The owner creates both in Linear (the MCP server cannot); until then the rules fall back to In Review and Backlog.
- **2026-09-30 — Status rules made mandatory for the hourly watch.** The first run answered and recorded decisions but left every issue In Progress and missed one answer because it only looked at recent updates. The task now scans all open issues with no time window, must change the status in the same step as every comment and verify it, and follows the rule: question answered → In Review; decision recorded → Done; real work left → its own issue.
- **2026-09-30 — Linear adopted as the live board.** The owner follows the project in Linear (workspace "HomeMade", team Crate App, key `CRA`). The board was populated from `tracking/STATUS.md`: project *Consolidation 0.3.0* (one milestone per repair step, one issue per register defect titled `[B12] …`, 94 of 107 already Done), project *Polish & Unify 0.4* (scenario B ideas and open decisions) and project *Repository & workflow* (history of the repository set-up). `CLAUDE.md` §1 gained the rules every session must follow: one issue per piece of work before starting it, statuses that follow the work, `Refs CRA-n` in commit messages, and a Linear report at the end of each answer. The repository files stay the versioned record; which of the two is authoritative long term is an open question (`CLAUDE.md` §9).
- **2026-09-30 — New way of working.** Everything written in the repository is now in English: the tracking system moved from `suivi/` to `tracking/` (`AVANCEMENT.md` → `STATUS.md`, `REGISTRE-DEFAUTS.md` → `DEFECTS.md`, `RAPPORT-AUDIT.md` → `AUDIT-REPORT.md`, `yarn suivi` → `yarn status`), every earlier document and change-log entry was translated, and commit messages are written in English from now on. `CLAUDE.md` gained standing rules: capture instructions as they are given, reply structure, and a hygiene check after every major task.

#### Added — Polish & Unify 0.4 (scenario B)

- **Export of the whole listening history (CRA-128), backend.** New `export_listening_history` command writes every listen, oldest first, to a CSV or JSON file: an independent copy of the history, readable in a spreadsheet or a script, that depends neither on the database nor on its key. The CSV follows RFC 4180 (commas, quotes, line breaks and accents round-trip); the JSON keeps numbers as numbers, missing values as `null`, and the stored metadata as real JSON when it parses. It reads the table in chunks of 2,000 rows so the database lock is never held while the file is written, writes to a `.part` file renamed at the end (a failure never leaves a half export nor damages an existing one), and refuses a destination whose extension does not match the format. Six tests. _The button in Crate Pulse comes with the Pulse interface work._
- **Smart playlists on listening statistics (CRA-131), backend.** The library smart-playlist rules gain listening criteria, usable as conditions and as sort field: `listens_total`, `listens_7d`, `listens_30d`, `listens_365d`, `set_plays` (times played in a Rekordbox set) and `minutes_listened`, plus the dates `last_listened` and `last_set_play`. This makes "never played in a set" (`set_plays = 0`), "top 30 days" (sort by `listens_30d`, limit 30) and "not heard for 90 days" one-line rules. Only Crate-local listens carry a track id; Rekordbox sets and the Spotify history carry just artist and title, so listens are matched to library tracks by those two names, case and surrounding spaces ignored, backed by an expression index (migration 16). Dates go through `datetime()` so the `Z`, offset and bare forms compare correctly ([B14]). Six tests. _The rule editor fields come with the library interface work._
- **"Your week" and "Your year" recap (CRA-125), backend.** New `get_recap` command assembles the statistics Crate Pulse already has over a **calendar** week (Monday to Sunday) or year, in local time, or over an earlier one (`offset`): total plays and minutes compared with the previous period, tracks and artists heard, tracks heard for the first time, top 5 tracks and artists, top 3 keys, the busiest day, the peak hour, the peak weekday and the minutes per source (Spotify, Crate, Mixed In Key, Rekordbox sets). To make it possible the existing statistics accept an exact window, `between:<start>,<end>` (UTC, start included, end excluded); the bounds are parsed and rewritten, so nothing the caller sends reaches the SQL, and a malformed window matches nothing instead of widening to "all time". Eleven tests. _The Pulse screen and the shareable image come with the Pulse interface work; the first recap will be more reliable once the Spotify history question (CRA-123) is settled._
- **Next-track suggestion from your real transitions (CRA-133), backend.** New `suggest_next_tracks` command proposes what to play after a track. First come the library tracks that really followed it in your Rekordbox sets, most often first, then most recent; each set is read on its own, so the end of one set never links to the start of the next, and a follower the library does not have is skipped. If fewer than asked for (10 by default, 50 at most), the list is completed with library tracks that mix with it: the same key, then a neighbouring or relative key on the Camelot wheel, within a deck's pitch range (6 %), the closest tempo first; a clashing key or a tempo out of range is never proposed. Each suggestion says where it comes from, how many times it followed, when, how the keys relate and the tempo gap. A new `harmonic` module reads keys in any notation (`8A`, `Am`, `A minor`, `F#m`, `E♭ minor`…) and relates them on the wheel; names are compared case-insensitively with SQLite lowercasing both sides (its `lower()` is ASCII-only, so pre-lowercased Rust strings would never match an accented title). Fifteen tests. _The panel that shows the suggestions comes with the DJ-preparation interface work._
- **Discovery → library → played-in-a-set funnel (CRA-126), backend.** New `get_discovery_funnel` command measures what the releases you found (Bandcamp, SoundCloud, Beatport…) became: how many were discovered, how many reached the library, how many were played in a Rekordbox set, in total and by source, optionally only for releases added since a given day. Nothing records which library track a release turned into, so a release counts as in the library when a track by the same artist has its title (as the track or the album) or one of its track names, and as played when such a track appears in a set; names are compared ignoring case and surrounding spaces and only exact matches count ("Artist A & B" is not "Artist A"; an empty title never matches). Migration 17 adds the matching indexes on tracks and releases. Six tests. _The funnel screen comes with the Pulse interface work._
- **Timeline of a Rekordbox set (CRA-127), backend.** New `get_rekordbox_session_timeline` command returns the tracks of one set in order with their position, the key, tempo and energy of each, and how every transition from the previous track mixes (harmonic relation on the Camelot wheel and tempo change), plus the number of harmonic, clashing and unknown transitions. Key, tempo and energy come from the matching library track when there is one (Mixed In Key's analysis, one notation for the whole set), else from what Rekordbox recorded; Rekordbox never records an energy, so a track outside the library has none. Four tests. _The timeline screen, with the key flow and the energy curve, comes with the Pulse interface work._

#### Security

- **[C2]** Removed the Beatport username and password hard-coded in `services/beatport/downloader.rs` and `client.rs`, **before the first commit**: the secret never entered the git history. The `beatportdl` configuration now keeps the `username`/`password` lines already present in the user's file (otherwise `beatportdl` uses the OAuth tokens written at sign-in), and the file is written with `600` permissions. Two tests cover these cases. _The password itself still has to be changed by the owner._
- **[B19]** The Beatport session (OAuth tokens) is stored in the **macOS Keychain** instead of a plaintext JSON file (`~/.config/crate/beatport_auth.json`) and the webview's `localStorage`. The old file is migrated then deleted on first launch, and the old `localStorage` copy is erased. Only `beatportdl-credentials.json`, which `beatportdl` requires, stays on disk: `600` permissions, deleted on sign-out.
- **[B20]** Removed the retrieval of the Beatport token from DJ.Studio's local configuration (`beatport_auto_detect_session` command).
- **[B21]** A Beatport token pasted by hand is no longer accepted without verification: it must be validated by Beatport's account API.
- **[I6]** Removed the duplicate Spotify commands (`spotify_set_client_id`, `spotify_set_client_secret`); the Spotify client secret is never sent back to the webview any more (`spotify_has_client_secret` only says whether it exists; leaving the field empty keeps it).
- **[I5]** Removed the `record_listen_event` command, never called, which let the webview inject arbitrary listens into the statistics.
- **[C5]** Removed the `test_sync_and_deduplicate_real_db` and `test_prune_missing_tracks_from_mik_db_real` tests, which opened the real Crate database (with its key) and the real Mixed In Key database and wrote to them on every `cargo test`.

#### Fixed — data protection

- **[C3]** The Mixed In Key sync no longer deletes Crate tracks that are missing from Mixed In Key ("strict purge" removed): Mixed In Key enriches the library, it no longer decides its contents. An unmounted volume or an unanalysed track no longer makes tracks, tags or playlists disappear.
- **[C4]** Crate never opens `Collection11.mikdb` for writing any more: removed the cascading purges (`purge_tracks_by_pks`, `purge_tracks_by_path`, `prune_missing_tracks_from_mik_db`, `wal_checkpoint(TRUNCATE)`), including when a track is deleted and after a FLAC upgrade.
- **[B5]** Tracks whose file has disappeared are no longer deleted at startup (a renamed folder was enough to erase them). The command remains available manually.
- **[B2]** Re-linking a Mixed In Key track by title and artist only happens when there is exactly one match whose file has disappeared (moved file); it no longer deletes the other tracks with the same name (original version, extended…). Duplicates of the same file transfer their tags and playlists before being merged.
- **[F6]** Removed the full Mixed In Key sync and library reload every time the window regains focus: the backend sync (at startup + file watching) is enough.
- **[Q1]** `tauri.prod.conf.json` restored as upstream has it (updater artifacts); the personal unsigned build goes through `yarn build:local` and `tauri.local.conf.json`.
- Tests: three new sync tests on an in-memory database and temporary files (missing track kept, same-name tracks never merged, moved file keeps its identity); the cleanup test no longer uses a personal path.

#### Fixed — Beatport upgrader

- **[C6]** The upgrader and the Beatport cart no longer "clean" the destination folder: `beatportdl` downloads into a private `.crate-download-…` folder created for the occasion, only validated FLAC files come out of it, then that folder (and only that folder) is deleted. Artwork, `.m3u` playlists, notes and subfolders of your music folder are never touched any more.
- **[C7]** Only the files created by the current download are taken into account (they are the only ones in the private folder): a FLAC already present can no longer be mistaken for the new one, and an existing file is never overwritten (`Title (1).flac`).
- **[C8]** An MP3 → FLAC upgrade updates the **existing** track (same identifier) instead of deleting it then re-importing it: cues, tags, playlists, rating, colour, play count and listening history are kept. The MP3 only goes to the Trash once the library has been updated; a failure to move it to the Trash is reported.
- **[B22]** FLAC validation by full decoding (symphonia): a file that is corrupted, truncated (fewer samples than announced) or whose duration does not match the Beatport track (±5 s or ±3%) is rejected.
- **[B23]** Scoring: a different version (radio edit, dub, instrumental…) can no longer replace an extended/original mix; an unknown duration gives a neutral score instead of a false match.
- **[B24]** Artists are split on whole words: "Daft Punk" is no longer cut into "Da" (`ft` separator), "Alex" keeps its x; matching artists by inclusion now only counts whole words. Also fixes duplicate detection.
- **[B25]** Tracks with no Beatport equivalent are remembered for 7 days (no new network search on every opening); Beatport's 429 responses are retried after 1, 2 then 4 s; a network error is never remembered as "no result".
- **[B26]** The MP3 path is read back from the library instead of being supplied by the webview; all errors are reported.
- **[I3]** Upgrade progress (`upgrade-progress`) shown in the button: "Mise à niveau 2/5 — Titre" (Upgrading 2/5 — Title).
- **[F14]** The replace button is disabled as long as Beatport is not connected.
- Tests: 12 new tests (decoding a real test FLAC, fake and truncated files, moving without overwriting, user folder never deleted, in-place replacement that keeps cues/tags/playlists, artist splitting, scoring).

#### Fixed — Mixed In Key and library

- **[B1]** The Mixed In Key sync no longer re-runs in bursts: the watcher ignores the `-shm` file (modified by any read, including Crate's) and compares the date + size of the database and the WAL. Each pass now only rewrites the tracks that actually change (comparison clause in the `UPDATE`), in a single transaction, and the artwork search now only covers added or modified tracks. Verified on a copy of the real library: 276 tracks rewritten on every pass before, 0 on the second pass now (~250 ms).
- **[B1]** Mixed In Key no longer overwrites the title, artist, album, genre, label or year of an existing track: these fields are only filled in when they are empty. BPM, key and energy remain driven by Mixed In Key.
- **[B3]** Mixed In Key cues have a stable identifier (`mik-<track>-<n>`) and are updated in place instead of being deleted and recreated on every sync; cues created in Crate are no longer erased. The old copies (random identifiers) are converted once: 2,033 cues converted with no loss or duplicate on the test copy.
- **[B4]** `get_track_cues` no longer re-reads the whole Mixed In Key database every time a track without cues is played: only files opened outside the library (standalone player) are looked up there, and outside the Crate database lock.
- **[B6]** macOS bookmarks are resolved without UI and without mounting volumes (an unplugged disk no longer triggers a mount attempt), `CFError`s are released; `create_bookmark` is reserved for tests.
- **[B7]** The fingerprint (`file_hash`) is recorded again on import; re-importing a file that is already present no longer crashes (the cues are attached to the existing track, which keeps its identifier).
- **[B8]** Serato Markers2 parser realigned with the real format (index, position, colour, name): cues imported from tags were not in the right place. Cue colours are now read.
- **[B9]** The `energy` column is included in backups and cloud sync (old backups remain readable).
- Tests: 7 new tests (idempotent sync, user metadata kept, stable cues and user cues kept, re-import, Serato parser on an entry built according to the format).

#### Fixed — listening statistics (Crate Pulse)

- **[C9]** Removed the "repair" run at every startup, which turned every Spotify listen shorter than 30 s into a full listen (inflated minutes and streams). Listens that were already modified cannot be restored.
- **[C10]** End of Spotify double counting: the official "recently played" history (every minute) is now the **only** source; the 4-second poller, which recorded the same listen a second time (and a third after a pause), is removed. Listening time is the track duration, capped by the gap with the previous listen (a track skipped after 50 s counts 50 s).
- **[B12]** Crate player: only actual playback time is counted (pauses and seeks within the track excluded, stop at end of track detected); the listen recorded at 30 s is updated with the total listening time when the track changes.
- **[B11]** Listen tracking in Mixed In Key (inferred from an opened file, analysis included) is **disabled by default** and can be enabled in Crate Pulse with a warning.
- **[B13]** Rekordbox XML import: only dated history playlists ("HISTORY 2026-09-20") become listens, dated with the session and in the order played — before, **the whole exported collection** was counted as listened to on the day of the import. A session is only imported once; listens without a real time are excluded from the heatmap. On the `master.db` side, session counters no longer inflate on every sync.
- **[B14]** "7 jours" and "30 jours" (7 days / 30 days) filters: dates are normalised (`datetime()`) before comparison, whatever their format or time zone; an unreadable date no longer makes the heatmap fail.
- **[B15]** Spotify JSON history import: a single transaction, `2024-03-01 20:15` and ISO 8601 dates handled, a malformed date is skipped instead of crashing the import.
- **[B16]** No more lock-order inversion between Spotify sign-out and the poller (risk of blocking the whole database): the poller no longer exists.
- **[B17]** Spotify token refresh serialised (only one at a time); a failure on an expired token is reported instead of silently returning the old token.
- **[B18]** The OAuth server on `127.0.0.1:8888` no longer runs permanently: it starts at sign-in and stops after success or 10 minutes; it requires a `state` generated by Crate; the parameters shown on the callback page are escaped (callback page cut from ~330 to ~40 lines).
- **[B34]** The Mixed In Key service called by the UI is the same instance as the background worker.
- Tests: 11 new tests (pauses, total duration updated, end of track, capped listening time, export dates, HTML escaping, tolerant heatmap, 7-day filter with time zone, batch import, idempotent Rekordbox XML import).

#### Fixed — DJ features

- **[B33]** Heavy backend work no longer runs on the async runtime. A new `run_blocking` helper (`error.rs`) runs synchronous work on tokio's blocking pool and turns a panic into an error; it now carries library import, artwork rescans, waveform and cue reads, Mixed In Key syncs, file deletion, album and standalone-track reads, USB export and sync, Rekordbox XML export, Spotify and Rekordbox history imports, duplicate scans, device listing, ejection and reformatting, and the startup and live Mixed In Key syncs. Three real defects found on the way: **(1)** audio replies were paired with requests only by arrival order through one shared queue, so two crossing calls could read each other's answer and a reply arriving after the 5 s timeout shifted every later reply by one; each request now carries its own reply channel (4 tests). **(2)** Selecting the whole library for analysis started one full decode per track at once (about 60 MB each); at most half the cores, between 1 and 4, run together and the others stay "pending" (4 tests). **(3)** The USB poller ran `diskutil` per volume every 2 s on an async worker; it now polls on the blocking pool and caches the UUID of a mounted volume (3 tests). `resync_mixed_in_key_tracks` reads every file's tags before taking the database lock, then writes everything in one transaction with a savepoint per track (a track that fails to write no longer leaves half-written cues; 4 tests), and the Rekordbox XML export releases the database before building and writing the file and indexes playlist keys with a map instead of a quadratic scan (golden test). The register advised synchronous Tauri commands; in Tauri 2 those run on the main thread, so `spawn_blocking` is the fix. _Left, tracked separately: the Mixed In Key startup/watcher sync still holds the lock while it reads files (CRA-142), plus smaller items (CRA-143)._
- **[C11]** Hot cues: a single 0-based numbering (A = 0 … H = 7) as upstream and Rekordbox use, in the Mixed In Key sync, the Serato parser and the reading of external files. On the UI side, a shared utility (`shared/utils/cues.ts`) maps pad ↔ cue: key 3 triggers cue 3 (and no longer cue 2), a memory cue no longer takes up a pad, and the waveform markers show 1–8 for hot cues and "M" for memory cues. Cues already in the database are converted automatically on the next sync.
- **[C12]** Valid Rekordbox XML export: `Location` paths fully percent-encoded in UTF-8 (`&`, `#`, `%`, spaces, accented characters) then escaped, memory cues exported with `Num="-1"` (and no longer as hot cue A), loops as `Type="4"` with `End`, cue colour carried over, no more invented BPM (120) or bitrate (320) for unknown values.
- **[B27]** Real waveform: peaks are computed from the audio file on first display (symphonia, 400 bars, ~90 ms for a 2.4 MB MP3) then cached; an unreadable file shows a neutral line instead of a fake pattern, and the previous track's waveform is no longer kept.
- **[B28]** Search: the FTS5 query is tokenised like the index ("You'll" → `"You"* "ll"*`); titles with an apostrophe, hyphen or slash ("You'll", "Jay-Z", "AC/DC") are found again, and `AND`, `OR`, `NOT` typed in capitals no longer make the list fail.
- **[B29]** Playback position: exact position from rodio (`Sink::get_pos`) at normal speed; at the end of the track the position is no longer frozen on an old value.
- **[C13]** `loadTracks()` gets back upstream's behaviour (no argument: all tracks, filter reset): removing the last tag empties the filter again and the "Mix harmonique" (harmonic mix) filter no longer stays stuck. The background refreshes added by the fork (Mixed In Key sync, duplicates, upgrader, player) use the new explicit `reloadWithCurrentFilter()` method.
- **[C14]** Dragging a tag onto a track works again (`data-track-id` and hover highlight restored on the rows).
- **[C15]** Never two sounds at once: a preview (discovery or Beatport) also stops the native audio engine when it is playing a file from the standalone player (only library tracks were stopped).
- **[F1]** Keys 1 to 8 (hot cues) are only active in the Player and Library views, with a track loaded.
- **[F2]** Space in the Player view pauses whatever is playing (including a preview) instead of starting a recent file.
- **[F3]** Space/Enter on a library row no longer triggers the global shortcut at the same time.
- **[F11]** Cues and waveform loaded for a file opened outside the library (lookup by path instead of a non-existent identifier).
- **[B30]** Audio file associations at `Alternate` rank without the `public.audio` umbrella: Crate appears in "Open With" without imposing itself as the default player. Removed the `set_as_default_audio_player` command (never called, targeting a non-existent `com.crate.app` bundle) and the `scripts/set_default_player.swift` script.
- **[B31]** Files opened with Crate at startup: a Rust-side queue drained only once by the UI when it is ready (`take_startup_files`); a file is no longer opened twice and several files opened together are all taken into account.
- **[B32]** Deleting tracks along with their files: moved to the Trash via Finder (path passed as an argument, never interpolated into the script), a track only leaves the library if its file actually went to the Trash, failures are reported, and no more permanent deletion outside macOS (`services/trash.rs` module, also used by the upgrader).
- **[B35]** The toolbar's duplicate counter is cached against a fingerprint of the library: the full scan no longer runs on every `duplicates-updated` event, only when the library has changed.
- Tests: 9 new Rust tests (search, path encoding, cue marks, valid XML with special characters, waveform peaks) and 5 Vitest tests (hot cue pads, 1–8 shortcuts limited to the right views).

#### Fixed — frontend

- **[L4]** Every backend error message is now English (about 30 strings in the Beatport client, the upgrader report, album import and `CrateError`), as the repository rule requires. The frequent, actionable ones are translated by the interface through a small table in `toErrorMessage` (`localizeBackendError`: Beatport sign-in required or expired, artist not found, invalid album folder, no audio in folder), so a French user still reads French. The pages shown in the browser after the Spotify sign-in follow the language chosen in Crate (English and French, others in English). While translating, the upgrader was found to detect an expired Beatport session by matching the French text `Authentification Beatport requise`: translating it alone would have broken that silently, so the match now uses a shared constant (`AUTH_REQUIRED_PREFIX`) covered by a Rust test.
- **[L3]** _(partial)_ Numbers and dates follow the language chosen in Crate instead of the system one: a new `formatNumber` helper (`Intl.NumberFormat`) replaces the bare `toLocaleString()` calls in Crate Pulse ("2 310" in French instead of "2,310"), the "Date added" column of the library now uses the user's date format and language through `formatDate`, and the month and weekday labels of the editor's calendar are reactive to the app language. Seven Vitest tests cover the helper. _The "plays" / "écoutes" wording is still mixed until the Pulse strings move to translation keys (L1)._
- **[L2]** Applied the owner's language policy (CRA-114): French and English stay complete, the 13 other locales keep falling back to English, and the README now says so instead of advertising 15 fully translated languages. Checking it found one key missing in French (`toast.tracksExported`, the USB export toast), now translated. New Vitest guards in `shared/i18n/locales.test.ts` fail if French lacks a key English has, has a key English lacks, or uses different `{arguments}`, and check that a language missing a key shows the English text rather than the raw key.
- **[L5]** The interface language now switches synchronously between English and French: `register('en')` and `register('fr')` were removed because `svelte-i18n` sends any locale that has a registered loader through its asynchronous queue, which undid the `addMessages` calls placed just above to avoid that. The other 13 languages stay lazy-loaded and fall back to English. Two Vitest tests in `shared/i18n/index.test.ts` fail without the fix.
- **[I1]** Backend error messages are finally displayed: Tauri rejects a command with a *string*, which every `error instanceof Error ? … : 'generic message'` threw away. New shared helper `toErrorMessage()` used by all stores and components (~85 occurrences, upstream included).
- **[I2]** Beatport tab: the settings that had no effect (AAC/MP3 quality, automatic Mixed In Key sync) are replaced by accurate information — verified FLAC downloads only, Mixed In Key analysis picked up automatically by the watcher on its database.
- **[I7]** The `library-updated` event (emitted after an upgrade) is listened to and refreshes the library; unused `searchType` parameter removed from the Beatport search.
- **[F4]** Initialisation: a step that exceeds its timeout no longer loses its cleanup function (menu listeners, media keys, initialisation); it is run on close, and failures are logged with the step's name.
- **[F5]** The splash screen closes when initialisation (settings included) is finished, and no longer after a fixed 1.5 s timer that could make the onboarding appear by mistake; 10 s safety net.
- **[F7]** The toolbar's "doublons" (duplicates) badge only re-runs the count when the number of tracks changes, no longer on every change to the list.
- **[F10]** Adding a player file to the library: the source store is updated instead of modifying a derived value.
- **[F13]** "Synchroniser avec Mixed In Key" (Sync with Mixed In Key) in the context menu only syncs the selected tracks.
- **[F15]** Tooltips close on click, when the pointer leaves and when the window loses focus (the one on the Mixed In Key badge stayed visible when the button became disabled during the sync).
- **[F8]** Beatport cart: track-by-track download with progress ("2/5 : Titre"); only the tracks actually downloaded leave the cart, failures stay in it with their reason. Beatport favourites and playlists, which are local only, are announced as such (no more fake "success").
- **[F9]** No more side effects when modules are imported: the Beatport token refresh timer starts with the explicit session restore at launch.
- **[F12]** Playback position: a backend response requested before a seek, a track change or a stop is ignored (it made the playhead jump back).
- **[I4]** `BeatportAuthState` now has only one schema, the backend's (snake_case); the duplicate camelCase fields are removed everywhere.
- **[I8]** Optional fields of the Beatport types accept `null`, the value the backend actually sends (two unguarded accesses fixed along the way).
- **[I9]** Library display options (columns, Camelot zero…) are saved in the database (and therefore backed up and restored with it); `localStorage` is now only a cache at startup.
- Tests: 3 Vitest tests for `toErrorMessage`.

#### Tooling and quality

- **Browser harness and end-to-end checks (CRA-134).** The throw-away harness of the 25 September audit is now versioned and automated. `yarn harness` (port 1430) runs the real frontend in a browser against a fake Tauri backend (`apps/desktop/harness/`, using `mockIPC`): 16 deterministic tracks, playlists, tags, Pulse statistics, Beatport, albums and discovery fixtures typed with the real `shared/types`, a safe default for every command nobody handled (each logged once in `window.__harness.unmocked`), and URL parameters for theme, accent, language, font, logged-out Beatport, empty library, onboarding and a playing track. `yarn test:e2e` (Playwright, the one new dev dependency, using the Chromium already cached, nothing downloaded) opens 8 views in dark and light, at 1000×600 and 1400×900, in English and French (74 tests, about a minute): no uncaught error, no `console.error`, no crash screen, a visible landmark, a screenshot, and the in-page audit (contrast, small text, unnamed controls, pointer-only elements, overlaps, crushed columns) compared with `e2e/baseline.json`, a **ratchet**: a count may go down, never up. First baseline, 1000×600 dark: Pulse has 163 low-contrast texts and 173 pointer-only elements, Beatport 86 and 72 crushed columns, the library 29 unnamed controls. Also found: SvelteKit ignores Vite's `transformIndexHtml` on `app.html` (a middleware injects the harness instead), the audit script caps every list at 40 entries (lifted in memory so the ratchet is not blind), the segmented control highlights "Beatport" while on Pulse or Discovery, and hard-coded French strings remain in the English interface (toolbar labels, Pulse, toasts).
- **[Q9]** The dev and staging builds are now recognisable in the Dock: their icon carries a blue `DEV` or purple `STG` band under the crate, while prod keeps the plain icon. The three were already byte-for-byte identical in upstream, so this is an improvement rather than a fork regression. Every format (`.icns`, `.ico`, PNG sizes, Windows, iOS, Android) was regenerated with `yarn tauri icon` from the band-marked source.
- **[Q15]** Design agent for coding assistants, built from five sources (taste-skill, Vercel's Web Interface Guidelines, image-to-code, awesome-design-md, Playwright CLI) and adapted to a dense desktop app:
  - `DESIGN.md` at the root (Stitch/awesome-design-md format): the real tokens from `style.css` for both themes, the typographic scale of the `Text` component, radii, elevation, common components, Camelot/energy data palettes, dos and don'ts, known gaps (contrast of status colours in the light theme: amber measured at 2.15:1 on white);
  - `design` agent (`.claude/agents/design.md`, project memory) that preloads five skills: `crate-design-system` (the twelve strict rules, debt → tokens conversion table, loaded automatically as soon as a file in `apps/desktop/src` is touched), `crate-ui-audit` (file:line audit linked to the register, with the Web Interface Guidelines adapted to Svelte/Tauri and the "AI" signatures in a dense app), `crate-ui-build` (reading the need, redesign protocol, ordered levers, pre-delivery check), `crate-image-to-code` (analysis of a screenshot or mock-up translated into tokens, inspiration library), `crate-visual-check` (light/dark × accents × 1000×600/1400×900/1920×1080 × fr/en matrix with Playwright CLI);
  - `ui-audit.js` script run in the page: real WCAG contrast through transparent layers, unnamed controls, clickable `div`s, overlaps, modals outside the window, squashed columns, text under 12 px (checked on a trap page: 9 defects out of 9 detected, no false positives);
  - `yarn design:scan [paths] [--details] [--strict]` scanner: palette, hex, `dark:`, arbitrary sizes, `transition-all`, radii, blur/glow, gradients, font weights, `svelte-ignore a11y`, `outline-none`, animations without `motion-reduce`, arbitrary z-index values, fixed heights, emojis, hard-coded strings, "..." in the locales. Starting point: 1,220 offending lines;
  - `CLAUDE.md` and README point to `DESIGN.md` and the agent; `.playwright-cli/` ignored by git.
  _Automatic visual verification requires a browser harness (fake Tauri backend) that has yet to be versioned._

- **[Q4]** Rust: `cargo fmt` applied (40 files) and `cargo clippy --features desktop -- -D warnings` passes (37 errors → 0): dead code removed (`ListenSource`, `set_track_rating`, `set_track_color`, `extract_bpm/key`, `detect_rekordbox_dir`, unused Spotify response fields, unneeded re-exports), iterations and sorts simplified.
- **[Q5]** TypeScript/Svelte: Prettier applied and ESLint at 0 errors (38 → 0): keys on every `{#each}` loop, no more explicit `any`, writable `$derived` in the search bar, non-reactive caches documented.
- **[Q6]** Fork CI on Linux: a frontend job (format, lint, types, Vitest) and a Rust job (clippy, tests) on every push; the Rust job also checks that the code compiles outside macOS.
- **[Q3]** `RunEvent::Opened` limited to the platforms that provide it (macOS, iOS, Android): the code compiles again on Windows/Linux.
- **[Q8]** Test dependencies pinned (Vitest 4.1.11, testing-library, jsdom), a single Vite version (7.3.0, via `resolutions`) for the app and the tests, `@vitest/coverage-v8` coverage provider added (`yarn test:coverage`).
- **[Q13]** `yarn dev` builds the app in debug (fast rebuilds) with optimised dependencies (`[profile.dev.package."*"]`) to keep audio decoding smooth.
- **[B10]** Migrations: labels aligned with their actual position (6 to 15) and the "append, never renumber" rule documented; the order is not changed because the local database has already applied them.
- **[Q11]** `set_default_player.swift` script deleted; the previous assistant's summary is archived in `tracking/history/`.
- **[Q10]** Documentation: keyboard shortcuts page corrected (arrows ±10 s, Cmd+arrows ±1 s, Shift+arrows depending on the view, hot cues 1–8, Shift+Tab).
- **[Q7]** Risky paths covered by tests on temporary databases and folders: file replacement, Mixed In Key sync, listen trackers, import, export, Trash; no test reads the real Mixed In Key database any more.
- **[Q12]** `CLAUDE.md` in place since the start of the fork (tracking rules and technical rules).

#### Fixed — visual foundations

- **[D2]** Missing design tokens declared for the light and dark themes (`surface-3`, `surface-4`, `stroke-strong`, `text-disabled`): ~70 classes (loading skeletons, progress tracks, hover states) that produced no style at all are finally rendered. The accent colour is exposed in `@theme inline`, which makes opacity modifiers (`bg-brand-primary/20`…) work while following the chosen accent live.
- **[D1]** `dark:` now follows the theme chosen in Crate (`[data-theme]`) and no longer the system's: the "light mode" styles added by the fork are no longer wrong when macOS and Crate differ.
- **[D6]** Modals bounded by the window (width and height minus 2rem); the Duplicate Killer fills the modal instead of overflowing it, and its footer and "Supprimer" (Delete) button stay visible.
- **[D8]** Jost and DM Sans fonts actually usable (`[data-font]` rules and values accepted by the backend: the choice was lost on restart); 15 unused files deleted (5 kept, 392 KB), duplicate Google Fonts loading removed, OFL licences referenced.
- **[D9]** Seven missing icons added (including `close`, missing since upstream) and a Vitest test that fails on any unknown icon; Beatport logo drawn inline (it came out black on a dark background).

#### Fixed — views

- **[D4]** Header usable at every width: below 1536 px the Player/Library/Beatport switcher sits after the logo instead of being centred on top of the tools; below 1400 px the Mixed In Key badge switches to a compact logo; below 1280 px the Import/Add buttons become icons (with an accessible name). Verified by measurement in the browser harness at 1000, 1280, 1440 and 1600 px.
- **[D5]** Player hero proportional to the window height (artwork from 140 to 260 px, column at natural height, reduced margins on short screens): the playback controls are no longer covered at 1000×640; the recent files list takes the remaining space.
- **[D6]** Verified in the browser harness: the Duplicate Killer's "Supprimer" (Delete) button is visible at 1400×900 and at 1000×640.
- **[D12]** "Build 57" badge removed from the header (the version remains under "À propos", i.e. About) and "Pro" mentions removed from the Mixed In Key labels.
- `Button` accepts an `aria-label` (icon-only buttons).

#### Documentation

- **Graphic charter (CRA-140).** `DESIGN.md` now records the owner's decision to keep every colour family (CRA-115) and sets the limits that were missing: the four families (accent, Deck, Pulse, Beatport) with their purpose, scope, light and dark roles and proposed tokens, a combination matrix (where each family may appear), the rules shared by all (surfaces, tints, borders, radius, glass and glow, type, states, motion, contrast), WCAG contrast ratios computed from the real token values (dark theme holds for every family; the light theme fails wherever a family hue is used as text), a seven-step checklist for a new view, and a table of the deviations still in the code (input for D3, D7, D10, D11). The former "bring every view back to the accent" direction was replaced. Five choices that are the owner's to make are tracked in CRA-141; until then the design fixes apply the recommended option, except the Beatport surfaces, which stay untouched.
- **[L6]** The README now announces the 15 languages actually shipped (instead of 11) and describes the fork, the tests and the tracking.

#### Tooling and tracking

- Private personal GitHub repository, tracking documents in `tracking/` (status, register, report, history), `CLAUDE.md`, `yarn status` script that recomputes progress.
- Upstream workflows (`ci.build`, `ci.lint`, `cd.docs`) switched to manual triggering: they ran macOS and Windows builds on every push, which is costly on a private repository and they were still failing (see Q2 to Q5). New `ci.fork.yml` workflow: Vitest on every push; `yarn test` first runs `svelte-kit sync` so that it works on a fresh clone.

#### Fork work prior to the audit (builds 36 to 57)

Committed as is in a single snapshot so as not to risk losing it again (**[C1]**); the known defects of this code are listed in the register.

- Standalone player and macOS audio file association (Player view, recent files)
- Mixed In Key 11 integration: reading `Collection11.mikdb`, cues, energy, watcher
- Beatport Quality Upgrader: search, scoring, FLAC download via `beatportdl`
- Crate Pulse: multi-source listening statistics (Spotify, local player, Mixed In Key, Rekordbox)
- Duplicate Killer, albums view, FTS5 full-text search, hot cues 1 to 8, harmonic mix, DJ shortcuts, Rekordbox XML export
- Vitest, testing-library and jsdom; 144 TypeScript tests

### Upstream (blackboxaudio)

### Added

- Provisioned the mobile database encryption key through the iOS Keychain / Android Keystore behind a feature-gated `KeyProvider` abstraction, so the SQLCipher key is never written as a plaintext file on mobile (desktop keeps its existing key-file behavior)

### Changed

- Prepared the backend to compile for mobile targets (iOS/Android) by gating desktop-only services (audio playback, USB export/sync, file import, track analysis, media keys, device detection) behind a default-on `desktop` Cargo feature, keeping desktop builds unchanged

## [0.2.9] - 2026-06-09

### Added

- Added shuffle mode to the audio player
- Added opt-in cross-device cloud sync for libraries, playlists, tags, cues, and discovery releases (audio files stay local)
- Added macOS keyboard shortcuts for hide/hide others/show all
- Added a right-click context menu for discovery tracks (like/unlike, play preview, search on YouTube, open/copy release URL) plus a "Search on YouTube" action on the release menu
- Added the ability to follow artists and labels (Bandcamp, SoundCloud, Discogs) to automatically surface their new releases in Discovery, with upcoming-release badges, release-day notifications, and a Following manager

### Fixed

- Fixed backup progress bar not visible due to invalid Tailwind color classes
- Fixed locate track functionality to check current playlist first
- Fixed continuous playback selecting next track from wrong context when navigating between views
- Fixed discovery row buttons (import and open URL) not working in playlist view

## [0.2.8] - 2026-03-15

### Added

- Added guided feature tour for first-time users

### Fixed

- Fixed metadata auto-fetching for unsupported URL domains in discovery
- Fixed editor form resetting during bulk metadata refresh for discovery releases
- Fixed particular strings not being translated on locale change

## [0.2.7] - 2026-03-14

### Added

- Added clickable track name in the player bar to scroll to and highlight the currently playing track
- Added unified filter panel for library and discovery views with per-context filter state
- Added click-to-enlarge artwork modal for discovery releases
- Added dynamic sidebar header that updates to match the active context (Library / Discovery)
- Added bulk drag-and-drop and "Move to Folder" context menu for multi-selected playlists
- Added persistence of navigation state, playlist tree scroll position, and discovery release expansion across restarts
- Added information display when restoring from a backup

### Fixed

- Fixed support for bulk-adding Bandcamp pages that use alternative indexing
- Fixed discovery playlist search not filtering by track name
- Fixed multi-select drag clearing selection when clicking to initiate a drag
- Fixed renaming smart playlist names in the modal to edit smart rules
- Fixed metadata refreshing in discovery playlists views

## [0.2.6] - 2026-03-14

### Added

- Added Ukrainian, Romanian, Polish, and Turkish locale support
- Added first-run onboarding setup wizard with language, theme, accent color, and font customization
- Added persistence of player state, including current track, playhead position, tempo control, and volume control 
- Added Apple code signing and notarization for macOS builds

### Changed

- Improved rendering of lists for library, discovery, and playlist views

### Fixed

- Fixed macOS Tahoe (26) compatibility issues
- Fixed database foreign key violations during restore across app installations
- Fixed discovery row buttons not working intermittently

## [0.2.5] - 2026-03-09

### Changed

- Improved metadata enrichment for discovery releases during bulk imports

### Fixed

- Fixed discovery selection bugs when navigating in-context
- Fixed Bandcamp discography parsing to include all releases

## [0.2.4] - 2026-03-09

### Fixed

- Fixed the "is liked" toggling of discovery tracks

## [0.2.3] - 2026-03-09

### Changed

- Improved search logic for discovery releases

### Fixed

- Fixed bug where bulk operations on filtered selections was misleading

## [0.2.2] - 2026-03-09

### Added

- Track-level likes for discovery releases with heart toggle and filter to show only releases with liked tracks

## [0.2.1] - 2026-03-08

### Added

- Smart playlists with rule-based auto-population for both library and discovery contexts
- Library backup and restore functionality in Settings > General

### Changed

- Replaced OS keyring with local key file for database encryption to avoid first-launch Keychain prompt

## [0.2.0] - 2026-03-08

### Added

- Seamless in-app updates via Tauri updater plugin; checks on launch and hourly, shows update modal with release notes and download progress
- Continuous playback setting for automatically playing the next track
- Music discovery feature for tracking releases from Bandcamp, SoundCloud, YouTube, and Discogs
- Discovery settings tab with auto-fetch metadata, transfer tags on import, and remove release after import preferences
- Automatic metadata fetching for discovery releases from Bandcamp, SoundCloud, YouTube, and Discogs URLs
- Playlist support for discovery releases with separate playlist hierarchies per view
- Export playlists to USB devices with Pioneer/Rekordbox compatibility
- Multi-language support with 11 locales: English, Japanese, Dutch, French, German, Spanish, Italian, Swedish, Korean, Portuguese, and Chinese
- Automatic system language detection with user preference override in Settings
- Track BPM and key analysis
- Discovery release deduplication with overlap detection during add flow
- Expandable track sub-rows in the discovery list with expand/collapse all
- Merge releases action for combining duplicate discovery entries
- SoundCloud set/playlist URL support for fetching all tracks in a set
- Bandcamp parent album detection for individual track pages
- YouTube preview playback support for single videos and playlists in discovery

## [0.1.0] - 2024-12-20

### Added

- Library management with automatic metadata extraction
- Playlist and folder organization
- Tag system with AND/OR filtering
- Audio playback with device selection
- USB device monitoring
- Waveform display with cue point management
- Search and filter across entire collection

[Unreleased]: https://github.com/blackboxaudio/crate/compare/v0.2.9...HEAD
[0.2.9]: https://github.com/blackboxaudio/crate/compare/v0.2.9...v0.2.9
[0.2.9]: https://github.com/blackboxaudio/crate/compare/v0.2.8...v0.2.9
[0.2.8]: https://github.com/blackboxaudio/crate/compare/v0.2.7...v0.2.8
[0.2.7]: https://github.com/blackboxaudio/crate/compare/v0.2.6...v0.2.7
[0.2.6]: https://github.com/blackboxaudio/crate/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/blackboxaudio/crate/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/blackboxaudio/crate/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/blackboxaudio/crate/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/blackboxaudio/crate/compare/v0.2.2-staging.1...v0.2.2
[0.2.1]: https://github.com/blackboxaudio/crate/compare/v0.2.1-staging.1...v0.2.1
[0.2.0]: https://github.com/blackboxaudio/crate/compare/v0.2.0-staging.1...v0.2.0
[0.1.0]: https://github.com/blackboxaudio/crate/releases/tag/v0.1.0
