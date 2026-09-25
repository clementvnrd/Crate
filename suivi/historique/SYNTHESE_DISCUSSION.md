# Synthèse Complète & Historique du Projet Crate

Ce document récapitule de manière exhaustive l'ensemble des travaux, décisions architecturales, résolutions de bugs, optimisations de performance et fonctionnalités développées au cours de cette discussion.

---

## Sommaire

1. [Origines, Diagnostic macOS & Mixed In Key 11 Pro](#1-origines-diagnostic-macos--mixed-in-key-11-pro)
2. [Évolution & Ergonomie du Player Dédié (Builds 36 à 40)](#2-évolution--ergonomie-du-player-dédié-builds-36-à-40)
3. [Beatport Quality Upgrader : Remplacement MP3 ➔ FLAC Lossless (Builds 41 à 44)](#3-beatport-quality-upgrader--remplacement-mp3--flac-lossless-builds-41-à-44)
4. [Crate Pulse & Statistiques Musicales Avancées (Builds 45 à 48)](#4-crate-pulse--statistiques-musicales-avancées-builds-45-à-48)
5. [Audit des Performances, Résolution du Crash i18n & Watcher MIK (Builds 49 à 56)](#5-audit-des-performances-résolution-du-crash-i18n--watcher-mik-builds-49-à-56)
6. [Refonte Majeure DJ Pro & Optimisation Complète (Build 57)](#6-refonte-majeure-dj-pro--optimisation-complète-build-57)
7. [Nouvel Emplacement & Organisation des Fichiers](#7-nouvel-emplacement--organisation-des-fichiers)
8. [Tableau Récapitulatif des Versions (Builds)](#8-tableau-récapitulatif-des-versions-builds)

---

## 1. Origines, Diagnostic macOS & Mixed In Key 11 Pro

### A. Séparation de Stems & SIP (System Integrity Protection)
- **Problématique** : L'activation de la séparation de stems dans Mixed In Key 11 Pro échouait lorsque le SIP de macOS était désactivé.
- **Analyse technique** : L'engine de séparation de stems de MIK fait appel à des frameworks audio privés ou sécurisés (notamment CoreAudio tap et des bibliothèques de machine learning précompilées) signés avec des entitlements Apple stricts (Hardened Runtime). Lorsque le SIP est désactivé, le sous-système de sécurité AMFI (Apple Mobile File Integrity) refuse d'accorder certaines extensions du sandbox et bloque le chargement des modèles de séparation neuronale.
- **Conclusion & Traitement** : Explication technique détaillée fournie à l'utilisateur, et fiabilisation de l'intégration directe de Crate avec la base de données SQLite de Mixed In Key (`Collection11.mikdb`) sans dépendre des fonctionnalités bloquées par le SIP.

### B. Correction des crashs en Mode Album
- Résolution des erreurs déclenchées lors du clic sur un morceau en mode album : normalisation de l'état réactif et gestion robuste des morceaux hors bibliothèque.

### C. Suppression du bandeau publicitaire Platinum Notes dans MIK 11
- Reverse-engineering Cocoa sur `/Applications/Mixed In Key 11.app` : identification des classes NSView et des clés de préférences pour neutraliser définitivement le popup/bandeau promotionnel au bas de l'écran.

---

## 2. Évolution & Ergonomie du Player Dédié (Builds 36 à 40)

Le composant `PlayerView.svelte` a fait l'objet d'une série d'itérations ergonomiques pour transformer Crate en lecteur audio autonome de référence sur macOS :

### Build 36 : Lecteur par Défaut macOS & Effet Liquid Glass
- **Configuration système** : Écriture et exécution d'un script Swift appelant `LSSetDefaultRoleHandlerForContentType` pour associer l'UTI `com.crate.app` à l'ensemble des formats audio (`public.mp3`, `com.apple.m4a-audio`, `public.wave-format`, `public.flac-audio`, `public.aiff-audio`, `org.xiph.ogg-audio`, etc.).
- **Hiérarchie visuelle** : Player en premier plan flottant (`z-20`, `backdrop-blur-2xl bg-surface-0/80`) avec liste des récents défilant délicatement en arrière-plan sous un flou épais.

### Build 37 : Layout Proportionné façon Apple Music macOS
- **Hero compact** : Hauteur limitée à ~36% de la fenêtre, artwork carrée de 170x170px, waveform cyan de 36px.
- **En-tête fixe pleine largeur** : La barre d'en-tête des fichiers récents est fixée sous le lecteur sans jamais défiler avec les pistes.

### Build 38 : Épuration des Contrôles & Intégration Centrale
- **Simplification** : Suppression du pitch fader et du volume slider du player hero au profit d'un transport centré (`⏮`, `▶/⏸ Cyan`, `⏹`).
- **Segmented Control 3 onglets** : `Player` | `Bibliothèque` | `Beatport`.
- **Comportement sélectif au démarrage** : Démarrage direct sur la *Bibliothèque*, mais bascule automatique sur le *Player* si un fichier audio externe est ouvert (double-clic Finder ou "Ouvrir avec").
- **Raccourci Espace** : Prise en charge globale de la barre Espace pour Play/Pause sur la page Player.

### Build 39 : Respiration & Typographie Imposante
- Artwork agrandie à **225x225px** avec ombre portée `shadow-2xl shadow-black/60`.
- Waveform cyan agrandie à **48px** (`h-12`) et bouton Play/Pause imposant de **52px**.
- Calibrage pour afficher exactement ~6 morceaux récents visibles sans scroll.

### Build 40 : Contrastes Mode Clair (White Mode) & Icône Finder Épurée
- Boutons d'action ("Ajouter à ma bibliothèque" et bouton Finder en icône seule `h-8 w-8`) repositionnés en haut à droite en face de la rangée de badges métadonnées.
- Adaptation des contrastes pour le mode clair (Ambient glow à 15%, bordures douces, lisibilité optimale du compteur cyan et des barres waveform).

---

## 3. Beatport Quality Upgrader : Remplacement MP3 ➔ FLAC Lossless (Builds 41 à 44)

Cette fonctionnalité permet de scanner la bibliothèque locale, de détecter les morceaux encodés en MP3 et de les remplacer automatiquement par leur équivalent FLAC Lossless officiel téléchargé depuis Beatport.

### Architecture Technique
1. **Migration 11 & Modèles Rust** :
   - Table `ignored_upgrade_matches` pour mémoriser les choix utilisateur.
   - Types `UpgradeMatch`, `UpgradeScanResult`, `UpgradeReplacementResult`.
2. **Recherche Multi-Passes Épurée** ([`upgrader.rs`](file:///Users/testuser/Coding%20Projects/crate/src-tauri/src/services/beatport/upgrader.rs)) :
   - Découpage et extraction de l'artiste principal (suppression des featurings `feat.`, `ft.`, `vs.`, `with`).
   - Épuration du titre (suppression des tags de mix parasites et tirets).
   - Cascade de requêtes : (Passe 1) Artiste épuré + Titre épuré $\rightarrow$ (Passe 2) Titre épuré seul $\rightarrow$ (Passe 3) Requête brute.
   - Algorithme de scoring multi-critères : seuil de confiance minimal de $75\%$.
3. **Élimination de la Surchauffe CPU (Build 43)** :
   - Problème résolu : le scanner tournait en boucle continue via un effect réactif, saturant le CPU à 98%.
   - Solution : Migration 12 (`upgrade_matches_cache`), lecture instantanée du compteur en SQLite (< 2ms), limitation de concurrence par `tokio::sync::Mutex` et throttling par lots de 4 pistes.
4. **Intégrité Audio & Fichiers Corrompus (Build 44)** :
   - Problème résolu : 19 morceaux sur 66 avaient été téléchargés en faux FLAC (flux preview MP3/AAC sauvegardé avec l'extension `.flac` pesant ~1.4 Mo), causant l'erreur *"Could not open..."* dans Mixed In Key.
   - Solution : Suppression définitive du fallback preview. Implémentation du validateur d'intégrité strict `validate_flac_file` (taille $\ge 3$ Mo et signature magic bytes `b"fLaC"`).
   - Règle de sécurité absolue : interdiction formelle de supprimer l'ancien MP3 tant que le nouveau FLAC n'est pas validé.
   - Nettoyage et purge des anciens fichiers corrompus dans la bibliothèque et dans `Collection11.mikdb`.

---

## 4. Crate Pulse & Statistiques Musicales Avancées (Builds 45 à 48)

Un dashboard complet d'analytics musicales et DJ, inspiré de Stats.fm et Liquid Glass :

### Architecture Backend
- **Migration 13** : Tables `listen_events`, `spotify_auth`, `rekordbox_sessions`.
- **Sources multiples unifiées** :
  - `spotify` : Tracking en temps réel via l'API Spotify Connect.
  - `crate_local` : Écoutes locales dans Crate.
  - `crate_beatport` : Pré-écoutes Beatport.
  - `rekordbox` : Sessions de mix DJ importées depuis Rekordbox.
  - `mixed_in_key` : Tracker de processus macOS détectant les écoutes directes dans Mixed In Key 11 Pro.

### Règle Stricte de Comptage (Minutes vs Streams)
- **Minutes réelles cumulées** : Toutes les secondes d'écoute sont enregistrées et accumulées dès la 1ère seconde (`played_ms >= 1000`).
- **Validation d'un Stream** : Un stream n'est validé qu'à partir de 30 secondes d'écoute continue (`played_ms >= 30000`).
- **Filtre des classements** : Clause SQL `HAVING SUM(CASE WHEN played_ms >= 30000 THEN 1 ELSE 0 END) >= 1` garantissant qu'aucun morceau ou artiste sans au moins un vrai stream ne peut apparaître dans les Tops.

### Intégration Spotify OAuth2 PKCE Fiabilisée (Builds 47-48)
- Correction de l'URI de redirection exigée par Spotify : stricte utilisation de l'IP loopback `http://127.0.0.1:8888/callback` (rejet de `localhost`).
- Serveur loopback local asynchrone Tokio sur le port 8888 capturant automatiquement le code d'autorisation.
- Résolution du bug `invalid_grant` : élimination de la race condition sur le `code_verifier` PKCE grâce à un appariement d'état unique `state` persisté en SQLite.
- Prise en charge optionnelle et recommandée du `Client Secret` en Basic Auth.

### Interface Frontend Svelte 5
- KPI cards héroïques : Minutes réelles, Titres écoutés, Artistes distincts, Sessions DJ.
- Jauge proportionnelle multi-sources colorée.
- Top Morceaux & Top Artistes avec artworks et métadonnées.
- Roue Harmonique Camelot & distribution du tempo (histogramme BPM).
- Heatmap hebdomadaire interactive 24h $\times$ 7j.

---

## 5. Audit des Performances, Résolution du Crash i18n & Watcher MIK (Builds 49 à 56)

Suite à des lenteurs constatées (délai d'1 seconde au clic, surchauffe CPU, latence de recherche) et à un crash critique au démarrage :

### A. Élimination du Crash Critique i18n (Build 56)
- **Cause** : `svelte-i18n` chargeait les dictionnaires `en.json` et `fr.json` de façon asynchrone. Lors du montage initial des composants avec runes Svelte 5, les templates appelaient `$translate('nav.library')` avant la résolution des promesses, provoquant l'erreur fatale : `Cannot format a message without first setting the initial locale`.
- **Correction** : Import statique synchrone direct des dictionnaires `en.json` et `fr.json` en mémoire vive (`addMessages`) dans `shared/i18n/index.ts`, garantissant une disponibilité immédiate dès la milliseconde zéro.

### B. Élimination de la Boucle Infinie du Watcher MIK (Build 56)
- **Cause** : Le watcher de `Collection11.mikdb` détectait la modification du fichier, lançait la synchronisation, qui appelait une méthode de purge écrivant dans la base SQLite MIK... ce qui mettait à jour le `mtime`, relançant indéfiniment la synchronisation à 100% de CPU.
- **Correction** : `sync_all_from_mik_db` converti en opération de lecture pure. Réactualisation atomique de l'horodatage `last_synced_mtime`. Suppression du `setInterval` frontend de 6s. Consommation CPU stabilisée à ~1% au repos.

---

## 6. Refonte Majeure DJ Pro & Optimisation Complète (Build 57)

Le Build 57 concrétise la totalité des préconisations de l'audit architectural et ergonomique pour hisser Crate au niveau des meilleurs outils DJ professionnels :

### A. Découplage de la Waveform & Requêtes Ultra-Légères
- **Anomalie historique** : `get_tracks` sérialisait le gros blob binaire `t.waveform_data` pour chaque morceau lors du chargement de la bibliothèque, provoquant un transit IPC JSON de centaines de mégaoctets et saturant la mémoire vive.
- **Solution** : `get_tracks` renvoie désormais `NULL as waveform_data`. La waveform n'est chargée qu'à la demande lors de la lecture d'un morceau via la commande dédiée `get_track_waveform(track_id)`.

### B. Moteur de Recherche Plein-Texte SQLite FTS5 (Migration 16)
- Remplacement des `LIKE '%...%'` séquentiels par une table virtuelle FTS5 `tracks_fts` :
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
- Triggers automatiques de synchronisation (`tracks_ai`, `tracks_ad`, `tracks_au`).
- Requêtes préfixées multi-mots instantanées (< 2ms).
- Chunking par lots de 400 dans `fetch_tags_for_tracks`.

### C. Asservissement sur l'Horloge Matérielle CoreAudio
- Mise à jour de `AudioCommand::GetState` dans le moteur audio Rust pour calculer et renvoyer la position exacte issue des échantillons audio réellement consommés (`p.get_current_position_ms()`).
- Synchronisation périodique frontend éliminant toute dérive entre le curseur visuel et le son perçu.

### D. Hot Cues 1 à 8 Interactifs & Waveform Réelle
- **Affichage Waveform** : Décodage du blob binaire en 64 barres proportionnelles avec fallback harmonieux.
- **Drapeaux Hot Cue sur la Waveform** : Marqueurs verticaux ambrés positionnés exactement à `(position_ms / duration_ms) * 100%`, cliquables pour seek immédiat.
- **8 Pads Hot Cue** : Rangée de pads sous la waveform affichant le statut, le numéro et l'horodatage (`0:45`), cliquables pour sauter au repère instantanément.

### E. Assistant de Mix Harmonique en 1 Clic
- Bouton `Mix Harmonique` avec icône `sparkles` positionné à côté du badge Camelot Key (ex: `8A`).
- Au clic : calcul automatique des tonalités harmoniquement compatibles via `getHarmonicKeys` (même clé, relative majeure/mineure, $\pm 1$ cran horaire, boost d'énergie $+2$) et de la tolérance tempo ($\pm 4\%$ BPM).
- Filtre automatiquement la bibliothèque et bascule sur la vue Bibliothèque avec confirmation toast.

### F. Raccourcis Clavier "Flight Deck" DJ
- **`Espace`** : Play / Pause instantané.
- **Touches `1` à `8`** : Saut direct aux Hot Cues 1 à 8.
- **`Shift + Flèche Droite` / `Shift + Flèche Gauche`** : Saut rapide de 15s (phrase musicale de 32 temps / 8 mesures).

### G. Exportation Pioneer Rekordbox XML
- Module [`rekordbox_xml.rs`](file:///Users/testuser/Coding%20Projects/crate/src-tauri/src/services/export/rekordbox_xml.rs) générant un fichier XML standard conforme `<DJ_PLAYLISTS Version="1.0.0">`.
- Export de la collection, des grilles de tempo BPM, des tonalités Camelot et de l'ensemble des cue points Mixed In Key sous forme de balises `<POSITION_MARK>` lisibles nativement sur platines CDJ et dans Rekordbox.
- Bouton d'exportation dédié ajouté dans la barre d'outils (`Toolbar.svelte`).

---

## 7. Nouvel Emplacement & Organisation des Fichiers

Le dossier du projet a été déplacé dans son intégralité :

📁 **Chemin principal :**
[`/Users/testuser/Coding Projects/crate`](file:///Users/testuser/Coding%20Projects/crate)

🔗 **Lien de rétrocompatibilité :**
Un lien symbolique a été configuré depuis l'ancien emplacement (`/Users/testuser/.gemini/antigravity/scratch/crate` $\rightarrow$ `/Users/testuser/Coding Projects/crate`) afin qu'aucun script ou commande existante ne soit rompu.

### Arborescence Clé du Projet

```text
/Users/testuser/Coding Projects/crate/
├── apps/
│   └── desktop/                  # Application Desktop (SvelteKit + Svelte 5 runes)
│       └── src/
│           ├── lib/
│           │   ├── components/   # Composants UI (player, library, stats, upgrader, duplicates)
│           │   ├── hooks/        # useKeyboardShortcuts, useAppSetup
│           │   └── stores/       # Stores desktop réactifs (library, recentTracks, export)
│           └── routes/           # +layout.svelte (navigation, badges) & +page.svelte
├── shared/                       # Code partagé universel
│   ├── api/                      # Wrappers IPC Tauri (library, player, export, stats, upgrader)
│   ├── stores/                   # Stores partagés (player, ui, settings, duplicate, upgrader)
│   ├── types/                    # Interfaces TypeScript (Track, Cue, Stats, Upgrader, etc.)
│   └── utils/                    # Utilitaires (camelot, format, mikSync, sorting)
├── src-tauri/                    # Backend natif Rust
│   ├── src/
│   │   ├── commands/             # Handlers IPC exposés au frontend
│   │   ├── db/                   # Schéma SQLite, Migrations 1 à 16 (FTS5)
│   │   ├── models/               # Structures de données Rust
│   │   └── services/             # Logique métier (audio, library, beatport, export, stats)
│   ├── Cargo.toml
│   └── tauri.conf.json
└── package.json                  # Scripts monorepo (yarn build:production, yarn test)
```

---

## 8. Tableau Récapitulatif des Versions (Builds)

| Build | Date | Apports Clés & Corrections Majeures | Statut Tests |
|---|---|---|---|
| **Build 36** | Début Sept | Lecteur par défaut macOS (script Swift UTIs), layout z-index Liquid Glass | ✅ Validé |
| **Build 37** | Début Sept | Layout proportionné Apple Music, artwork 170x170px, en-tête des récents fixe | ✅ Validé |
| **Build 38** | Début Sept | Centrage du transport, Player onglet central (3 onglets), ouverture sélective | ✅ Validé |
| **Build 39** | Début Sept | Agrandissement artwork (225px), waveform (48px) et transport (52px) | ✅ Validé |
| **Build 40** | Début Sept | Mode clair (White Mode), repositionnement boutons haut-droit, icône Finder | ✅ Validé |
| **Build 41** | Mi-Sept | Beatport Quality Upgrader (MP3 ➔ FLAC Lossless), modale comparative | ✅ Validé |
| **Build 42** | Mi-Sept | Auto-refresh token Beatport OAuth2, recherche multi-passes nettoyée | ✅ Validé |
| **Build 43** | Mi-Sept | Cache SQLite `upgrade_matches_cache`, fin de la surchauffe CPU 98% | ✅ Validé |
| **Build 44** | Mi-Sept | Validateur d'intégrité FLAC (`b"fLaC"`, $\ge 3$ Mo), purge fichiers corrompus | ✅ Validé |
| **Build 45** | Mi-Sept | Crate Pulse & Stats : tracking multi-sources, KPI cards, Heatmap 24x7 | ✅ Validé |
| **Build 46** | Mi-Sept | Règle de comptage stricte (1s minutes vs 30s streams), tracker MIK 11 Pro | ✅ Validé |
| **Build 47** | Mi-Sept | Spotify loopback Tokio `127.0.0.1:8888/callback`, fin du blocage `localhost` | ✅ Validé |
| **Build 48** | Mi-Sept | Résolution race condition PKCE Spotify (paramètre `state`), Client Secret | ✅ Validé |
| **Build 56** | 14 Sept | Préchargement synchrone i18n (anti-crash), suppression boucle watcher MIK | 141 tests Vitest (100%) |
| **Build 57** | 14 Sept | **Refonte Majeure DJ Pro** : FTS5 (Migration 16), Waveform à la demande, Horloge CoreAudio, Hot Cues 1-8, Mix Harmonique 1-Clic, Raccourcis Flight Deck, Export Rekordbox XML | **227 Rust (100%)<br>144 Vitest (100%)<br>0 erreur svelte-check** |

---

*Document généré le 25 septembre 2026. L'application compilée en production est installée dans `/Applications/Crate.app`.*
