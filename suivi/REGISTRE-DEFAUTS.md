> **Instantané figé de l’audit du 25 septembre 2026.** Chaque défaut garde son identifiant (C1, B12, D4…) : c’est lui qu’on retrouve dans [AVANCEMENT.md](AVANCEMENT.md), dans le [CHANGELOG](../CHANGELOG.md) et dans les messages de commit (`git log --grep "\[C2\]"`). Version éditable : [Crate — Registre des défauts & correctifs](https://claude.ai/code/artifact/5f3fd54e-ebb9-457f-bc3a-b584ad01a41c).

# Crate — Registre des défauts & correctifs


## Comment lire ce registre

Ce registre liste les défauts de Crate, classés par gravité puis par domaine, chacun avec son correctif et son effort. Il complète le rapport « Crate — Audit complet & Vision ».

| Gravité | Signification pour un projet personnel |
| --- | --- |
| Critique | Perte de données, fuite de secret, fonction DJ qui donne un résultat faux, travail non sauvegardé |
| Majeur | Bug visible au quotidien, performance ou fiabilité dégradée, outillage cassé |
| Mineur | Défaut limité ou rare, dette facile à rembourser |
| Smell | Qualité de code, sans effet utilisateur direct |

| Effort | Durée indicative |
| --- | --- |
| XS | moins d'une heure |
| S | une demi-journée |
| M | un à deux jours |
| L | trois jours ou plus |

**Méthode.** Six audits parallèles ont lu intégralement le code : Rust des nouvelles fonctions, Rust du cœur (base, bibliothèque, audio, export), frontend, design et accessibilité, contrat IPC, dépôt et CI. Ils ont produit environ 250 constats bruts qui se recoupent ; ce registre les dédoublonne.

**Vérifié par exécution** : Vitest, svelte-check, cargo check, cargo test, clippy avec les flags de la CI, ESLint, Prettier, cargo fmt, build mobile sur copie isolée, requêtes FTS5 dans sqlite3, deux panics reproduits, l'app réelle lancée 25 minutes avec logs, et chaque vue capturée dans un navigateur avec un faux backend. Les trois défauts les plus graves (mot de passe, purge au démarrage, tests sur bases réelles) ont été revérifiés à la main.

**Vérifié par lecture seulement** : le build Windows, le pipeline de release et le comportement de `beatportdl`. Les chemins de fichiers cités partent de `src-tauri/src` pour le Rust et de la racine du dépôt pour le reste.

## Tableau de synthèse

Le registre retient environ 180 défauts après dédoublonnage, dont 15 critiques et 72 majeurs. Le backend concentre deux tiers des critiques ; le design et l'i18n pèsent surtout en volume de travail.

| Domaine | Critiques | Majeurs | Mineurs et smells | Préfixe |
| --- | --- | --- | --- | --- |
| Backend Rust | 10 | 35 | environ 50 | C, B |
| Frontend Svelte | 4 | 15 | environ 25 | C, F |
| Cohérence backend ↔ frontend | 0 | 3 | 6 | I |
| Design, responsive, accessibilité | 0 | 10 | 2 | D |
| Internationalisation | 0 | 2 | 4 | L |
| Qualité, outillage, CI | 1 | 7 | 7 | C, Q |
| **Total** | **15** | **72** | **environ 94** |  |

Effort total estimé pour tout corriger : environ 5 à 6 semaines de travail, dont 3 semaines pour les critiques, les majeurs fonctionnels et l'hygiène, et 2 à 3 semaines pour l'unification visuelle et la traduction.

## Défauts critiques

Quinze défauts critiques uniques : neuf peuvent détruire ou fausser vos données, un expose un secret, quatre rendent une fonction DJ fausse, et le dernier est l'absence totale de commit. Les C1 à C5 doivent être traités avant toute nouvelle utilisation de l'app sur votre bibliothèque.

| # | Défaut | Où | Impact | Correctif | Effort |
| --- | --- | --- | --- | --- | --- |
| C1 | 27 800 lignes de travail dans aucun commit, ni branche, ni stash | dépôt entier | Un `git checkout` ou `git clean` malheureux efface des semaines de travail | Branche locale immédiate avec un commit instantané, puis découpage propre en phase 3 | XS |
| C2 | Identifiant et mot de passe Beatport en clair dans le code, réécrits dans `~/.config/beatportdl/beatportdl-config.yml` | `services/beatport/downloader.rs:108-109`, `client.rs:409` | Fuite du compte au premier push ; le fichier sur disque contient déjà le secret | Changer le mot de passe maintenant ; supprimer les littéraux ; lire les identifiants depuis le Trousseau macOS ; ajouter gitleaks au pre-commit | XS |
| C3 | Purge stricte automatique : tout titre Crate absent de Mixed In Key est supprimé, avec propagation cloud | `lib.rs:805-806`, `services/library/mik_db.rs` | Au démarrage, à chaque focus de fenêtre et à chaque modification de la base MIK ; un volume démonté suffit à déclencher des suppressions | Supprimer l'appel au démarrage ; remplacer par un rapport « titres absents de MIK » avec confirmation explicite | S |
| C4 | Écritures destructives dans `Collection11.mikdb` (DELETE en cascade sur les tables Core Data, `wal_checkpoint(TRUNCATE)`) | `mik_db.rs:114-189`, `upgrader.rs:884`, `update.rs:378` | Corruption possible de la bibliothèque Mixed In Key, sans sauvegarde | Ouvrir MIK exclusivement en lecture seule ; si une écriture est un jour voulue, sauvegarde `.mikdb.bak` et opt-in | S |
| C5 | Deux tests `cargo test` ouvrent vos vraies bases Crate (avec la clé) et Mixed In Key et y écrivent | `mik_db.rs:1022-1046` | Chaque `cargo test` altère votre bibliothèque | Supprimer ces tests ; fixtures sur bases temporaires ; interdire `~/Library` dans les tests | XS |
| C6 | L'upgrader vide des sous-dossiers et supprime les .jpg, .m3u, .txt du dossier de destination | `downloader.rs:151-198` (`flatten_and_clean_destination`) | Le dossier par défaut est votre dossier musique : pochettes, playlists et notes supprimées | Télécharger dans un dossier temporaire dédié, ne déplacer que le FLAC validé, ne jamais nettoyer un dossier utilisateur | S |
| C7 | Des FLAC déjà présents dans le dossier sont pris pour le téléchargement, et le MP3 part à la corbeille à tort | `downloader.rs`, `upgrader.rs` | Remplacement par le mauvais fichier | Identifier le fichier téléchargé par son chemin exact dans le dossier temporaire, vérifier durée et métadonnées | S |
| C8 | L'upgrade supprime le titre et le réimporte avec un nouvel identifiant | `upgrader.rs` | Cues, tags, playlists, note, couleur, compteur d'écoutes et lien cloud perdus | Mettre à jour `file_path`, format et bitrate du titre existant en place, dans une transaction | M |
| C9 | La « réparation » Spotify réécrit chaque écoute de moins de 30 s en écoute complète, à chaque démarrage | `services/stats/recorder.rs` (`repair_spotify_historical_durations`) | Minutes et streams gonflés, visible dans le log (« Repaired 5 Spotify historical listen events ») | Supprimer la réparation ; si nécessaire, migration unique et idempotente | XS |
| C10 | Double comptage Spotify : le poller live et la synchro « recently played » enregistrent la même écoute, et une pause suivie d'une reprise crée une nouvelle écoute | `services/stats/spotify.rs` | Tops et totaux faux | Clé de dédoublonnage (identifiant Spotify + horodatage de début arrondi) avec contrainte UNIQUE ; une seule source d'enregistrement | S |
| C11 | Hot cues décalés d'un cran : le backend indexe en base 1, le front accepte index, index−1 et index+1 ; le cue mémoire occupe un pad | `shared/stores/player.ts:1120-1130`, `PlayerView.svelte:151-162` | La touche 3 saute au cue 2 ; les pads mentent | Base 0 partout comme l'amont (ANLZ, PCOB) ; conversion unique à l'import MIK ; pads filtrés sur `cue_type = hot` | S |
| C12 | Export Rekordbox XML invalide dès qu'un chemin contient `&`, `"` ou `<` ; URL non encodée pour les caractères non ASCII ; cues mémoire exportés comme hot cue A, boucles perdues | `services/export/rekordbox_xml.rs` | Rekordbox refuse le fichier ou importe de mauvais cues | Échapper tous les attributs ; `file://localhost/` + percent-encoding ; `Num=-1` pour les cues mémoire, 0 à 7 pour les hot cues, `End` pour les boucles | S |
| C13 | Régression : `loadTracks()` sans argument réutilise le filtre courant | `apps/desktop/src/lib/stores/library.ts:51-65` | Retirer le dernier tag ne vide plus le filtre ; le filtre « Mix harmonique » devient collant | Restaurer la sémantique amont ; ajouter une méthode explicite `reloadWithCurrentFilter()` | XS |
| C14 | Régression : glisser un tag sur un titre ne fait plus rien (`data-track-id` retiré) | `TrackRow.svelte`, `useDragDropCoordination.ts:159` | Fonction amont cassée | Restaurer l'attribut et l'état de survol ; test de composant | XS |
| C15 | Le moteur Rust et la préécoute HTML peuvent jouer en même temps | `shared/stores/player.ts:481-492`, `555-562` | Deux sons superposés en mode lecteur autonome | Un seul point d'entrée « stopAllEngines() » appelé avant toute lecture, quelle que soit la source | S |

## Backend Rust

Le backend compte 35 défauts majeurs hors critiques, concentrés dans l'intégration Mixed In Key, les trackers de statistiques et le client Beatport. Le motif récurrent est le même : du travail lourd ou bloquant exécuté sous le verrou global de la base, sur le runtime async, sans test.

**Mixed In Key et bibliothèque**

| # | Défaut | Où | Correctif | Effort |
| --- | --- | --- | --- | --- |
| B1 | La synchro se redéclenche en rafale : 4 passages en 14 s observés au démarrage, chacun réécrit les 276 titres sans détecter de changement | `lib.rs` (watcher), `mik_db.rs` | Un seul déclencheur (mtime + taille), debounce réel, comparaison avant écriture, pas de synchro au focus | M |
| B2 | Rattachement par titre et artiste puis réécriture du `file_path` d'un autre titre | `mik_db.rs` (`sync_all_from_mik_db`) | Rattacher par chemin normalisé NFC puis par hash ; ne jamais réécrire un chemin sans confirmation | S |
| B3 | Les cues MIK sont recréés avec de nouveaux UUID à chaque synchro, sans tombstones ; les cues utilisateur sont effacés | `mik_db.rs` | Identifiant déterministe (hash titre + position + index) et upsert ; ne toucher qu'aux cues d'origine MIK | S |
| B4 | `get_track_cues` scanne toute la base MIK et résout tous les signets à chaque lecture, sous le verrou global | `commands/library.rs`, `mik_db.rs` | Lire les cues depuis la base Crate (déjà synchronisés) | S |
| B5 | `prune_missing_tracks` supprime au démarrage les titres d'un dossier simplement renommé | `services/library` | Marquer « manquant » (l'amont le fait déjà) au lieu de supprimer | XS |
| B6 | Signets macOS résolus sans `WithoutMounting` ni `WithoutUI`, fuite de `CFError` | `services/library/macos_bookmark.rs` | Ajouter les options de résolution, libérer les objets CF, retirer `create_bookmark` mort | S |
| B7 | `file_hash` n'est plus enregistré à l'import, et réimporter un fichier existant échoue sur une contrainte de clé étrangère | `services/library/import.rs` | Restaurer l'écriture du hash ; insérer les cues après résolution de l'identifiant final | S |
| B8 | Parseur Serato Markers2 décalé d'un octet (index, position, nom) | `services/library` (parseur Serato) | Corriger l'offset et ajouter un test sur un fichier réel anonymisé | S |
| B9 | La nouvelle colonne `energy` n'est ni sérialisée ni fusionnée par la synchro cloud | `services/cloud_sync` | Ajouter la colonne aux lignes et au merge | XS |
| B10 | Numérotation des migrations divergente de l'amont (15 entrées, libellées 7 à 16, la 6 sautée) | `db/schema.rs:385-534` | Renuméroter maintenant, avant que des bases ne dépendent de l'ordre actuel ; commentaire par migration | S |

