# Changelog

All notable changes to Crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fork personnel — journal des modifications

> Journal tenu en direct depuis l'audit du 25 septembre 2026. Chaque entrée cite l'identifiant du défaut corrigé (voir [suivi/REGISTRE-DEFAUTS.md](suivi/REGISTRE-DEFAUTS.md)) ; l'avancement global est dans [suivi/AVANCEMENT.md](suivi/AVANCEMENT.md). Les entrées les plus récentes sont en haut de chaque rubrique.

#### Sécurité

- **[C2]** Retrait de l'identifiant et du mot de passe Beatport écrits en dur dans `services/beatport/downloader.rs` et `client.rs`, **avant le premier commit** : le secret n'est jamais entré dans l'historique git. La configuration `beatportdl` conserve désormais les lignes `username`/`password` déjà présentes dans le fichier de l'utilisateur (sinon `beatportdl` utilise les jetons OAuth écrits à la connexion), et le fichier est écrit avec les droits `600`. Deux tests couvrent ces cas. _Le mot de passe lui-même doit encore être changé par le propriétaire._
- **[B19]** La session Beatport (jetons OAuth) est stockée dans le **Trousseau macOS** au lieu d'un fichier JSON en clair (`~/.config/crate/beatport_auth.json`) et du `localStorage` du webview. L'ancien fichier est migré puis supprimé au premier lancement, l'ancienne copie `localStorage` est effacée. Seul `beatportdl-credentials.json`, indispensable à `beatportdl`, reste sur disque : droits `600`, supprimé à la déconnexion.
- **[B20]** Suppression de la récupération du jeton Beatport dans la configuration locale de DJ.Studio (commande `beatport_auto_detect_session`).
- **[B21]** Un jeton Beatport collé à la main n'est plus accepté sans vérification : il doit être validé par l'API compte de Beatport.
- **[I6]** Suppression des commandes Spotify en double (`spotify_set_client_id`, `spotify_set_client_secret`) ; le secret client Spotify n'est plus jamais renvoyé au webview (`spotify_has_client_secret` indique seulement s'il existe, le champ reste vide pour le conserver).
- **[I5]** Suppression de la commande `record_listen_event`, jamais appelée, qui permettait au webview d'injecter des écoutes arbitraires dans les statistiques.
- **[C5]** Suppression des tests `test_sync_and_deduplicate_real_db` et `test_prune_missing_tracks_from_mik_db_real`, qui ouvraient la vraie base Crate (avec sa clé) et la vraie base Mixed In Key et y écrivaient à chaque `cargo test`.

#### Corrigé — protection des données

- **[C3]** La synchronisation Mixed In Key ne supprime plus les titres Crate absents de Mixed In Key (« purge stricte » retirée) : Mixed In Key enrichit la bibliothèque, il n'en décide plus le contenu. Un volume démonté ou un titre non analysé ne fait plus disparaître de titres, de tags ni de playlists.
- **[C4]** Crate n'ouvre plus jamais `Collection11.mikdb` en écriture : suppression des purges en cascade (`purge_tracks_by_pks`, `purge_tracks_by_path`, `prune_missing_tracks_from_mik_db`, `wal_checkpoint(TRUNCATE)`), y compris lors de la suppression d'un titre et après un upgrade FLAC.
- **[B5]** Plus de suppression au démarrage des titres dont le fichier a disparu (un dossier renommé suffisait à les effacer). La commande reste disponible manuellement.
- **[B2]** Le rattachement d'un titre Mixed In Key par titre et artiste n'a lieu que s'il existe une seule correspondance dont le fichier a disparu (fichier déplacé) ; il ne supprime plus les autres titres homonymes (version originale, extended…). Les doublons d'un même fichier transfèrent leurs tags et playlists avant d'être fusionnés.
- **[F6]** Suppression de la synchronisation Mixed In Key complète et du rechargement de la bibliothèque à chaque retour sur la fenêtre : la synchro backend (démarrage + surveillance du fichier) suffit.
- **[Q1]** `tauri.prod.conf.json` restauré comme en amont (artefacts de mise à jour) ; le build personnel sans signature passe par `yarn build:local` et `tauri.local.conf.json`.
- Tests : trois nouveaux tests de synchronisation sur base en mémoire et fichiers temporaires (titre absent conservé, homonymes jamais fusionnés, fichier déplacé qui garde son identité) ; le test de nettoyage n'utilise plus un chemin personnel.

#### Documentation

- **[L6]** Le README annonce désormais les 15 langues réellement livrées (au lieu de 11) et décrit le fork, les tests et le suivi.

#### Outillage et suivi

- Dépôt GitHub personnel privé, documents de suivi dans `suivi/` (avancement, registre, rapport, historique), `CLAUDE.md`, script `yarn suivi` qui recalcule la progression.
- Workflows amont (`ci.build`, `ci.lint`, `cd.docs`) passés en déclenchement manuel : ils lançaient des builds macOS et Windows à chaque push, coûteux sur un dépôt privé et encore en échec (voir Q2 à Q5). Nouveau workflow `ci.fork.yml` : Vitest à chaque push ; `yarn test` lance d'abord `svelte-kit sync` pour fonctionner sur un clone neuf.

#### Travail du fork antérieur à l'audit (builds 36 à 57)

Commité tel quel dans un instantané unique pour ne plus risquer de le perdre (**[C1]**) ; les défauts connus de ce code sont listés dans le registre.

- Lecteur autonome et association des fichiers audio macOS (vue Player, fichiers récents)
- Intégration Mixed In Key 11 : lecture de `Collection11.mikdb`, cues, énergie, watcher
- Beatport Quality Upgrader : recherche, scoring, téléchargement FLAC via `beatportdl`
- Crate Pulse : statistiques d'écoute multi-sources (Spotify, lecteur local, Mixed In Key, Rekordbox)
- Duplicate Killer, vue albums, recherche plein texte FTS5, hot cues 1 à 8, mix harmonique, raccourcis DJ, export Rekordbox XML
- Vitest, testing-library et jsdom ; 144 tests TypeScript

### Amont (blackboxaudio)

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
