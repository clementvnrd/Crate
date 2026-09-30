> **Frozen snapshot of the audit of 25 September 2026.** This document describes the state of the code *before* the fixes. Live progress is in [STATUS.md](STATUS.md) and the change log in [CHANGELOG.md](../CHANGELOG.md). Editable version: [Crate — Full audit & vision](https://claude.ai/code/artifact/9a600818-17fb-44e6-b7c1-6924af9c11f5).

# Crate — Full audit & vision


## Executive summary

Verdict: Crate is an excellent foundation and the fork has very good ideas, but it is not safe to use as is on your real library. It first needs to be consolidated, then polished, and only after that should features be added.

- **The upstream foundation is solid**: encryption, migrations, IPC contract, design system and CI are of professional quality.
- **The fork works**: the app starts, runs for 25 minutes without errors, all 371 tests pass and the backend ↔ frontend contract is perfectly aligned.
- **But it can destroy data**: automatic purge of tracks missing from Mixed In Key, writes to the Mixed In Key database, an upgrader that empties folders and loses cues.
- **And several things it displays are wrong**: offset hot cues, fake waveform, doubled Spotify statistics, a search that ignores apostrophes.
- **A Beatport password is in plain text in the code** and 27,800 lines of work are in no commit.

| Key figures | Value |
| --- | --- |
| Unique critical defects | 15 |
| Lines added but not committed | about 27,800 |
| CI checks that would fail | 5 of 8 |

**Recommendation**: scenario A (consolidate, about 3 weeks) then scenario B (polish and unify, about 2 months). The first day is used solely to make the work safe: change the password, back up the databases, commit on a local branch, switch off the purge. The details of each defect and its fix are in the companion document, the defect register.

## What Crate is today

Crate is a desktop DJ library manager, version 0.2.9, built on Tauri 2: a Rust backend, a SvelteKit frontend in Svelte 5 (runes), and a SQLite database encrypted with SQLCipher. The repository is a personal fork of `blackboxaudio/crate`, with the `develop` branch tracking `origin/develop`.

| Layer | Technology | Volume measured on 25 Sept. 2026 |
| --- | --- | --- |
| Backend | Rust, rusqlite + SQLCipher, rodio/symphonia, tokio | 54,260 lines, 125 IPC commands, 16 SQL migrations |
| Desktop frontend | SvelteKit, Svelte 5, Tailwind | 33,560 lines, 139 components |
| Shared code | TypeScript (IPC API, stores, types, i18n, utils) | 13,833 lines |
| Mobile | SvelteKit (iOS/Android skeleton) | 15 lines of source |
| Languages | svelte-i18n JSON | 15 locale files (the README advertises 11) |
| Tests | cargo test, Vitest | 227 Rust tests, 144 Vitest tests |

**Scope inherited from upstream**: import and metadata extraction (MP3, FLAC, WAV, AIFF, M4A), tags and categories, manual and smart playlists, analysis (waveform, key, BPM, energy), Bandcamp/SoundCloud/YouTube/Discogs discovery with artist and label following, playback with cues, Pioneer CDJ/XDJ USB export with Rekordbox database generation, incremental device sync, bulk metadata editing, themes and accents, backups, optional cloud sync, automatic updates.

**Scope added by the fork** (not committed, detailed in the next section): standalone player with macOS audio file association, direct reading of the Mixed In Key 11 database (`Collection11.mikdb`, cues and energy), MP3 to FLAC replacement via Beatport, the multi-source listening statistics dashboard "Crate Pulse" (Spotify, local plays, Mixed In Key, Rekordbox sessions), duplicate detection, albums view, FTS5 full-text search, hot cues 1 to 8, harmonic mixing assistant, DJ keyboard shortcuts, Rekordbox XML export.

**The owner's real environment**: a 2.6 MB `crate.db` database (a library of a few hundred tracks), 419 cached artworks, a 73 MB automatic backup, and Rekordbox 7, Mixed In Key 11 and Spotify installed on the machine. The modest size of the library makes "10,000 tracks" optimisations less urgent than reliability and polish.

## What the personal fork has done

All the work from builds 36 to 57 exists in the code and works (the app installed on 14 September matches the current working tree exactly), but nothing is committed: 174 pending paths, zero commits ahead of `origin/develop`.

| Theme | Builds | What exists in the code | Key files |
| --- | --- | --- | --- |
| Standalone player and macOS default player | 36 to 40 | `PlayerView` view, recent files, audio type association in `tauri.conf.json`, Swift script `set_default_player.swift`, handling of the file opened at startup | `components/player/PlayerView.svelte`, `services/standalone.rs`, `commands/standalone.rs` |
| Mixed In Key 11 integration | 1 then 56 | Reading `Collection11.mikdb`, sync of cues and energy, file watcher, macOS bookmarks | `services/library/mik_db.rs`, `mik.rs`, `macos_bookmark.rs` |
| Beatport Quality Upgrader | 41 to 44 | Multi-pass search, scoring, FLAC download, integrity validator, SQLite cache, migrations 11 and 12 | `services/beatport/`, `components/upgrader/`, `components/beatport/` |
| Crate Pulse (statistics) | 45 to 48 | `listen_events` table, trackers for Spotify (OAuth PKCE loopback), Rekordbox, Mixed In Key and the local player; KPIs, tops, Camelot wheel, heatmap; migration 13 | `services/stats/`, `components/stats/` |
| Duplicates and albums | undated | Duplicate detection with ignored groups, player albums view | `services/duplicate.rs`, `services/album.rs` |
| DJ Pro overhaul | 57 | FTS5 (migration 16), on-demand waveform, sample-based audio position, hot cues 1 to 8, harmonic mixing, shortcuts, Rekordbox XML export | `db/schema.rs`, `services/library/query.rs`, `services/audio/mod.rs`, `services/export/rekordbox_xml.rs` |
| Tooling | 56 to 57 | Vitest, testing-library, jsdom, 11 TS test files, synchronous en/fr i18n | `vitest.config.ts`, `shared/i18n/index.ts` |

| Git state on 25 Sept. 2026 | Value |
| --- | --- |
| Modified tracked files | 100 (+4,835 / −1,035 lines) |
| Untracked paths | 74 |
| New untracked code | about 14,000 lines |
| Commits ahead of upstream | 0 |
| Branch, stash, backup of the work | none |

Two configuration changes went unmentioned in the previous summary: `tauri.prod.conf.json` now only produces the `.app` bundle and disables update artefacts, and the icons of the three variants (dev, staging, prod) have been replaced. `CHANGELOG.md` was not touched.

## Strengths

The upstream foundation is of very high quality, and the fork has brought genuinely good ideas: the problem is not the vision, it is polish and safety.

**What upstream does well and should be preserved**

- **Data security**: SQLCipher everywhere, key abstraction with a compile-time guard, iOS Keychain and Android Keystore correctly configured.
- **Minimal Tauri surface**: scoped permissions, no shell or http plugin on the webview side, asset protocol restricted to the data folder.
- **Transactional migrations**: each migration and its version number are committed in the same transaction.
- **A real design system**: three surfaces, three levels of text, brand colour driven by the chosen accent, common components (Text, Button, Modal, Tooltip…).
- **Complete CI**: TypeScript and Rust lint, desktop and mobile svelte-check, macOS and Windows builds, iOS/Android check, dependency audit, release gated by the CHANGELOG.

**What the fork did well**

- **Flawless IPC contract**: all 230 commands are registered, all 226 frontend calls find their command, zero argument mismatches, everything goes through `shared/api`.
- **Mirror types**: the new Rust models (stats, upgrader, duplicates, albums, player) match the TypeScript interfaces field for field.
- **On-demand waveform**: `get_tracks` no longer carries the blob, a single guarded consumer.
- **Real fixes**: the i18n crash at startup and the CPU loop in the Mixed In Key watcher are properly resolved (CPU measured at 0.1% when idle).
- **Domain safeguards**: FLAC validation (signature and size) before deleting an MP3, read-only access to the Mixed In Key database for the sync, PKCE with a persisted `state` for Spotify, Rekordbox without an embedded key.
- **Tests added**: 144 Vitest tests passing in 3 seconds, 52 new Rust tests, mostly on an in-memory database.
- **Product**: Crate Pulse looks great in the dark theme, the Duplicate Killer and the Upgrader present clear comparisons, and the Player view with cue pads is pleasant to use.

## Measured technical health

The app compiles, starts and runs without crashing on your machine, but 5 of the 8 upstream CI checks would fail if the tree were committed as is. Everything below was run on 25 September 2026.

| Check | Result | Detail |
| --- | --- | --- |
| Vitest | OK | 144 tests in 11 files, all passing |
| svelte-check desktop and mobile | OK | 0 errors, 2 accessibility warnings |
| cargo check (desktop, release) | OK | 15 warnings |
| cargo test | OK but dangerous | 227 of 227; 2 tests write to your real Crate and Mixed In Key databases |
| cargo clippy with -D warnings (CI flags) | Failure | 37 errors, mostly dead code |
| ESLint | Failure | 38 errors (missing keys in `each` blocks, `any`) |
| Prettier and cargo fmt | Failure | 60 TS files and 35 Rust files not formatted |
| iOS/Android build | Failure | 10 errors: new modules not gated by the `desktop` feature |
| Windows/Linux build | Failure (analysis) | `RunEvent::Opened` only exists on macOS |
| IPC contract | OK | 230 commands registered, 226 invoked, 0 orphan calls, 0 argument mismatches |
| Events | OK | 22 of 23 paired; `library-updated` emitted without a listener |
| i18n | Partial | 17 new components with no translation at all; 13 of the 15 locales lack 49 to 53 keys |

**Real run of `/Applications/Crate.app`** for 25 minutes with detailed logs:

- No panic, no error, CPU at 0.1% when idle.
- The Mixed In Key sync runs 4 times in 14 seconds at startup, then again 11 minutes later. Each run rewrites your 276 tracks without detecting any change, re-reads the audio files and restarts the artwork search. Each run also triggers the strict purge described in the defect register.
- A "repair" of Spotify listens runs at every startup. It turns short plays into full listens.
- The Spotify OAuth server listens permanently on port 8888 from launch.

**Visual rendering**: I built a harness that runs the frontend in a browser with a fake Tauri backend and 16 fake tracks. It made it possible to capture every view in the light and dark themes, in French and in English, at 1000×640, 1400×900 and 1920×1080. The visual defects found are detailed in the register. The harness can be reused for automated tests.

## Main weaknesses

The fork was built fast and wide: each feature works in the nominal case, but several can destroy data, and the whole has lost upstream's visual and linguistic consistency. The six audits produced about 250 raw findings; the defect register groups and ranks them.

**The seven problems that matter most to you**

1. **Risk of data loss in your library.** At startup and on every window focus, every Crate track missing from Mixed In Key is deleted. The upgrader can empty subfolders of your music folder and loses the cues, tags, playlists and notes of the replaced track.
2. **Writes to the Mixed In Key database.** Crate deletes rows in `Collection11.mikdb`, a third-party Core Data database, with no backup and no consent. Two Rust tests do the same thing on your real databases.
3. **A Beatport password in plain text in the source code.** It is also rewritten to disk on every download. It must be changed before any commit.
4. **Skewed statistics.** Spotify listens are counted twice, and plays shorter than 30 seconds become full listens at every startup. The Mixed In Key tracker counts an open file as a listen.
5. **Wrong DJ features.** Hot cues are off by one (key 3 jumps to cue 2). The "real waveform" has no source: no code writes `waveform_data`, so what is displayed is a fake pattern. The Rekordbox XML export produces an invalid file as soon as a path contains `&`. The FTS5 search no longer finds tracks with an apostrophe or a hyphen.
6. **Regressions from upstream.** Removing the last tag filter no longer clears the list, dragging and dropping a tag onto a track no longer does anything, and the Rust engine can play at the same time as the preview.
7. **Visual and linguistic inconsistency.** Three visual languages coexist, no new view follows the accent colour, the light theme is broken on Crate Pulse, and all the new views stay in French when the app is in English.

**Legal risk to be aware of**: the FLAC download does not go through your Beatport purchases. It drives the third-party tool `beatportdl`, which fetches the streams of the streaming subscription, using the OAuth client from the Beatport documentation. This breaches Beatport's terms. For strictly personal use, that is your decision; this code must, however, never be pushed to a public repository.

**Claims in the previous summary that are wrong**: the audio position is not slaved to the CoreAudio clock, it is a wall clock multiplied by the speed. There are 15 migrations, not 16. The Mixed In Key watcher is not "purely read-only". The added volume is about 27,800 lines, not 14,000.

## What Crate could become

Crate has everything it needs to become a DJ's personal cockpit: the reference library, the preparation tool and the dashboard of what one actually listens to and plays. It replaces neither Rekordbox (mixing) nor Mixed In Key (analysis): it connects them and it measures. The right direction is not to add features, the fork already has too many for its maturity, but to make each one flawless and consistent.

**Three pillars, aligned with what you love**

| Pillar | Promise | What already exists | What is missing |
| --- | --- | --- | --- |
| Flawless library | Everything in one place, organised, without duplicates, in maximum quality, consistent with Mixed In Key and Rekordbox | Import, tags, smart playlists, duplicates, FLAC upgrader, reading of Collection11.mikdb, FTS5 | Physical organisation of files, a report of key/BPM/cue discrepancies between the three tools, acoustic fingerprinting for duplicates |
| Personal statistics | Knowing what one listens to and what one plays, across all sources, with insights and a reliable history | Crate Pulse: Spotify, local, Mixed In Key, Rekordbox sessions, heatmap, Camelot wheel | Recaps (week, month, year), a discovery → library → played in a set funnel, data export, reliable counting across sources |
| DJ preparation | Building a set, checking the harmonic and energy flow, exporting cleanly | Hot cues, harmonic mixing, shortcuts, USB and XML export | A "set" view with a key/energy curve, next-track suggestion based on your real sets, writing cues back to Mixed In Key |

**Three scenarios**

| Scenario | Scale | Indicative duration | Content | Outcome |
| --- | --- | --- | --- | --- |
| A. Consolidate | Few changes | 2 to 3 weeks | Commit, fix the critical and major defects, translate the new views, repair responsiveness and the light theme, no new features | A stable and honest personal 0.3.0 release |
| B. Polish and unify | Moderate changes | 1 to 2 months after A | A single visual language driven by the accent colour, Crate Pulse v2 (recaps, exports, funnel), incremental Mixed In Key sync, secured upgrader, "set" mode | The app you describe: beautiful, consistent, and working |
| C. Reinvent | Many changes | 3 to 6 months after B | Pluggable statistics sources, mobile companion for stats, acoustic fingerprinting, assisted tagging (energy, mood), detachable Pulse window | A complete product, at the cost of a long and risky undertaking for a personal project |

Recommendation: A then B. Scenario A is a non-negotiable prerequisite (14,000 uncommitted lines, open critical defects). Scenario B is the one that maximises the pleasure of daily use for someone who loves "les petites tech qui fonctionnent" (small tech that works). Scenario C is only justified if the app becomes a shared or public project.

**High value-for-effort ideas for you**

| Idea | Pillar | Why it is good for you | Effort |
| --- | --- | --- | --- |
| "Your week" and "Your year": automatic recap with top tracks, artists, keys, peak hours, and a shareable image | Stats | The Stats.fm pleasure, but across all your sources including your sets | M |
| Discovery → library → played in a set funnel | Stats | Measures what your discoveries actually become | M |
| Timeline of Rekordbox sessions with tracklist, key flow and energy curve | Stats + Preparation | Review a set like a match, learn from your transitions | M |
| CSV/JSON export of the whole listening history | Stats | Your data belongs to you, an independent backup | S |
| Crate / Mixed In Key / Rekordbox discrepancy report (key, BPM, cues, missing files) | Library | Everything organised and consistent on the computer | M |
| Assisted physical organisation: naming and folder-structure rule with a dry run before moving | Library | The library on disk becomes as clean as in the app | M |
| Smart playlists based on stats ("never played in a set", "top 30 days", "discovered this month") | Library + Stats | Brings organisation and statistics together in a single gesture | S |
| "Set" mode: selection, order, harmonic and energy check, one-click XML/USB export | Preparation | Harmonic mixing becomes a complete workflow | L |
| Next-track suggestion based on your real transitions (Rekordbox sessions) | Preparation + Stats | Personal, not generic | M |
| End-to-end tests on the browser harness built for this audit | Reliability | Every view checked automatically in light, dark and a small window | M |

**What to avoid**: opening new fronts (mobile, AI, new sources) before the existing views share a visual language, a language and a test base. Each feature added by the fork was delivered with its own style, its own hard-coded strings and its own shortcuts: that is what makes the whole less of a "petite tech qui fonctionne" (small tech that works) than it deserves to be.

## Phased implementation plan

The plan follows the recommended scenario: phases 0 to 3 make up scenario A (about 3 weeks), phases 4 and 5 scenario B (about 2 months). The order is dictated by risk: first stop whatever can destroy data, then fix what is wrong, and make things beautiful last.

| Phase | Goal | Main work | Effort | Exit criterion |
| --- | --- | --- | --- | --- |
| 0. Make safe | Lose nothing more | Change the Beatport password and remove it from the code; back up `crate.db` and `Collection11.mikdb`; create a local branch and commit a complete snapshot; disable the strict purge and the writes to Mixed In Key; mark the two tests on real databases `#[ignore]`; restore `tauri.prod.conf.json` | 1 day | All the work is in git, no destructive operation runs at startup |
| 1. Stop data loss | Every file operation is safe | Mixed In Key strictly read-only; purge replaced by a missing-tracks report with confirmation; an upgrader that replaces the file while keeping the track identifier (cues, tags, playlists preserved); download into an isolated temporary folder; FLAC validation by full decoding; removal of `flatten_and_clean_destination` | 3 to 4 days | Integration tests on temporary databases and folders covering purge, upgrade and deletion |
| 2. Correctness | What is displayed is true | Zero-based hot cues everywhere; waveform generated during analysis or read from Mixed In Key; FTS5 search with correct escaping and fallback; escaped XML export and cues compliant with the specification; deduplicated Spotify stats, removal of the "repair", handling of pauses; three upstream regressions; two IPC panics; single incremental Mixed In Key sync | 4 to 5 days | Every fix has a test; search for "You'll" and XML export with `&` verified |
| 3. Hygiene and clean history | A healthy repository | TS/Rust format and lint, dead code, `desktop` gating of modules, Windows fix; split into 9 thematic commits; CHANGELOG, `CLAUDE.md`, icons regenerated per environment, unnecessary fonts removed | 2 to 3 days | Green CI on the branch, personal 0.3.0 installed |
| 4. Visual and language unification | A single app, beautiful and consistent | Foundations (missing tokens, `dark:` variant tied to the app theme, fonts, icons); compliance of Player, Pulse, Beatport, Upgrader, Duplicates; translation of every string into the 15 languages; usable at 1000×600; keyboard accessibility | 10 to 14 days | Light and dark screenshots, 2 accents, 2 window sizes, with no defect; CI scan against hard-coded colours |
| 5. Crate Pulse v2 and DJ preparation | The product you love | Week/month/year recaps; CSV/JSON export; discovery → set funnel; smart playlists based on stats; Crate/Mixed In Key/Rekordbox discrepancy report; "set" mode; end-to-end tests on the browser harness | 4 to 6 weeks | Every feature delivered translated, tested and compliant with the design rules |

Phases 0 and 1 are not negotiable before using the app day to day with your real library. Phase 4 can start in parallel with phase 3 on the visual foundations, because it does not touch the backend.

## Decisions to make

Five decisions are yours to make before starting phase 1; the other technical choices can follow the register's recommendations.

| Decision | Options | Recommendation |
| --- | --- | --- |
| Overall scenario | A alone, A then B, or A then B then C | A then B |
| Beatport download (upgrader) | Keep it private on a branch that is never pushed; connect it only to your purchases; remove it | Private branch while deciding, and never on a public repository |
| Role of Mixed In Key | Source of truth that can delete Crate tracks; read-only enrichment source | Read-only: Crate enriches, never deletes without confirmation |
| Interface language | Translate everything into the 15 languages; French and English only | Complete French and English, the other languages falling back to English |
| Visual language | Return to the accent-driven upstream system; create a distinct "DJ" identity with its own tokens | Upstream system extended with three tokens (cue, live, brand) |

**Open questions**

- [ ] Does the remote repository `blackboxaudio/crate` belong to you, or do you need to create your own private repository to push this work?
- [ ] Do you use Crate on other machines (Windows, iPhone) or only on this Mac? If it is this Mac alone, the Windows and mobile fixes drop to low priority.
- [ ] Should the Spotify history already recorded be kept, given that part of it is skewed by double counting? A re-import from the official Spotify export is possible.
- [ ] Do you want Crate to become the default audio player on macOS, or only to appear in "Open With"?