**Statistiques (Crate Pulse)**

| # | Défaut | Où | Correctif | Effort |
| --- | --- | --- | --- | --- |
| B11 | Tracker Mixed In Key : « fichier ouvert dans `lsof` » est compté comme une écoute ; minutes plafonnées vers 30 s | `services/stats/mik.rs:56-101` | Ne compter que si MIK joue réellement (état du lecteur) ou désactiver par défaut | M |
| B12 | Tracker local : pauses non gérées, temps mural incluant les pauses, seek compté comme écoute | `services/player.rs` | Accumuler uniquement les intervalles en lecture ; réarmer après reprise | S |
| B13 | Import Rekordbox : horodatage = heure de l'import, réimport complet à chaque synchro, `master.db` illisible sans clé | `services/stats/rekordbox.rs` | Lire la date de session du XML, dédoublonner par session, documenter la limite `master.db` | S |
| B14 | Requêtes de stats : comparaisons de chaînes de dates hétérogènes, heatmap qui échoue sur une seule ligne invalide, agrégation en Rust sur toute la table | `services/stats/recorder.rs` | Stocker `played_at` en epoch millisecondes UTC, agréger en SQL, tolérer les lignes invalides | M |
| B15 | Import JSON Spotify : fichier entier transmis en chaîne par IPC, sans transaction, dédoublonnage par scan complet ; panic sur une date mal formée | `services/stats/spotify.rs` | Passer le chemin, parser en streaming, transaction unique, contrainte UNIQUE ; parser la date sans découpage d'octets | S |
| B16 | Inversion d'ordre des verrous entre `spotify_disconnect` et le poller : deadlock possible qui gèle toute la base | `services/stats/spotify.rs` | Ordre de verrouillage unique documenté, ou un seul verrou | S |
| B17 | Rafraîchissement du jeton Spotify : échec silencieux, jeton périmé renvoyé, course entre poller et commandes | `services/stats/spotify.rs` | Rafraîchissement centralisé derrière un mutex async, erreur remontée à l'UI | S |
| B18 | Serveur OAuth permanent sur 127.0.0.1:8888, `state` non imposé, paramètres reflétés dans le HTML de réponse | `services/stats/spotify.rs:1070-1076` | Lancer le serveur seulement pendant une connexion, avec délai ; exiger `state` ; échapper la réponse | S |

