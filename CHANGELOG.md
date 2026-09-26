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

#### Corrigé — upgrader Beatport

- **[C6]** L'upgrader et le panier Beatport ne « nettoient » plus le dossier de destination : `beatportdl` télécharge dans un dossier privé `.crate-download-…` créé pour l'occasion, seuls les FLAC validés en sortent, puis ce dossier (et lui seul) est supprimé. Pochettes, playlists `.m3u`, notes et sous-dossiers de ton dossier musique ne sont plus jamais touchés.
- **[C7]** Seuls les fichiers créés par le téléchargement en cours sont pris en compte (ils sont seuls dans le dossier privé) : un FLAC déjà présent ne peut plus être pris pour le nouveau, et un fichier existant n'est jamais écrasé (`Titre (1).flac`).
- **[C8]** Un upgrade MP3 → FLAC met à jour le titre **existant** (même identifiant) au lieu de le supprimer puis de le réimporter : cues, tags, playlists, note, couleur, compteur et historique d'écoute sont conservés. Le MP3 ne part à la corbeille qu'une fois la bibliothèque mise à jour ; un échec de mise à la corbeille est signalé.
- **[B22]** Validation FLAC par décodage complet (symphonia) : un fichier corrompu, tronqué (moins d'échantillons qu'annoncé) ou dont la durée ne correspond pas au titre Beatport (±5 s ou ±3 %) est refusé.
- **[B23]** Scoring : une version différente (radio edit, dub, instrumental…) ne peut plus remplacer un extended/original mix ; une durée inconnue donne un score neutre au lieu d'un faux accord.
- **[B24]** Découpage des artistes par mots entiers : « Daft Punk » n'est plus coupé en « Da » (séparateur `ft`), « Alex » garde son x ; le rapprochement d'artistes par inclusion ne compte plus que des mots entiers. Corrige aussi la détection de doublons.
- **[B25]** Les titres sans équivalent Beatport sont mémorisés 7 jours (plus de nouvelle recherche réseau à chaque ouverture) ; les réponses 429 de Beatport sont réessayées après 1, 2 puis 4 s ; une erreur réseau n'est jamais mémorisée comme « aucun résultat ».
- **[B26]** Le chemin du MP3 est relu dans la bibliothèque au lieu d'être fourni par le webview ; toutes les erreurs sont remontées.
- **[I3]** Progression de l'upgrade (`upgrade-progress`) affichée dans le bouton : « Mise à niveau 2/5 — Titre ».
- **[F14]** Le bouton de remplacement est désactivé tant que Beatport n'est pas connecté.
- Tests : 12 nouveaux tests (décodage d'un vrai FLAC de test, fichiers factices et tronqués, déplacement sans écrasement, dossier utilisateur jamais supprimé, remplacement en place qui garde cues/tags/playlists, découpage d'artistes, scoring).

#### Corrigé — Mixed In Key et bibliothèque

