# Complete Summary & History of the Crate Project

This document exhaustively recaps all the work, architectural decisions, bug fixes, performance optimisations and features developed over the course of this discussion.

---

## Contents

1. [Origins, macOS Diagnosis & Mixed In Key 11 Pro](#1-origins-macos-diagnosis--mixed-in-key-11-pro)
2. [Evolution & Ergonomics of the Dedicated Player (Builds 36 to 40)](#2-evolution--ergonomics-of-the-dedicated-player-builds-36-to-40)
3. [Beatport Quality Upgrader: MP3 ➔ FLAC Lossless Replacement (Builds 41 to 44)](#3-beatport-quality-upgrader-mp3--flac-lossless-replacement-builds-41-to-44)
4. [Crate Pulse & Advanced Music Statistics (Builds 45 to 48)](#4-crate-pulse--advanced-music-statistics-builds-45-to-48)
5. [Performance Audit, i18n Crash Fix & MIK Watcher (Builds 49 to 56)](#5-performance-audit-i18n-crash-fix--mik-watcher-builds-49-to-56)
6. [Major DJ Pro Overhaul & Complete Optimisation (Build 57)](#6-major-dj-pro-overhaul--complete-optimisation-build-57)
7. [New Location & File Organisation](#7-new-location--file-organisation)
8. [Version Summary Table (Builds)](#8-version-summary-table-builds)

---

## 1. Origins, macOS Diagnosis & Mixed In Key 11 Pro

### A. Stem Separation & SIP (System Integrity Protection)
- **Problem**: Enabling stem separation in Mixed In Key 11 Pro failed when macOS SIP was disabled.
- **Technical analysis**: MIK's stem separation engine relies on private or secured audio frameworks (notably CoreAudio tap and precompiled machine-learning libraries) signed with strict Apple entitlements (Hardened Runtime). When SIP is disabled, the AMFI (Apple Mobile File Integrity) security subsystem refuses to grant certain sandbox extensions and blocks the loading of the neural separation models.
- **Conclusion & Handling**: A detailed technical explanation was given to the user, and Crate's direct integration with the Mixed In Key SQLite database (`Collection11.mikdb`) was made reliable without depending on the features blocked by SIP.

### B. Fixing the Crashes in Album Mode
- Fixed the errors triggered when clicking a track in album mode: normalisation of the reactive state and robust handling of tracks outside the library.

### C. Removing the Platinum Notes Advertising Banner in MIK 11
- Cocoa reverse-engineering of `/Applications/Mixed In Key 11.app`: identification of the NSView classes and preference keys needed to permanently neutralise the promotional popup/banner at the bottom of the screen.

---

## 2. Evolution & Ergonomics of the Dedicated Player (Builds 36 to 40)

The `PlayerView.svelte` component went through a series of ergonomic iterations to turn Crate into a reference standalone audio player on macOS:

### Build 36: Default macOS Player & Liquid Glass Effect
- **System configuration**: Wrote and ran a Swift script calling `LSSetDefaultRoleHandlerForContentType` to associate the `com.crate.app` UTI with all audio formats (`public.mp3`, `com.apple.m4a-audio`, `public.wave-format`, `public.flac-audio`, `public.aiff-audio`, `org.xiph.ogg-audio`, etc.).
- **Visual hierarchy**: Player floating in the foreground (`z-20`, `backdrop-blur-2xl bg-surface-0/80`) with the list of recent files scrolling gently in the background under a thick blur.

### Build 37: Proportioned Layout in the Style of Apple Music for macOS
- **Compact hero**: Height limited to ~36% of the window, 170x170px square artwork, 36px cyan waveform.
- **Fixed full-width header**: The header bar of the recent files is fixed under the player and never scrolls with the tracks.

### Build 38: Streamlined Controls & Central Integration
- **Simplification**: Removed the pitch fader and the volume slider from the player hero in favour of a centred transport (`⏮`, `▶/⏸ Cyan`, `⏹`).
- **3-tab Segmented Control**: `Player` | `Library` | `Beatport`.
- **Selective behaviour at startup**: Starts directly on the *Library*, but switches automatically to the *Player* if an external audio file is opened (Finder double-click or "Open With").
- **Space shortcut**: Global support for the Space bar as Play/Pause on the Player page.

### Build 39: Breathing Room & Imposing Typography
- Artwork enlarged to **225x225px** with a `shadow-2xl shadow-black/60` drop shadow.
- Cyan waveform enlarged to **48px** (`h-12`) and an imposing **52px** Play/Pause button.
- Calibrated to show exactly ~6 recent tracks without scrolling.

### Build 40: Light Mode (White Mode) Contrast & Streamlined Finder Icon
- Action buttons ("Ajouter à ma bibliothèque" (Add to my library) and the Finder button as an icon only, `h-8 w-8`) moved to the top right, facing the row of metadata badges.
- Contrast adjusted for light mode (Ambient glow at 15%, soft borders, optimal legibility of the cyan counter and the waveform bars).

---

## 3. Beatport Quality Upgrader: MP3 ➔ FLAC Lossless Replacement (Builds 41 to 44)

This feature scans the local library, detects tracks encoded as MP3 and automatically replaces them with their official FLAC Lossless equivalent downloaded from Beatport.

### Technical Architecture
1. **Migration 11 & Rust Models**:
   - `ignored_upgrade_matches` table to remember the user's choices.
   - `UpgradeMatch`, `UpgradeScanResult`, `UpgradeReplacementResult` types.
2. **Cleaned-up Multi-Pass Search** ([`upgrader.rs`](../../src-tauri/src/services/beatport/upgrader.rs)):
   - Splitting and extraction of the main artist (removal of featurings `feat.`, `ft.`, `vs.`, `with`).
   - Title clean-up (removal of stray mix tags and dashes).
   - Query cascade: (Pass 1) Cleaned artist + Cleaned title $\rightarrow$ (Pass 2) Cleaned title alone $\rightarrow$ (Pass 3) Raw query.
   - Multi-criteria scoring algorithm: minimum confidence threshold of $75\%$.
3. **Eliminating CPU Overheating (Build 43)**:
   - Problem solved: the scanner ran in a continuous loop through a reactive effect, saturating the CPU at 98%.
   - Solution: Migration 12 (`upgrade_matches_cache`), instant reading of the counter from SQLite (< 2ms), concurrency limited by `tokio::sync::Mutex` and throttling in batches of 4 tracks.
4. **Audio Integrity & Corrupted Files (Build 44)**:
   - Problem solved: 19 tracks out of 66 had been downloaded as fake FLAC (MP3/AAC preview stream saved with the `.flac` extension, weighing ~1.4 MB), causing the *"Could not open..."* error in Mixed In Key.
   - Solution: Permanent removal of the preview fallback. Implementation of the strict integrity validator `validate_flac_file` (size $\ge 3$ MB and `b"fLaC"` magic bytes signature).
   - Absolute safety rule: the old MP3 must never be deleted until the new FLAC has been validated.
   - Clean-up and purge of the old corrupted files in the library and in `Collection11.mikdb`.

---

## 4. Crate Pulse & Advanced Music Statistics (Builds 45 to 48)

A complete music and DJ analytics dashboard, inspired by Stats.fm and Liquid Glass:

### Backend Architecture
- **Migration 13**: `listen_events`, `spotify_auth`, `rekordbox_sessions` tables.
- **Unified multiple sources**:
  - `spotify`: Real-time tracking via the Spotify Connect API.
  - `crate_local`: Local listens in Crate.
  - `crate_beatport`: Beatport pre-listens.
  - `rekordbox`: DJ mix sessions imported from Rekordbox.
  - `mixed_in_key`: macOS process tracker detecting direct listens in Mixed In Key 11 Pro.

### Strict Counting Rule (Minutes vs Streams)
- **Actual cumulative minutes**: Every second of listening is recorded and accumulated from the 1st second (`played_ms >= 1000`).
- **Stream validation**: A stream is only validated from 30 seconds of continuous listening (`played_ms >= 30000`).
- **Ranking filter**: SQL clause `HAVING SUM(CASE WHEN played_ms >= 30000 THEN 1 ELSE 0 END) >= 1` guaranteeing that no track or artist without at least one real stream can appear in the Tops.

### Reliable Spotify OAuth2 PKCE Integration (Builds 47-48)
- Fixed the redirect URI required by Spotify: strict use of the loopback IP `http://127.0.0.1:8888/callback` (`localhost` is rejected).
- Asynchronous local Tokio loopback server on port 8888 automatically capturing the authorisation code.
- Fixed the `invalid_grant` bug: eliminated the race condition on the PKCE `code_verifier` thanks to unique `state` pairing persisted in SQLite.
- Optional and recommended support for the `Client Secret` via Basic Auth.

### Svelte 5 Frontend Interface
- Hero KPI cards: Actual minutes, Tracks listened to, Distinct artists, DJ sessions.
- Coloured proportional multi-source gauge.
- Top Tracks & Top Artists with artworks and metadata.
- Camelot Harmonic Wheel & tempo distribution (BPM histogram).
- Interactive 24h $\times$ 7d weekly heatmap.

---

## 5. Performance Audit, i18n Crash Fix & MIK Watcher (Builds 49 to 56)

Following observed slowness (1-second delay on click, CPU overheating, search latency) and a critical crash at startup:

### A. Eliminating the Critical i18n Crash (Build 56)
- **Cause**: `svelte-i18n` loaded the `en.json` and `fr.json` dictionaries asynchronously. During the initial mounting of components using Svelte 5 runes, the templates called `$translate('nav.library')` before the promises resolved, causing the fatal error: `Cannot format a message without first setting the initial locale`.
- **Fix**: Direct synchronous static import of the `en.json` and `fr.json` dictionaries into memory (`addMessages`) in `shared/i18n/index.ts`, guaranteeing immediate availability from millisecond zero.

### B. Eliminating the Infinite Loop of the MIK Watcher (Build 56)
- **Cause**: The `Collection11.mikdb` watcher detected the file modification and launched synchronisation, which called a purge method writing to the MIK SQLite database... which updated the `mtime`, relaunching synchronisation indefinitely at 100% CPU.
- **Fix**: `sync_all_from_mik_db` converted into a pure read operation. Atomic refresh of the `last_synced_mtime` timestamp. Removal of the 6s frontend `setInterval`. CPU usage stabilised at ~1% at idle.

---

## 6. Major DJ Pro Overhaul & Complete Optimisation (Build 57)

Build 57 delivers all the recommendations of the architectural and ergonomic audit to raise Crate to the level of the best professional DJ tools:

### A. Waveform Decoupling & Ultra-Light Queries
- **Historical anomaly**: `get_tracks` serialised the large binary blob `t.waveform_data` for every track when loading the library, causing hundreds of megabytes of JSON IPC traffic and saturating RAM.
- **Solution**: `get_tracks` now returns `NULL as waveform_data`. The waveform is only loaded on demand when a track is played, through the dedicated `get_track_waveform(track_id)` command.

### B. SQLite FTS5 Full-Text Search Engine (Migration 16)
- Replacement of the sequential `LIKE '%...%'` queries with an FTS5 virtual table `tracks_fts`:
  ```sql
  CREATE VIRTUAL TABLE IF NOT EXISTS tracks_fts USING fts5(
      track_id UNINDEXED,
      title,
      artist,
      album,
      genre,
      label,
      content='tracks',
      content_rowid='rowid',
      tokenize='unicode61 remove_diacritics 2'
  );
  ```
- Automatic synchronisation triggers (`tracks_ai`, `tracks_ad`, `tracks_au`).
- Instant multi-word prefix queries (< 2ms).
- Chunking in batches of 400 in `fetch_tags_for_tracks`.

### C. Slaving to the CoreAudio Hardware Clock
- Updated `AudioCommand::GetState` in the Rust audio engine to compute and return the exact position derived from the audio samples actually consumed (`p.get_current_position_ms()`).
- Periodic frontend synchronisation eliminating any drift between the visual cursor and the perceived sound.

### D. Interactive Hot Cues 1 to 8 & Real Waveform
- **Waveform display**: Decoding of the binary blob into 64 proportional bars with a graceful fallback.
- **Hot Cue flags on the Waveform**: Amber vertical markers positioned exactly at `(position_ms / duration_ms) * 100%`, clickable for an immediate seek.
- **8 Hot Cue Pads**: Row of pads under the waveform showing the status, number and timestamp (`0:45`), clickable to jump to the cue instantly.

### E. One-Click Harmonic Mix Assistant
- `Mix Harmonique` (Harmonic Mix) button with a `sparkles` icon placed next to the Camelot Key badge (e.g. `8A`).
- On click: automatic computation of the harmonically compatible keys via `getHarmonicKeys` (same key, relative major/minor, $\pm 1$ clockwise step, $+2$ energy boost) and of the tempo tolerance ($\pm 4\%$ BPM).
- Automatically filters the library and switches to the Library view with a toast confirmation.

### F. DJ "Flight Deck" Keyboard Shortcuts
- **`Space`**: Instant Play / Pause.
- **Keys `1` to `8`**: Direct jump to Hot Cues 1 to 8.
- **`Shift + Right Arrow` / `Shift + Left Arrow`**: Quick 15s jump (32-beat / 8-bar musical phrase).

### G. Pioneer Rekordbox XML Export
- [`rekordbox_xml.rs`](../../src-tauri/src/services/export/rekordbox_xml.rs) module generating a standard-compliant `<DJ_PLAYLISTS Version="1.0.0">` XML file.
- Export of the collection, BPM tempo grids, Camelot keys and all Mixed In Key cue points as `<POSITION_MARK>` tags readable natively on CDJ decks and in Rekordbox.
- Dedicated export button added to the toolbar (`Toolbar.svelte`).

---

## 7. New Location & File Organisation

The project folder was moved in its entirety:

📁 **Main path:**
[`~/Coding Projects/crate`](../..)

🔗 **Backward-compatibility link:**
A symbolic link was set up from the old location (`~/.gemini/antigravity/scratch/crate` $\rightarrow$ `~/Coding Projects/crate`) so that no existing script or command is broken.

### Key Project Tree

```text
~/Coding Projects/crate/
├── apps/
│   └── desktop/                  # Desktop application (SvelteKit + Svelte 5 runes)
│       └── src/
│           ├── lib/
│           │   ├── components/   # UI components (player, library, stats, upgrader, duplicates)
│           │   ├── hooks/        # useKeyboardShortcuts, useAppSetup
│           │   └── stores/       # Reactive desktop stores (library, recentTracks, export)
│           └── routes/           # +layout.svelte (navigation, badges) & +page.svelte
├── shared/                       # Universal shared code
│   ├── api/                      # Tauri IPC wrappers (library, player, export, stats, upgrader)
│   ├── stores/                   # Shared stores (player, ui, settings, duplicate, upgrader)
│   ├── types/                    # TypeScript interfaces (Track, Cue, Stats, Upgrader, etc.)
│   └── utils/                    # Utilities (camelot, format, mikSync, sorting)
├── src-tauri/                    # Native Rust backend
│   ├── src/
│   │   ├── commands/             # IPC handlers exposed to the frontend
│   │   ├── db/                   # SQLite schema, Migrations 1 to 16 (FTS5)
│   │   ├── models/               # Rust data structures
│   │   └── services/             # Business logic (audio, library, beatport, export, stats)
│   ├── Cargo.toml
│   └── tauri.conf.json
└── package.json                  # Monorepo scripts (yarn build:production, yarn test)
```

---

## 8. Version Summary Table (Builds)

| Build | Date | Key Additions & Major Fixes | Test Status |
|---|---|---|---|
| **Build 36** | Early Sept | Default macOS player (Swift UTI script), Liquid Glass z-index layout | ✅ Validated |
| **Build 37** | Early Sept | Apple Music proportioned layout, 170x170px artwork, fixed recents header | ✅ Validated |
| **Build 38** | Early Sept | Centred transport, Player as central tab (3 tabs), selective opening | ✅ Validated |
| **Build 39** | Early Sept | Enlarged artwork (225px), waveform (48px) and transport (52px) | ✅ Validated |
| **Build 40** | Early Sept | Light mode (White Mode), buttons moved to top right, Finder icon | ✅ Validated |
| **Build 41** | Mid-Sept | Beatport Quality Upgrader (MP3 ➔ FLAC Lossless), comparison modal | ✅ Validated |
| **Build 42** | Mid-Sept | Beatport OAuth2 token auto-refresh, cleaned-up multi-pass search | ✅ Validated |
| **Build 43** | Mid-Sept | SQLite `upgrade_matches_cache` cache, end of the 98% CPU overheating | ✅ Validated |
| **Build 44** | Mid-Sept | FLAC integrity validator (`b"fLaC"`, $\ge 3$ MB), purge of corrupted files | ✅ Validated |
| **Build 45** | Mid-Sept | Crate Pulse & Stats: multi-source tracking, KPI cards, 24x7 Heatmap | ✅ Validated |
| **Build 46** | Mid-Sept | Strict counting rule (1s minutes vs 30s streams), MIK 11 Pro tracker | ✅ Validated |
| **Build 47** | Mid-Sept | Spotify Tokio loopback `127.0.0.1:8888/callback`, end of the `localhost` blocking | ✅ Validated |
| **Build 48** | Mid-Sept | Fixed Spotify PKCE race condition (`state` parameter), Client Secret | ✅ Validated |
| **Build 56** | 14 Sept | Synchronous i18n preloading (anti-crash), removal of the MIK watcher loop | 141 Vitest tests (100%) |
| **Build 57** | 14 Sept | **Major DJ Pro Overhaul**: FTS5 (Migration 16), On-demand Waveform, CoreAudio Clock, Hot Cues 1-8, 1-Click Harmonic Mix, Flight Deck Shortcuts, Rekordbox XML Export | **227 Rust (100%)<br>144 Vitest (100%)<br>0 svelte-check errors** |

---

*Document generated on 25 September 2026. The production build of the application is installed in `/Applications/Crate.app`.*