**Beatport et upgrader** (en plus de C2, C6, C7, C8)

| # | Défaut | Où | Correctif | Effort |
| --- | --- | --- | --- | --- |
| B19 | Jetons Beatport en clair dans `~/.config/crate/beatport_auth.json`, copiés dans le dossier de `beatportdl` et dans `localStorage` | `services/beatport/client.rs:358-416` | Trousseau macOS ou base chiffrée, une seule source de vérité | S |
| B20 | Récupération du jeton stocké par DJ.Studio dans sa configuration locale | `client.rs:281-321` | Supprimer | XS |
| B21 | `validate_token` ne valide rien et persiste un état « authentifié » fictif | `client.rs` | Appel réel à l'API (profil) ou suppression | XS |
| B22 | Validation FLAC insuffisante : un fichier tronqué de plus de 3 Mo passe | `downloader.rs:20-45` | Décoder le fichier entier avec symphonia et comparer la durée à celle attendue | S |
| B23 | Scoring : un radio edit peut remplacer un extended mix ; valeurs Beatport par défaut fictives (240 000 ms, 2026-01-01) injectées dans le score | `upgrader.rs` | Pénaliser l'écart de durée et de mix ; ne jamais scorer une valeur absente | S |
| B24 | Découpage d'artistes par regex sans frontière de mot : « Daft Punk » devient « Da » ; panic par index d'octets sur texte minusculisé | `upgrader.rs` (`split_artists`) | Frontières de mot, séparateurs entourés d'espaces, indices calculés sur la même chaîne | S |
| B25 | Pas de cache négatif ni de gestion des 429 : chaque ouverture relance un scan réseau complet | `upgrader.rs` (`find_upgrade_matches`) | Mettre en cache les « aucun résultat » avec expiration, backoff sur 429 | S |
| B26 | `execute_upgrade_replacements` : ordre non sûr, erreurs ignorées, plusieurs minutes sans progression ni annulation, `file_path` fourni par le webview | `commands/upgrader.rs:48-64` | Relire le chemin en base, étapes transactionnelles, événements de progression et annulation | M |

**Audio, recherche et export** (en plus de C12)

