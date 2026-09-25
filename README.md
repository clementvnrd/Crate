# Crate

[![CI fork](https://github.com/clementvnrd/Crate/actions/workflows/ci.fork.yml/badge.svg)](https://github.com/clementvnrd/Crate/actions/workflows/ci.fork.yml)

> Cross-platform DJ library manager with music discovery, track analysis, and USB export

<p align="center">
  <img src="assets/screenshot.png" alt="Crate — Discovery view showing label browsing with release preview playback" width="800" />
</p>

---

## 🧰 Fork personnel

Ce dépôt est un **fork personnel et privé** de [blackboxaudio/crate](https://github.com/blackboxaudio/crate) (remote `upstream`), utilisé sur un seul Mac avec Mixed In Key 11, Rekordbox 7 et Spotify. Il ajoute au socle amont :

- **Player** : lecteur autonome, fichiers récents, association des fichiers audio macOS, hot cues 1 à 8, mix harmonique, raccourcis DJ
- **Mixed In Key 11** : lecture de la base `Collection11.mikdb` (cues, énergie, tonalités)
- **Crate Pulse** : statistiques d'écoute multi-sources (Spotify, lecteur local, Mixed In Key, sessions Rekordbox)
- **Duplicate Killer**, vue albums, recherche plein texte FTS5, export Rekordbox XML
- **Beatport Quality Upgrader** : remplacement MP3 → FLAC via l'outil tiers `beatportdl`

> ⚠️ **État actuel : en cours de remise en état.** Un audit complet (25 septembre 2026) a relevé ~180 défauts, dont 15 critiques pouvant toucher aux données. Tant que les étapes 1 à 7 du plan ne sont pas cochées, garder des sauvegardes de `crate.db`, `db.key` et `Collection11.mikdb`.

| Document | Contenu |
| --- | --- |
| [suivi/AVANCEMENT.md](suivi/AVANCEMENT.md) | Progression, cases à cocher par défaut, actions du propriétaire |
| [CHANGELOG.md](CHANGELOG.md) | Journal détaillé des modifications |
| [suivi/REGISTRE-DEFAUTS.md](suivi/REGISTRE-DEFAUTS.md) | Registre des défauts et correctifs prévus |
| [suivi/RAPPORT-AUDIT.md](suivi/RAPPORT-AUDIT.md) | Rapport d'audit, vision et plan par phases |
| [CLAUDE.md](CLAUDE.md) | Règles de travail pour les assistants de code |

Le téléchargement Beatport passe par `beatportdl`, qui ne respecte pas les conditions d'utilisation de Beatport : cette partie ne doit jamais être publiée sur un dépôt public.

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
- **Localization** - Available in 15 languages (EN, FR, DE, ES, IT, JA, KO, NL, PL, PT, RO, SV, TR, UK, ZH)
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

Les tests n'utilisent que des bases temporaires : ils ne touchent jamais la bibliothèque réelle.

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

## 🔗 Links

- [Projet amont](https://github.com/blackboxaudio/crate) · [Site officiel](https://crate.bbx-audio.com)
- [Suivi du fork](suivi/README.md)

## ⚠️ Disclaimer

Crate's discovery features interact with third-party music services for browsing and previewing content. These features are intended for personal use only. Users are responsible for ensuring their usage complies with applicable terms of service.

## 📄 License

This project is source-available under the [PolyForm Shield License 1.0.0](LICENSE). You can read, learn from, and contribute to the code, but you can't use it to build a competing product.