- **[B1]** La synchronisation Mixed In Key ne se relance plus en rafale : le watcher ignore le fichier `-shm` (modifié par toute lecture, y compris celle de Crate) et compare date + taille de la base et du WAL. Chaque passage ne réécrit plus que les titres qui changent réellement (clause de comparaison dans l'`UPDATE`), dans une seule transaction, et la recherche de pochettes ne porte plus que sur les titres ajoutés ou modifiés. Vérifié sur une copie de la vraie bibliothèque : 276 titres réécrits à chaque passage avant, 0 au second passage maintenant (~250 ms).
- **[B1]** Mixed In Key n'écrase plus le titre, l'artiste, l'album, le genre, le label ni l'année d'un titre existant : ces champs ne sont remplis que s'ils sont vides. BPM, tonalité et énergie restent pilotés par Mixed In Key.
- **[B3]** Les cues Mixed In Key ont un identifiant stable (`mik-<titre>-<n>`) et sont mis à jour sur place au lieu d'être supprimés et recréés à chaque synchro ; les cues créés dans Crate ne sont plus effacés. Les anciennes copies (identifiants aléatoires) sont converties une fois : 2 033 cues convertis sans perte ni doublon sur la copie de test.
- **[B4]** `get_track_cues` ne relit plus toute la base Mixed In Key à chaque lecture d'un titre sans cues : seuls les fichiers ouverts hors bibliothèque (lecteur autonome) y sont cherchés, et hors du verrou de la base Crate.
- **[B6]** Résolution des signets macOS sans interface ni montage de volume (un disque débranché ne déclenche plus de tentative de montage), libération des `CFError` ; `create_bookmark` réservé aux tests.
- **[B7]** L'empreinte (`file_hash`) est de nouveau enregistrée à l'import ; réimporter un fichier déjà présent ne plante plus (les cues sont rattachés au titre existant, qui garde son identifiant).
- **[B8]** Parseur Serato Markers2 recalé sur le format réel (index, position, couleur, nom) : les cues importés depuis les tags n'étaient pas au bon endroit. La couleur des cues est désormais lue.
- **[B9]** La colonne `energy` est incluse dans les sauvegardes et la synchronisation cloud (les anciennes sauvegardes restent lisibles).
- Tests : 7 nouveaux tests (synchro idempotente, métadonnées utilisateur conservées, cues stables et cues utilisateur gardés, réimport, parseur Serato sur une entrée construite selon le format).

#### Corrigé — statistiques d'écoute (Crate Pulse)

- **[C9]** Suppression de la « réparation » lancée à chaque démarrage, qui transformait toute écoute Spotify de moins de 30 s en écoute complète (minutes et streams gonflés). Les écoutes déjà modifiées ne peuvent pas être restaurées.
- **[C10]** Fin du double comptage Spotify : l'historique officiel « recently played » (toutes les minutes) est désormais la **seule** source ; le poller toutes les 4 s, qui enregistrait la même écoute une seconde fois (et une troisième après une pause), est supprimé. Le temps écouté est la durée du titre, plafonnée par l'écart avec l'écoute précédente (un titre passé au bout de 50 s compte 50 s).
- **[B12]** Lecteur Crate : seul le temps de lecture réel est compté (pauses et sauts dans le morceau exclus, arrêt en fin de piste détecté) ; l'écoute enregistrée à 30 s est mise à jour avec la durée totale écoutée au changement de titre.
- **[B11]** Le suivi des écoutes dans Mixed In Key (déduit d'un fichier ouvert, analyse comprise) est **désactivé par défaut** et activable dans Crate Pulse avec un avertissement.
- **[B13]** Import XML Rekordbox : seuls les playlists d'historique datées (« HISTORY 2026-09-20 ») deviennent des écoutes, datées de la session et dans l'ordre joué — avant, **toute la collection** exportée était comptée comme écoutée le jour de l'import. Une session n'est importée qu'une fois ; les écoutes sans heure réelle sont exclues de la heatmap. Côté `master.db`, les compteurs de session ne gonflent plus à chaque synchro.
- **[B14]** Filtres « 7 jours » et « 30 jours » : les dates sont normalisées (`datetime()`) avant comparaison, quel que soit leur format ou fuseau ; une date illisible ne fait plus échouer la heatmap.
- **[B15]** Import de l'historique JSON Spotify : une seule transaction, dates `2024-03-01 20:15` et ISO 8601 gérées, une date mal formée est ignorée au lieu de faire planter l'import.
- **[B16]** Plus d'inversion d'ordre de verrous entre la déconnexion Spotify et le poller (risque de blocage de toute la base) : le poller n'existe plus.
- **[B17]** Rafraîchissement du jeton Spotify sérialisé (un seul à la fois) ; un échec sur un jeton expiré est signalé au lieu de renvoyer silencieusement l'ancien jeton.
- **[B18]** Le serveur OAuth `127.0.0.1:8888` ne tourne plus en permanence : il démarre à la connexion et s'arrête après succès ou 10 minutes ; il exige un `state` généré par Crate ; les paramètres affichés dans la page de retour sont échappés (page de retour ramenée de ~330 à ~40 lignes).
- **[B34]** Le service Mixed In Key appelé par l'interface est la même instance que le worker de fond.
- Tests : 11 nouveaux tests (pauses, durée totale mise à jour, fin de piste, temps écouté plafonné, dates d'export, échappement HTML, heatmap tolérante, filtre 7 jours avec fuseau, import par lot, import XML Rekordbox idempotent).

#### Corrigé — fonctions DJ

- **[C11]** Hot cues : numérotation unique en base 0 (A = 0 … H = 7) comme l'amont et Rekordbox, dans la synchro Mixed In Key, le parseur Serato et la lecture des fichiers externes. Côté interface, un utilitaire partagé (`shared/utils/cues.ts`) associe pad ↔ cue : la touche 3 déclenche le cue 3 (et non plus le 2), un cue mémoire n'occupe plus de pad, les repères de la waveform affichent 1–8 pour les hot cues et « M » pour les cues mémoire. Les cues déjà en base sont convertis automatiquement à la prochaine synchro.
- **[C12]** Export Rekordbox XML valide : chemins `Location` entièrement percent-encodés en UTF-8 (`&`, `#`, `%`, espaces, accents) puis échappés, cues mémoire exportés avec `Num="-1"` (et non plus comme hot cue A), boucles en `Type="4"` avec `End`, couleur des cues reprise, plus de BPM (120) ni de débit (320) inventés pour les valeurs inconnues.
- **[B27]** Waveform réelle : les pics sont calculés depuis le fichier audio au premier affichage (symphonia, 400 barres, ~90 ms pour un MP3 de 2,4 Mo) puis mis en cache ; un fichier illisible affiche une ligne neutre au lieu d'un faux motif, et la waveform du titre précédent n'est plus conservée.
- **[B28]** Recherche : la requête FTS5 est découpée comme l'index (« You'll » → `"You"* "ll"*`) ; les titres avec apostrophe, tiret ou slash (« You'll », « Jay-Z », « AC/DC ») sont de nouveau trouvés, et `AND`, `OR`, `NOT` tapés en majuscules ne font plus échouer la liste.
- **[B29]** Position de lecture : position exacte de rodio (`Sink::get_pos`) à vitesse normale ; en fin de piste la position n'est plus figée sur une valeur ancienne.
- **[C13]** `loadTracks()` retrouve le comportement de l'amont (sans argument : tous les titres, filtre remis à zéro) : retirer le dernier tag vide de nouveau le filtre et le filtre « Mix harmonique » ne reste plus collé. Les rafraîchissements en arrière-plan ajoutés par le fork (synchro Mixed In Key, doublons, upgrader, lecteur) utilisent la nouvelle méthode explicite `reloadWithCurrentFilter()`.
- **[C14]** Glisser un tag sur un titre fonctionne de nouveau (`data-track-id` et surbrillance de survol restaurés sur les lignes).
- **[C15]** Jamais deux sons à la fois : une préécoute (découverte ou Beatport) arrête le moteur audio natif aussi quand il lit un fichier du lecteur autonome (seules les pistes de bibliothèque étaient arrêtées).
- **[F1]** Les touches 1 à 8 (hot cues) ne sont actives que dans les vues Player et Bibliothèque, avec un morceau chargé.
- **[F2]** Espace dans la vue Player met en pause ce qui joue (y compris une préécoute) au lieu de lancer un fichier récent.
- **[F3]** Espace/Entrée sur une ligne de la bibliothèque ne déclenche plus en même temps le raccourci global.
- **[F11]** Cues et waveform chargés pour un fichier ouvert hors bibliothèque (recherche par chemin au lieu d'un identifiant inexistant).
- **[B30]** Associations de fichiers audio en rang `Alternate` sans le parapluie `public.audio` : Crate apparaît dans « Ouvrir avec » sans s'imposer comme lecteur par défaut. Suppression de la commande `set_as_default_audio_player` (jamais appelée, visant un bundle `com.crate.app` inexistant) et du script `scripts/set_default_player.swift`.
- **[B31]** Fichiers ouverts avec Crate au démarrage : file d'attente côté Rust vidée une seule fois par l'interface une fois prête (`take_startup_files`) ; un fichier n'est plus ouvert deux fois et plusieurs fichiers ouverts ensemble sont tous pris en compte.
- **[B32]** Suppression de titres avec leurs fichiers : mise à la corbeille via Finder (chemin passé en argument, jamais interpolé dans le script), un titre ne quitte la bibliothèque que si son fichier est bien parti à la corbeille, les échecs sont signalés, et plus aucune suppression définitive hors macOS (module `services/trash.rs`, aussi utilisé par l'upgrader).
- **[B35]** Le compteur de doublons de la barre d'outils est mis en cache selon une empreinte de la bibliothèque : le scan complet ne tourne plus à chaque événement `duplicates-updated`, seulement quand la bibliothèque a changé.
- Tests : 9 nouveaux tests Rust (recherche, encodage des chemins, marques de cue, XML valide avec caractères spéciaux, pics de waveform) et 5 tests Vitest (pads de hot cues, raccourcis 1–8 limités aux bonnes vues).

#### Corrigé — frontend

- **[I1]** Les messages d'erreur du backend s'affichent enfin : Tauri rejette une commande avec une *chaîne*, que tous les `error instanceof Error ? … : 'message générique'` jetaient. Nouveau helper partagé `toErrorMessage()` utilisé par tous les stores et composants (~85 occurrences, amont compris).
- **[I2]** Onglet Beatport : les réglages sans effet (qualité AAC/MP3, synchro Mixed In Key automatique) sont remplacés par une information exacte — téléchargements FLAC vérifiés uniquement, analyse Mixed In Key récupérée automatiquement par la surveillance de sa base.
- **[I7]** L'événement `library-updated` (émis après un upgrade) est écouté et rafraîchit la bibliothèque ; paramètre `searchType` inutilisé retiré de la recherche Beatport.
- **[F4]** Initialisation : une étape qui dépasse son délai ne perd plus sa fonction de nettoyage (écouteurs de menu, touches média, initialisation) ; elle est exécutée à la fermeture, et les échecs sont journalisés avec le nom de l'étape.
- **[F5]** L'écran de démarrage se ferme quand l'initialisation (réglages compris) est terminée, et non plus après un minuteur fixe de 1,5 s qui pouvait faire apparaître l'onboarding par erreur ; filet de sécurité à 10 s.
- **[F7]** Le badge « doublons » de la barre d'outils ne relance le comptage que lorsque le nombre de titres change, plus à chaque modification de la liste.
- **[F10]** Ajout d'un fichier du lecteur à la bibliothèque : le store source est mis à jour au lieu de modifier une valeur dérivée.
- **[F13]** « Synchroniser avec Mixed In Key » dans le menu contextuel ne synchronise que les titres sélectionnés.
- **[F15]** Les infobulles se ferment au clic, à la sortie du pointeur et quand la fenêtre perd le focus (celle du badge Mixed In Key restait affichée quand le bouton se désactivait pendant la synchro).
- **[F8]** Panier Beatport : téléchargement titre par titre avec progression (« 2/5 : Titre ») ; seuls les titres réellement téléchargés quittent le panier, les échecs y restent avec leur raison. Les favoris et playlists Beatport, uniquement locaux, sont annoncés comme tels (plus de faux « succès »).
- **[F9]** Plus d'effet de bord à l'import des modules : le minuteur de rafraîchissement du jeton Beatport démarre avec la restauration explicite de la session au lancement.
- **[F12]** Position de lecture : une réponse du backend demandée avant un saut, un changement de titre ou un arrêt est ignorée (elle faisait reculer la tête de lecture).
- **[I4]** `BeatportAuthState` n'a plus qu'un schéma, celui du backend (snake_case) ; les champs camelCase en double sont supprimés partout.
- **[I8]** Les champs optionnels des types Beatport acceptent `null`, la valeur réellement envoyée par le backend (deux accès non protégés corrigés au passage).
- **[I9]** Les options d'affichage de la bibliothèque (colonnes, zéro Camelot…) sont enregistrées dans la base (donc sauvegardées et restaurées avec elle) ; le `localStorage` ne sert plus que de cache au démarrage.
- Tests : 3 tests Vitest pour `toErrorMessage`.

#### Outillage et qualité

- **[Q4]** Rust : `cargo fmt` appliqué (40 fichiers) et `cargo clippy --features desktop -- -D warnings` passe (37 erreurs → 0) : code mort supprimé (`ListenSource`, `set_track_rating`, `set_track_color`, `extract_bpm/key`, `detect_rekordbox_dir`, champs de réponse Spotify inutilisés, réexports inutiles), itérations et tris simplifiés.
- **[Q5]** TypeScript/Svelte : Prettier appliqué et ESLint à 0 erreur (38 → 0) : clés sur toutes les boucles `{#each}`, plus de `any` explicite, `$derived` modifiable dans la barre de recherche, caches non réactifs documentés.
- **[Q6]** CI du fork sur Linux : un job frontend (format, lint, types, Vitest) et un job Rust (clippy, tests) à chaque push ; le job Rust vérifie aussi la compilation hors macOS.
- **[Q3]** `RunEvent::Opened` limité aux plateformes qui le fournissent (macOS, iOS, Android) : le code compile de nouveau sous Windows/Linux.
- **[Q8]** Dépendances de test épinglées (Vitest 4.1.11, testing-library, jsdom), une seule version de Vite (7.3.0, via `resolutions`) pour l'app et les tests, fournisseur de couverture `@vitest/coverage-v8` ajouté (`yarn test:coverage`).
- **[Q13]** `yarn dev` compile l'app en debug (reconstructions rapides) avec des dépendances optimisées (`[profile.dev.package."*"]`) pour garder un décodage audio fluide.
- **[B10]** Migrations : libellés alignés sur leur position réelle (6 à 15) et règle « on ajoute, on ne renumérote jamais » documentée ; l'ordre n'est pas modifié car la base locale les a déjà appliquées.
- **[Q11]** Script `set_default_player.swift` supprimé ; la synthèse de l'assistant précédent est archivée dans `suivi/historique/`.
- **[Q10]** Documentation : page des raccourcis clavier corrigée (flèches ±10 s, Cmd+flèches ±1 s, Shift+flèches selon la vue, hot cues 1–8, Shift+Tab).
- **[Q7]** Chemins à risque couverts par des tests sur base et dossiers temporaires : remplacement de fichier, synchro Mixed In Key, trackers d'écoute, import, export, corbeille ; plus aucun test ne lit la vraie base Mixed In Key.
- **[Q12]** `CLAUDE.md` en place depuis le début du fork (règles de suivi et règles techniques).

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