| # | Défaut | Où | Correctif | Effort |
| --- | --- | --- | --- | --- |
| B27 | Aucun code n'écrit `waveform_data` : la waveform affichée est toujours le motif factice de 64 barres | `services/analysis.rs`, `PlayerView.svelte:138` | Générer les pics à l'analyse (symphonia) ou lire `ZWAVEFORM` de MIK en lecture seule ; masquer la waveform si absente | M |
| B28 | FTS5 : la sanitisation rend introuvables les titres avec apostrophe, tiret ou slash, et `AND`, `OR`, `NOT` en majuscules font échouer `get_tracks` | `services/library/query.rs` | Mettre chaque jeton entre guillemets doubles avec `*` en suffixe ; repli sur `LIKE` en cas d'erreur ; tests sur « You'll », « Jay-Z », « AC/DC » | S |
| B29 | Position de lecture = horloge murale × vitesse, périmée en fin de piste (pas d'horloge CoreAudio contrairement à la synthèse) | `services/audio/mod.rs:347` | Utiliser `Sink::get_pos()` de rodio | S |
| B30 | Associations de fichiers en `rank: Default` avec le parapluie `public.audio` ; bundle `com.crate.app` inexistant dans le code natif | `tauri.conf.json:38-61`, `commands/standalone.rs:101-150` | `rank: Alternate`, retirer `public.audio`, supprimer le FFI et le script Swift | XS |
| B31 | `StartupFile` : course au démarrage, plusieurs fichiers ouverts s'écrasent | `lib.rs` | File d'attente de chemins, vidée quand le frontend est prêt | S |
| B32 | `delete_tracks_and_files` : corbeille via `osascript` par fichier, échecs silencieux, suppression définitive hors macOS | `services/library/update.rs` | Crate `trash`, erreurs remontées | S |
| B33 | I/O bloquantes, sous-processus (`beatportdl`, `lsof`, `osascript`) et rusqlite sous `std::sync::Mutex` exécutés sur le runtime tokio | plusieurs services | `spawn_blocking` ou commandes synchrones Tauri ; ne jamais tenir le verrou pendant une I/O disque ou réseau | M |
| B34 | Services instanciés deux fois : l'état géré par Tauri n'est pas celui des tâches de fond | `lib.rs:594-599` | Une instance `Arc` partagée | XS |
| B35 | `get_duplicate_count` lance un scan complet des doublons à chaque événement `duplicates-updated` | `services/duplicate.rs` | Compteur mis en cache, invalidé par les mutations | S |

**Mineurs et smells backend** (environ 50, regroupés) : erreurs réseau Beatport transformées en listes vides ; messages d'erreur français codés en dur dans `CrateError` ; tables sans clés étrangères et croissance non bornée (`listen_events`, `upgrade_matches_cache`, `pkce_verifier_*`) ; boucles de fond sans annulation ni backoff ; huit copies du mapper `Track` ; lecteurs lofty/symphonia dupliqués quatre fois ; `Regex::new` recompilée à chaque appel ; base64 réimplémenté ; 330 lignes de HTML dupliquées dans le handler OAuth ; chemins personnels dans le code (`downloader.rs:76`, `mik_db.rs:1005`). Chacun est XS à S et se traite au fil de la phase 3.

## Frontend Svelte

Le frontend compile sans erreur de type, mais 15 défauts majeurs touchent les raccourcis, l'initialisation, la performance et l'honnêteté de l'interface. La plupart viennent d'un même travers : des effets et des timers ajoutés sans penser à leur nettoyage ni aux autres vues.

| # | Défaut | Où | Correctif | Effort |
| --- | --- | --- | --- | --- |
| F1 | Touches 1 à 8 capturées partout, sans condition de vue ni de modificateur | `hooks/useKeyboardShortcuts.ts` | Actives seulement en vue Player et hors champ de saisie, ou derrière un modificateur | XS |
| F2 | Espace en vue Player lance un titre récent au lieu de mettre en pause la préécoute ; logique dupliquée | `useKeyboardShortcuts.ts`, `PlayerView.svelte` | Un seul gestionnaire « lecture/pause » qui agit sur la source active | S |
| F3 | Espace ou Entrée sur une ligne focalisée déclenche la lecture et le raccourci global | `TrackRow.svelte`, raccourcis | `stopPropagation` sur la ligne, ou ignorer les raccourcis quand une ligne a le focus | XS |
| F4 | `withTimeout` : une initialisation de plus de 2,5 s perd les fonctions de nettoyage (écouteurs Tauri jamais détachés) et masque les erreurs | `hooks/useAppSetup.ts:752-798` | Retirer le timeout ou conserver la promesse et nettoyer quand elle se résout | S |
| F5 | Splash fermé par un timer de module à 1,5 s : course avec le chargement des réglages, flash possible de l'onboarding | `stores/splash.ts:15` | Fermer le splash quand les réglages sont chargés | XS |
| F6 | Synchro Mixed In Key complète et rechargement de toute la bibliothèque à chaque focus de fenêtre | `routes/+layout.svelte:341-347` | Supprimer ; le watcher backend suffit | XS |
| F7 | L'effet du Toolbar déclenche un appel `get_duplicate_count` à chaque mutation de la bibliothèque | `layout/Toolbar.svelte:55-60` | S'abonner à l'événement backend seulement | XS |
| F8 | Beatport : favoris et playlists « créés » uniquement en local avec un toast de succès trompeur ; panier vidé même si des téléchargements échouent | `shared/stores/beatport.ts` | Appeler l'API réelle ou masquer ces actions ; ne retirer du panier que les succès | S |
| F9 | Effets de bord à l'import de modules dans `shared/` (timer Beatport de 3 min, timer du splash) et dépendances desktop dans le code partagé | `shared/stores/beatport.ts`, `splash.ts` | Initialisation explicite depuis `useAppSetup` | S |
| F10 | Mutation d'une valeur `$derived` (`activeHeroTrack.is_in_library = true`) | `PlayerView.svelte` | Mettre à jour le store source | XS |
| F11 | Cues et waveform jamais chargés pour un fichier externe : un UUID est envoyé à `get_track_cues` | `shared/stores/player.ts` | Charger par chemin pour les fichiers hors bibliothèque | S |
| F12 | Course de réponses obsolètes dans le suivi de position (intervalle async appelant `getPlaybackState` chaque seconde) | `shared/stores/player.ts` | Numéro de requête et abandon des réponses anciennes, ou événement de position poussé par le backend | S |
| F13 | « Synchroniser avec Mixed In Key » du menu contextuel ignore la sélection et resynchronise toute la base | `library/trackContextMenuItems.ts` | Passer les identifiants sélectionnés | XS |
| F14 | Upgrader : la confiance s'affiche « 0.96% » au lieu de « 96 % » ; le bouton de remplacement reste actif quand Beatport n'est pas connecté | `components/upgrader/` | Multiplier par 100 avec `Intl.NumberFormat` ; désactiver sans connexion | XS |
| F15 | L'infobulle du badge Mixed In Key reste affichée après le départ du pointeur | `layout/Toolbar.svelte` | Utiliser le composant `Tooltip` commun | XS |

**Mineurs et smells frontend** (environ 25) : `{#each}` sans clé sur des listes mutables ; `window.confirm()` natif pour supprimer un album ; colonne taille de fichier toujours « - » ; deux stores de préférences d'affichage non synchronisés ; `formatBitrate` qui suppose du PCM 24 bits ; lecture continue en mode autonome qui enchaîne sur l'historique ; 16 `any` explicites ; une trentaine d'exports morts ; un `console.log` oublié ; composants géants (`PlayerView.svelte` fait 938 lignes) ; logique copiée entre `PlayerView` et `Player`.

## Cohérence backend ↔ frontend

Le câblage est le point le plus sain du fork : aucune commande manquante, aucun appel orphelin, aucun désaccord d'arguments. Les défauts sont dans le contrat d'erreurs, dans des réglages sans effet et dans l'absence de progression pour les opérations longues.

| Mesure | Valeur |
| --- | --- |
| Commandes `#[tauri::command]` | 230 noms, toutes enregistrées |
| Noms invoqués depuis le frontend | 226, tous résolus |
| Nouvelles commandes du fork | 75 |
| Commandes jamais invoquées | 4 (2 doublons nouveaux, 2 amont) |
| Désaccords d'arguments | 0 |
| Événements émis sans écouteur | 1 (`library-updated`) |

| # | Défaut | Où | Correctif | Effort |
| --- | --- | --- | --- | --- |
| I1 | Messages d'erreur backend perdus : Tauri rejette avec une chaîne, et les nouveaux stores testent `instanceof Error`, toujours faux ; l'heuristique d'authentification de l'Upgrader ne peut jamais détecter l'erreur | `shared/stores/{upgrader,duplicate,stats,player,albums}.ts` | Helper partagé `toErrorMessage(e)` ; à terme, sérialiser `CrateError` en `{ code, message }` et traduire le code côté TS | S |
| I2 | Deux réglages Beatport sans effet : le téléchargeur force `lossless` et lance toujours la synchro Mixed In Key | `services/beatport/downloader.rs:110,284` | Lire `AppSettings` dans la commande et propager qualité et synchro | S |
| I3 | Aucun événement de progression pour le téléchargement Beatport et l'upgrader : l'UI reste figée plusieurs minutes | `commands/beatport.rs:122-160`, `commands/upgrader.rs` | `tauri::ipc::Channel` de progression ; les types TS existent déjà mais sont inutilisés | M |
| I4 | `BeatportAuthState` porte deux conventions (camelCase côté TS, snake\_case côté Rust) ; la cohérence repose sur 6 sites du store qui dupliquent les deux | `shared/types/beatport.ts:89-100`, `client.rs:82-90` | Un seul schéma snake\_case aligné sur Rust | S |
| I5 | `record_listen_event` accepte un événement complet depuis le webview mais n'est appelé nulle part | `commands/stats.rs:75` | Supprimer | XS |
| I6 | Doublons de commandes `spotify_set_client_id` et `spotify_set_client_secret` ; le secret Spotify est relisible par le webview | `commands/stats.rs:95-131` | Supprimer les doublons et la commande de relecture du secret | XS |
| I7 | `library-updated` émis sans écouteur ; paramètre `searchType` de la recherche Beatport ignoré | `commands/upgrader.rs:62`, `commands/beatport.rs:92-97` | Écouter l'événement dans le layout ou le supprimer ; implémenter ou retirer le paramètre | XS |
| I8 | Une vingtaine de champs `Option<T>` Rust typés `?: T` côté TS alors que la valeur réelle est `null` | `shared/types/beatport.ts`, `album.ts`, `index.ts` | Typer `T \| null` | XS |
| I9 | Options d'affichage stockées uniquement dans `localStorage` : ni sauvegarde, ni synchro | `shared/stores/displaySettings.ts:34` | Les persister dans la table `settings` comme les autres préférences | S |

## Design, responsive et accessibilité

L'amont a un vrai système de design que le fork ignore presque entièrement : trois langages visuels cohabitent (amont piloté par l'accent, cyan et ambre du Player et de Pulse, vert néon de Beatport), et aucune nouvelle vue ne suit la couleur d'accent choisie dans les réglages. La mise en conformité est estimée à 10 à 14 jours.

