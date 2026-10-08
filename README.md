# Crate

[![CI fork](https://github.com/clementvnrd/Crate/actions/workflows/ci.fork.yml/badge.svg)](https://github.com/clementvnrd/Crate/actions/workflows/ci.fork.yml)

> Cross-platform DJ library manager with music discovery, track analysis, and USB export

<p align="center">
  <img src="assets/screenshot.png" alt="Crate — Discovery view showing label browsing with release preview playback" width="800" />
</p>

---

## 🧰 Personal fork

This repository is a **personal, private fork** of [blackboxaudio/crate](https://github.com/blackboxaudio/crate) (remote `upstream`), used on a single Mac with Mixed In Key 11, Rekordbox 7 and Spotify. It adds to the upstream base:

- **Player**: standalone player, recent files, macOS audio file association, hot cues 1 to 8, harmonic mix, DJ shortcuts
- **Mixed In Key 11**: reading the `Collection11.mikdb` database (cues, energy, keys)
- **Crate Pulse**: multi-source listening statistics (Spotify, local player, Mixed In Key, Rekordbox sessions)
- **Duplicate Killer**, albums view, FTS5 full-text search, Rekordbox XML export
- **Beatport Quality Upgrader**: MP3 → FLAC replacement via the third-party tool `beatportdl`
- **Discrepancy report**: read-only comparison of Crate against Mixed In Key and a Rekordbox XML export (key, tempo, energy, cues, missing files, tracks only on one side)

> ⚠️ **Current state: being repaired.** A full audit (25 September 2026) found ~180 defects, 15 of them critical and able to affect data. Until steps 1 to 7 of the plan are ticked, keep backups of `crate.db`, `db.key` and `Collection11.mikdb`.

| Document | Contents |
| --- | --- |
| [tracking/STATUS.md](tracking/STATUS.md) | Progress, checkboxes per defect, actions for the owner |
| [CHANGELOG.md](CHANGELOG.md) | Detailed change log |
| [tracking/DEFECTS.md](tracking/DEFECTS.md) | Defect register and planned fixes |
| [tracking/AUDIT-REPORT.md](tracking/AUDIT-REPORT.md) | Audit report, vision and phased plan |
| [DESIGN.md](DESIGN.md) | Design system: tokens, components, visual rules |
| [CLAUDE.md](CLAUDE.md) | Working rules for coding assistants (`design` agent in `.claude/`) |

Beatport downloading goes through `beatportdl`, which does not comply with Beatport's terms of use: this part must never be published in a public repository.

---

## 🎧 Overview

Crate is a cross-platform desktop application for managing DJ audio libraries. It handles everything from discovery, organization, analysis, and USB export.

## 🎵 Features

- **Library management** - Import and organize audio files (MP3, FLAC, WAV, AIFF, M4A/AAC) with automatic metadata extraction, search, and filtering
- **Tagging** - Categorize tracks with a flexible tag system for fast filtering
- **Playlists** - Build playlists manually or with smart rules
- **Track analysis** - Waveform generation, key detection, BPM analysis, and energy profiling
- **Music discovery** - Browse and preview releases from Bandcamp, SoundCloud, YouTube, and Discogs
- **Audio playback** - Preview tracks with waveform display and cue point management
- **USB export** - Export to Pioneer CDJ/XDJ devices with full Rekordbox database generation
- **Device sync** - Detect connected USB devices and sync library changes incrementally
- **Metadata editing** - Edit track metadata in bulk or individually
- **Customization** - Themes, accent colors, and font preferences
- **Localization** - English and French are complete. The other 13 languages (DE, ES, IT, JA, KO, NL, PL, PT, RO, SV, TR, UK, ZH) are partial and fall back to English for any text not yet translated, so the interface never shows a raw key
- **Auto-updates** - Stay on the latest version with minimal effort

## 🚀 Getting Started

### Prerequisites

- **Node.js** 22.x or higher
- **Yarn** (package manager)
- **Rust** nightly toolchain

For Tauri development dependencies, see the [Tauri v2 prerequisites guide](https://v2.tauri.app/start/prerequisites/).

#### Windows

Windows requires [Strawberry Perl](https://strawberryperl.com/) to build SQLCipher with OpenSSL:

```powershell
choco install strawberryperl
```

Or download the installer from https://strawberryperl.com/. Restart your terminal after installation.

### Installation

Clone the repository:

```bash
git clone https://github.com/clementvnrd/Crate.git
cd Crate
git remote add upstream https://github.com/blackboxaudio/crate.git
```

Install dependencies:

```bash
yarn install
```

### Tests

```bash
yarn test                                   # Vitest (TypeScript)
cd src-tauri && cargo test --features desktop   # Rust
```

The tests only use temporary databases: they never touch the real library.

```bash
yarn design:scan                            # deviations from the DESIGN.md rules (tokens, radii, a11y, i18n…)
yarn design:scan apps/desktop/src/lib/components/stats --details
```

### Development

Start the development server with hot reload:

```bash
yarn dev
```

This launches both the Vite dev server (port 1420) and the Tauri application window.

### Building

Build for production:

```bash
yarn build
```

Build a personal, unsigned `.app` (no updater artifacts, no DMG):

```bash
yarn build:local
```

Build for staging (with devtools):

```bash
yarn build:staging
```

Output binaries are placed in `src-tauri/target/release/bundle/`.

Platform targets:
- **macOS** - `.dmg`, `.app`
- **Windows** - `.msi`, `.exe`

### Releases and updates

The fork is released for Apple Silicon Macs only, as signed builds published in the public repository [`clementvnrd/crate-releases`](https://github.com/clementvnrd/crate-releases) (the code stays private). The app checks that repository for updates and installs only builds signed with the fork's own key. From the repository root:

```bash
yarn bump minor staging                   # 0.2.9 -> 0.3.0-staging.1
yarn changelog:prepare 0.3.0-staging.1    # moves the [Unreleased] notes under the version
yarn release:local --publish              # build, sign and publish from this Mac
```

**First install:** the app is not notarised (no Apple Developer certificate), so the first launch goes through **System Settings → Privacy & Security → Open Anyway**. Later updates install by themselves.

The full procedure, the signing-key rules and the rollback plan are in [docs/RELEASING.md](docs/RELEASING.md).

## 🔗 Links

- [Upstream project](https://github.com/blackboxaudio/crate) · [Official website](https://crate.bbx-audio.com)
- [Fork tracking](tracking/README.md)

## ⚠️ Disclaimer

Crate's discovery features interact with third-party music services for browsing and previewing content. These features are intended for personal use only. Users are responsible for ensuring their usage complies with applicable terms of service.

## 📄 License

This project is source-available under the [PolyForm Shield License 1.0.0](LICENSE). You can read, learn from, and contribute to the code, but you can't use it to build a competing product.