| Indicateur | Amont | Nouveau code |
| --- | --- | --- |
| Classes de palette Tailwind au lieu de tokens | 88 | 584 |
| Couleurs hexadécimales dans les composants | 143 (surtout dans `style.css`) | 299 |
| Tailles de police arbitraires `text-[8–12px]` | 20 | 195 |
| Rayons `rounded-xl` à `3xl` | 1 | 88 |
| `backdrop-blur` | 1 | 26 |
| Variantes `dark:` | 0 | 43 |
| Boutons ad hoc contre composant `Button` | — | 102 contre 8 |

| # | Défaut | Où | Correctif | Effort |
| --- | --- | --- | --- | --- |
| D1 | `dark:` suit le thème de l'OS, pas celui de l'app : les « corrections mode clair » du build 40 sont fausses dès que l'OS et l'app diffèrent | 43 occurrences | Supprimer `dark:` et utiliser les tokens, ou déclarer `@custom-variant dark` sur `[data-theme='dark']` | S |
| D2 | Environ 70 classes référencent des tokens inexistants (`surface-3`, `surface-4`, `stroke-strong`, `text-disabled`, `brand-primary/NN`) : squelettes de chargement et pistes de progression invisibles | nouveaux composants | Créer les tokens manquants ou remplacer par les existants | S |
| D3 | Thème clair cassé : cartes d'intégration de Pulse sombres en haut et claires en bas ; badges de l'en-tête sombres ; contrastes mesurés de 1,04:1 à 2,78:1 sur l'onglet Player, les badges, la heatmap et le badge énergie | `components/stats/`, `Toolbar.svelte`, `EnergyBadge.svelte` | Tokens uniquement ; objectif 4,5:1 pour le texte et 3:1 pour les éléments d'interface | M |
| D4 | En-tête à la largeur minimale (1000 px) : les icônes chevauchent le contrôle segmenté Player/Bibliothèque/Beatport, un clic sur « doublons » atteint Beatport ; même à 1400 px, « Beatport » est tronqué en vue Découvertes | `layout/Toolbar.svelte` | Regrouper les outils dans un menu « Outils », contrôle segmenté en `flex-shrink-0`, badge MIK réduit à une pastille | S |
| D5 | Hero du Player à hauteur fixe (`h-[225px]` pour environ 260 px de contenu) : à 1000×640 le transport est recouvert et il reste 2 lignes de récents ; à 1920×1080 de grands vides | `player/PlayerView.svelte` | Hauteur intrinsèque, pochette en `clamp()`, liste des récents en `flex-1 min-h-0` | M |
| D6 | Modales plus hautes ou plus larges que la fenêtre : le pied du Duplicate Killer et son bouton « Supprimer » sont coupés à 1400×900 ; les modales `4xl` (1152 px) dépassent la largeur minimale | `components/duplicates/`, `upgrader/`, `common/Modal.svelte` | `max-h-[calc(100vh-32px)]` avec défilement interne et pied fixe ; largeur `min(1152px, 100vw - 32px)` | S |
| D7 | Tableau Beatport à 1000 px : la colonne titre se réduit à un caractère | `components/beatport/` | Grille `minmax(0, …)` avec largeur minimale sur le titre, masquage des colonnes secondaires | S |
| D8 | Polices Jost et DM Sans proposées dans les réglages mais non câblées (`[data-font='jost']` absent) ; 15 fichiers sur 20 inutilisés (896 Ko) ; licences OFL absentes ; double chargement via Google Fonts | `style.css`, `static/fonts/` | Câbler ou retirer l'option, garder 5 fichiers, ajouter OFL, retirer l'import Google | XS |
| D9 | Six icônes utilisées n'existent pas dans `Icon.svelte` ; le logo Beatport SVG chargé en `<img>` avec `currentColor` sort noir sur fond sombre | `common/Icon.svelte`, `components/beatport/` | Ajouter les icônes et typer le nom (union) pour que `svelte-check` échoue sur un nom inconnu ; logo inline | S |
| D10 | Accessibilité : environ 25 boutons icône sans nom (transport, segments, badge MIK, actions des récents) ; lignes cliquables en `div` masquées par `svelte-ignore a11y` ; waveform souris uniquement ; heatmap non focusable ; 3 overlays sans piège de focus ; `ToggleSwitch` imbrique deux contrôles ; 19 animations infinies sans `motion-reduce` ; l'amont supprime le contour de focus globalement | nombreux | `aria-label` partout, `<button>` pour tout élément cliquable, flèches clavier sur la waveform, `Modal` commun, `focus-visible:ring` | M |
| D11 | Composants réinventés au lieu des communs : 4 contrôles segmentés, checkbox, select, spinner, tooltip et confirm maison ; badge Camelot copié 8 fois | nouveaux composants | Extraire `SegmentedControl`, `KeyBadge`, `EnergyBadge` et utiliser `Button`, `Checkbox`, `Select`, `Tooltip`, `Spinner` | M |
| D12 | Badge « Build 57 » permanent à côté du logo et libellés « PRO » sur les marques tierces | `Toolbar.svelte`, `+layout.svelte` | Afficher la version dans « À propos » uniquement | XS |

**Règles de design strictes proposées**, dérivées des conventions de l'amont :

1. Couleurs par tokens uniquement, aucune classe de palette ni hex dans un composant.
2. Toute nouvelle couleur sémantique devient un token déclaré pour les deux thèmes, avec contraste vérifié ; trois au maximum pour cue, live et marque.
3. Jamais `dark:` : le thème est `[data-theme]`, pas l'OS.
4. Typographie via le composant `Text` : pas de `text-[Npx]`, corps minimal 12 px pour une donnée.
5. Rayons `rounded` ou `rounded-md` pour les contrôles, `rounded-lg` pour les cartes et modales, `rounded-full` pour les pastilles.
6. Élévation limitée à `shadow-sm`, `shadow-lg`, `shadow-xl` ; pas de dégradé, de glow ni de flou d'arrière-plan.
7. Composants communs obligatoires : `Button`, `IconButton`, `Modal`, `Checkbox`, `Select`, `Tooltip`, `Spinner`, `Icon`.
8. Tout élément cliquable est un bouton avec nom accessible ; pas de `svelte-ignore a11y`.
9. Zéro chaîne en dur : clé de traduction ajoutée dans la même modification.
10. Pas de hauteur fixe en pixels sur une zone de contenu ; chaque vue et chaque modale utilisable à 1000×600.
11. Transitions de 150 à 200 ms sur couleur, opacité ou transformation ; animations infinies réservées aux spinners avec `motion-reduce`.
12. Définition de terminé : captures clair et sombre, deux accents, deux tailles de fenêtre, et un scan CI qui refuse palette, hex, `dark:` et tailles arbitraires.

## Internationalisation

Toutes les nouvelles vues sont écrites en français en dur : avec l'app en anglais, la bibliothèque est traduite mais Crate Pulse, l'Upgrader, le Duplicate Killer, le Player et l'onglet Beatport restent en français. La vue Beatport mélange même les deux langues (« Purchased tracks » à côté de « Ajouter tout au panier »).

| Zone | Appels de traduction | Chaînes en dur repérées |
| --- | --- | --- |
| `components/stats/` | 0 | 64 |
| `player/PlayerView.svelte` | 0 | 44 |
| `components/beatport/` | 0 | 27 |
| `components/upgrader/` | 0 | 26 |
| `settings/tabs/BeatportTab.svelte` | 0 | 25 |
| `components/duplicates/` | 0 | 13 |
| `settings/tabs/DisplayOptionsTab.svelte` | 30 | 0 |
| Stores `shared/` (toasts) | 0 | 18 |
| Erreurs Rust (`CrateError`, client Beatport) | — | messages français codés en dur |

| Locale | Clés manquantes sur 792 | Couverture |
| --- | --- | --- |
| en | 0 | 100 % |
| fr | 1 | 99,9 % |
| ja | 49 | 93,8 % |
| de, es, it, ko, nl, pt, sv, zh | 50 | 93,7 % |
| pl, ro, tr, uk | 53 | 93,3 % |

| # | Défaut | Correctif | Effort |
| --- | --- | --- | --- |
| L1 | 17 composants nouveaux sans aucune traduction, environ 220 chaînes | Extraire les chaînes en clés `stats.*`, `player.*`, `beatport.*`, `upgrader.*`, `duplicates.*` dans `en.json` et `fr.json` | M |
| L2 | 13 locales sans les 49 à 53 clés ajoutées par le fork | Décider : traduire, ou laisser le repli anglais de svelte-i18n et le documenter | S |
| L3 | Nombres et unités formatés à l'anglaise en français (« 2,310 »), « plays » et « écoutes » mélangés dans Pulse | `Intl.NumberFormat` avec la locale active, pluriels ICU | XS |
| L4 | Messages d'erreur Rust en français | Codes d'erreur en anglais traduits côté frontend (voir I1) | S |
| L5 | `register('en')` et `register('fr')` conservés après `addMessages` : l'initialisation repasse par le chemin asynchrone | Retirer les deux `register` redondants | XS |
| L6 | Le README annonce 11 langues alors que le dépôt en contient 15 | Mettre à jour le README | XS |

## Qualité, tests, outillage et CI

Si l'arbre était commité tel quel, 5 des 8 jobs de la CI amont échoueraient, et aucun workflow n'exécute les tests. Pour un usage personnel sur ce seul Mac, les builds Windows et mobile comptent peu ; la configuration de release et les tests, en revanche, protègent vos données.

| # | Défaut | Où | Correctif | Effort |
| --- | --- | --- | --- | --- |
| Q1 | `tauri.prod.conf.json` modifié pour un build local (`targets: ["app"]`, artefacts de mise à jour désactivés) : une release publierait un `latest.json` à signature vide et casserait l'auto-update | `src-tauri/tauri.prod.conf.json:5-6` | Restaurer ; build local avec `--config '{"bundle":{"targets":["app"],"createUpdaterArtifacts":false}}'` | XS |
| Q2 | Build iOS/Android cassé : `services::beatport`, `services::stats` et deux commandes de bibliothèque ne sont pas protégés par la feature `desktop` | `lib.rs:193-194`, `services/mod.rs`, `commands/mod.rs` | `#[cfg(feature = "desktop")]` sur modules, enregistrements et `.manage()` | S |
| Q3 | Build Windows/Linux cassé : `RunEvent::Opened` n'existe que sur macOS, iOS et Android | `lib.rs` | `#[cfg(any(target_os = "macos", target_os = "ios"))]` autour du bras ; lire `argv` ailleurs | XS |
| Q4 | Clippy avec `-D warnings` : 37 erreurs (16 de code mort) ; cargo fmt : 35 fichiers | Rust | `cargo fmt`, suppression du code mort, correctifs clippy | S |
| Q5 | ESLint : 38 erreurs ; Prettier : 60 fichiers | TypeScript et Svelte | `yarn format:fix && yarn lint:fix`, puis correction manuelle des `each` sans clé et des `any` | S |
| Q6 | Aucun workflow n'exécute `yarn test` ni `cargo test` | `.github/workflows/` | Ajouter un job de tests (Vitest et cargo test sur base temporaire) | XS |
| Q7 | Tests absents sur les chemins à risque : remplacement de fichiers, purge, pollers, OAuth ; 3 tests lisent la vraie base MIK et passent silencieusement si elle est absente | `services/library/mik_db.rs:982-1010` | Tests d'intégration sur `tempdir` ; fixtures MIK anonymisées | M |
| Q8 | Vitest tourne sur Vite 8.2 alors que l'app utilise Vite 7.3 ; `test:coverage` sans fournisseur de couverture ; dépendances de test avec caret alors que tout le reste est épinglé | `package.json`, `yarn.lock` | Épingler vitest sur une version compatible Vite 7 ; ajouter `@vitest/coverage-v8` | XS |
| Q9 | Icônes dev, staging et prod devenues identiques octet pour octet ; `.ico`, icônes Windows, iOS et Android non régénérées | `src-tauri/icons/` | `yarn tauri icon` par environnement, avec variante visible pour dev | S |
| Q10 | CHANGELOG, version, README et site de documentation non mis à jour ; la doc des raccourcis contredit le nouveau comportement de Shift+flèches | `CHANGELOG.md`, `README.md`, `docs/` | Entrées `[Unreleased]` par fonction ; version 0.3.0 ; pages docs mises à jour | S |
| Q11 | Fichiers à ne pas commiter : `SYNTHESE_DISCUSSION.md` (chemins personnels), `scripts/set_default_player.swift`, polices inutilisées, logos de marques tierces | racine, `static/` | Exclure ou déplacer ; `.gitignore` | XS |
| Q12 | Pas de `CLAUDE.md` : chaque assistant redécouvre les règles (feature `desktop`, flags clippy de la CI, i18n, jamais de bases réelles en test) | racine | Créer un `CLAUDE.md` de 15 lignes à partir des règles de ce registre | XS |
| Q13 | `yarn dev` compile le Rust en `--release` : chaque modification coûte plusieurs minutes (amont) | `package.json:11` | Profil debug pour le développement | XS |
| Q14 | 87 alertes `yarn audit` (61 hautes) dans l'outillage transitif ; `cargo audit` non installé localement | dépendances | `yarn upgrade` ciblé ; installer `cargo-audit` | S |

## Ordre de réparation recommandé

Réparer dans cet ordre : chaque étape rend la suivante plus sûre, et chaque ligne correspond à un ou deux commits vérifiables. Les étapes 1 à 9 forment le scénario A du rapport principal.

| Étape | Contenu | Défauts traités | Effort | Vérification |
| --- | --- | --- | --- | --- |
| 1 | Changer le mot de passe Beatport ; sauvegarder `crate.db`, `db.key` et `Collection11.mikdb` ; branche locale et commit instantané du travail actuel | C1, C2 (rotation) | 1 h | `git log` montre le commit ; sauvegardes présentes |
| 2 | Couper les opérations destructives : purge au démarrage, écritures MIK, synchro au focus, tests sur bases réelles ; restaurer la config prod | C3, C4, C5, B5, F6, Q1 | 2 h | Relancer l'app : aucun « prune » dans les logs ; `cargo test` ne touche plus `~/Library` |
| 3 | Retirer les identifiants du code, jetons Beatport et secret Spotify dans le Trousseau, supprimer la récupération DJ.Studio | C2, B19, B20, I6 | 0,5 j | `grep` sans résultat ; gitleaks en pre-commit |
| 4 | Upgrader sûr : dossier temporaire, remplacement en place, validation par décodage, progression et annulation (sur branche privée) | C6, C7, C8, B22, B26, I3 | 2 à 3 j | Tests sur `tempdir` : cues et tags conservés, aucun fichier voisin touché |
| 5 | Mixed In Key propre : lecture seule, synchro unique et incrémentale, cues déterministes, rattachement par chemin | B1, B2, B3, B4, B6 | 1,5 j | Log de démarrage : une seule synchro, « 0 updated » au second lancement |
| 6 | Statistiques justes : supprimer la réparation, dédoublonnage Spotify, pauses, tracker MIK, dates en epoch, verrous | C9, C10, B11 à B18 | 2 j | Test : une écoute Spotify ne produit qu'une ligne ; une pause ne compte pas |
| 7 | Fonctions DJ exactes : hot cues base 0, waveform réelle, FTS5, export XML, position audio, trois régressions, deux moteurs audio | C11 à C15, B27 à B29, F1 à F3, F11 | 3 j | Touche 3 = cue 3 ; recherche « You'll » trouvée ; XML avec `&` ouvert dans Rekordbox |
| 8 | Frontend robuste : `withTimeout`, splash, effets coûteux, erreurs IPC, réglages Beatport effectifs | F4 à F10, F12 à F15, I1, I2, I4 | 1,5 j | Pas d'appel IPC par frappe ; messages d'erreur backend visibles |
| 9 | Hygiène : fmt, lint, code mort, feature `desktop`, Windows, tests en CI, icônes, polices, CHANGELOG, `CLAUDE.md`, découpage en commits thématiques | Q2 à Q14, B10 | 2 à 3 j | CI verte ; installation d'une 0.3.0 personnelle |
| 10 | Fondations visuelles : tokens manquants, `dark:`, polices, icônes, focus visible, gabarit des modales | D1, D2, D6, D8, D9 | 1 j | Captures clair et sombre sans élément invisible |
| 11 | Mise en conformité vue par vue : Player, Pulse, Beatport, Upgrader, Doublons, en-tête | D3, D4, D5, D7, D10, D11, D12 | 8 à 12 j | Définition de terminé des règles de design |
| 12 | Traduction complète français et anglais, repli documenté pour les autres langues | L1 à L6 | 2 à 3 j | App en anglais : aucune chaîne française |

Les mineurs et smells (environ 90) se traitent au passage, dans l'étape qui touche le même fichier. Le harnais navigateur construit pour cet audit (faux backend Tauri, 16 pistes, paramètres de thème et de langue) peut servir de base aux tests de bout en bout des étapes 10 à 12.
