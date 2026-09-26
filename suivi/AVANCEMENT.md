# Avancement des corrections

Ce fichier est **la** source de vérité du suivi : chaque défaut du [registre](REGISTRE-DEFAUTS.md) y a une case, cochée dans le même commit que sa correction. Le détail de chaque modification est dans le [CHANGELOG](../CHANGELOG.md) ; les commits portent l'identifiant entre crochets (`git log --grep "\[C11\]"`).

Légende : `[x]` corrigé et vérifié · `[ ]` à faire · _note en italique_ = précision ou reste à faire. Le tableau de progression se recalcule avec `yarn suivi`.

<!-- progression:start -->
**Progression globale : 93 / 106 défauts corrigés (88 %)**

| Étape | Corrigés | Progression |
| --- | --- | --- |
| Étape 1 — Mise en sûreté | 2 / 2 | ██████████ |
| Étape 2 — Couper les opérations destructives | 6 / 6 | ██████████ |
| Étape 3 — Secrets et authentification | 4 / 4 | ██████████ |
| Étape 4 — Upgrader sûr | 9 / 9 | ██████████ |
| Étape 5 — Mixed In Key propre | 8 / 8 | ██████████ |
| Étape 6 — Statistiques justes | 10 / 10 | ██████████ |
| Étape 7 — Fonctions DJ exactes | 17 / 18 | █████████░ |
| Étape 8 — Frontend robuste | 17 / 17 | ██████████ |
| Étape 9 — Hygiène et outillage | 11 / 14 | ████████░░ |
| Étape 10 — Fondations visuelles | 5 / 5 | ██████████ |
| Étape 11 — Conformité vue par vue | 3 / 7 | ████░░░░░░ |
| Étape 12 — Traduction | 1 / 6 | ██░░░░░░░░ |
<!-- progression:end -->

## Actions réservées au propriétaire

Ces actions ne peuvent pas être faites à ta place (comptes personnels, décisions).

- [ ] **Changer le mot de passe du compte Beatport** : il était en clair dans le code (C2). Le code est nettoyé, mais l'ancien mot de passe doit être considéré comme compromis.
- [ ] **Confirmer que Mixed In Key reste en lecture seule** (décision du rapport, commentaire ouvert dans le document d'audit). Les corrections de l'étape 2 partent de ce principe.
- [ ] **Répondre aux questions ouvertes** du [rapport](RAPPORT-AUDIT.md#décisions-à-prendre) : autres machines que ce Mac ? conserver l'historique Spotify faussé ? Crate lecteur par défaut ou seulement « Ouvrir avec » ?

## Hors registre

- [x] Sauvegarde de `crate.db`, `db.key`, de la sauvegarde automatique, des pochettes et de `Collection11.mikdb` dans `~/Documents/Crate-sauvegardes/2026-09-26-avant-corrections/` (empreintes vérifiées)
- [x] Droits du fichier `~/.config/beatportdl/beatportdl-config.yml` réduits à `600` (il contient l'ancien mot de passe)
- [x] Dépôt GitHub personnel privé `Crate`, remote `origin` ; l'amont `blackboxaudio/crate` devient `upstream`
- [x] Workflows amont (builds macOS/Windows à chaque push) passés en déclenchement manuel pour ne pas consommer les minutes Actions ; CI légère `ci.fork.yml` (Vitest)
- [x] Documents de suivi : ce fichier, [rapport](RAPPORT-AUDIT.md), [registre](REGISTRE-DEFAUTS.md), [historique](historique/SYNTHESE_DISCUSSION.md), `CLAUDE.md`

## Défauts du registre, par étape de réparation

Les mineurs et smells (environ 90, non numérotés) sont traités au passage dans l'étape qui touche le même fichier et notés dans le CHANGELOG.

### Étape 1 — Mise en sûreté

_Critère de sortie : Sauvegardes, commit instantané, secret retiré avant tout push._

- [x] **C1** — 27 800 lignes de travail dans aucun commit, ni branche, ni stash
- [x] **C2** — Identifiant et mot de passe Beatport en clair dans le code, réécrits dans… — _Code nettoyé (rotation du mot de passe à faire par le propriétaire)_

### Étape 2 — Couper les opérations destructives

_Critère de sortie : Aucune suppression automatique, aucun test sur vraies bases._

- [x] **C3** — Purge stricte automatique : tout titre Crate absent de Mixed In Key est supprimé, avec propagation cloud
- [x] **C4** — Écritures destructives dans `Collection11.mikdb` (DELETE en cascade sur les tables Core Data,…
- [x] **C5** — Deux tests `cargo test` ouvrent vos vraies bases Crate (avec la clé) et Mixed In Key et y écrivent
- [x] **B5** — `prune_missing_tracks` supprime au démarrage les titres d'un dossier simplement renommé — _le nettoyage des fichiers disparus reste disponible, mais uniquement sur action manuelle_
- [x] **F6** — Synchro Mixed In Key complète et rechargement de toute la bibliothèque à chaque focus de fenêtre
- [x] **Q1** — `tauri.prod.conf.json` modifié pour un build local (`targets: ["app"]`, artefacts de mise à jour… — _build personnel : `yarn build:local`_

### Étape 3 — Secrets et authentification

_Critère de sortie : Plus aucun identifiant en clair, une seule source de vérité._

- [x] **B19** — Jetons Beatport en clair dans `~/.config/crate/beatport_auth.json`, copiés dans le dossier de `beatportdl`… — _`beatportdl-credentials.json` reste nécessaire à beatportdl : écrit en `600` et supprimé à la déconnexion_
- [x] **B20** — Récupération du jeton stocké par DJ.Studio dans sa configuration locale
- [x] **B21** — `validate_token` ne valide rien et persiste un état « authentifié » fictif
- [x] **I6** — Doublons de commandes `spotify_set_client_id` et `spotify_set_client_secret` ; le secret Spotify est…

### Étape 4 — Upgrader sûr

_Critère de sortie : Aucun fichier voisin touché, cues et tags conservés._

- [x] **C6** — L'upgrader vide des sous-dossiers et supprime les .jpg, .m3u, .txt du dossier de destination
- [x] **C7** — Des FLAC déjà présents dans le dossier sont pris pour le téléchargement, et le MP3 part à la corbeille à tort
- [x] **C8** — L'upgrade supprime le titre et le réimporte avec un nouvel identifiant
- [x] **B22** — Validation FLAC insuffisante : un fichier tronqué de plus de 3 Mo passe
- [x] **B23** — Scoring : un radio edit peut remplacer un extended mix ; valeurs Beatport par défaut fictives (240 000 ms,…
- [x] **B24** — Découpage d'artistes par regex sans frontière de mot : « Daft Punk » devient « Da » ; panic par index… — _le panic par index d’octets n’a pas été retrouvé dans le code actuel_
- [x] **B25** — Pas de cache négatif ni de gestion des 429 : chaque ouverture relance un scan réseau complet
- [x] **B26** — `execute_upgrade_replacements` : ordre non sûr, erreurs ignorées, plusieurs minutes sans progression ni… — _chemin relu en base, ordre sûr, erreurs remontées, progression ; annulation non implémentée_
- [x] **I3** — Aucun événement de progression pour le téléchargement Beatport et l'upgrader : l'UI reste figée plusieurs… — _fait pour l’upgrader (`upgrade-progress`) ; téléchargement du panier Beatport sans progression_

### Étape 5 — Mixed In Key propre

_Critère de sortie : Une seule synchro, incrémentale, en lecture seule._

- [x] **B1** — La synchro se redéclenche en rafale : 4 passages en 14 s observés au démarrage, chacun réécrit les 276… — _vérifié sur une copie de la vraie base : 2e passage « 0 updated » (276 titres réécrits à chaque passage avant)_
- [x] **B2** — Rattachement par titre et artiste puis réécriture du `file_path` d'un autre titre — _rattachement par titre/artiste limité à une correspondance unique dont le fichier a disparu ; rattachement par hash non fait_
- [x] **B3** — Les cues MIK sont recréés avec de nouveaux UUID à chaque synchro, sans tombstones ; les cues utilisateur… — _vérifié sur copie : 2 033 cues convertis sans perte ni doublon_
- [x] **B4** — `get_track_cues` scanne toute la base MIK et résout tous les signets à chaque lecture, sous le verrou global
- [x] **B6** — Signets macOS résolus sans `WithoutMounting` ni `WithoutUI`, fuite de `CFError`
- [x] **B7** — `file_hash` n'est plus enregistré à l'import, et réimporter un fichier existant échoue sur une contrainte…
- [x] **B8** — Parseur Serato Markers2 décalé d'un octet (index, position, nom)
- [x] **B9** — La nouvelle colonne `energy` n'est ni sérialisée ni fusionnée par la synchro cloud

### Étape 6 — Statistiques justes

_Critère de sortie : Une écoute = une ligne, pauses exclues._

- [x] **C9** — La « réparation » Spotify réécrit chaque écoute de moins de 30 s en écoute complète, à chaque démarrage — _les écoutes déjà « réparées » par les versions précédentes ne peuvent pas être restaurées (voir question ouverte sur l’historique Spotify)_
- [x] **C10** — Double comptage Spotify : le poller live et la synchro « recently played » enregistrent la même écoute, et… — _l’historique Spotify déjà en double n’est pas nettoyé automatiquement (décision du propriétaire)_
- [x] **B11** — Tracker Mixed In Key : « fichier ouvert dans `lsof` » est compté comme une écoute ; minutes plafonnées… — _suivi désactivé par défaut, activable dans Crate Pulse ; pas de détection fiable de la lecture MIK_
- [x] **B12** — Tracker local : pauses non gérées, temps mural incluant les pauses, seek compté comme écoute
- [x] **B13** — Import Rekordbox : horodatage = heure de l'import, réimport complet à chaque synchro, `master.db`… — _`master.db` de Rekordbox 6/7 reste illisible (chiffré) : l’import XML est la voie supportée_
- [x] **B14** — Requêtes de stats : comparaisons de chaînes de dates hétérogènes, heatmap qui échoue sur une seule ligne…
- [x] **B15** — Import JSON Spotify : fichier entier transmis en chaîne par IPC, sans transaction, dédoublonnage par scan… — _le fichier JSON transite encore en texte par l’IPC (acceptable pour un usage perso)_
- [x] **B16** — Inversion d'ordre des verrous entre `spotify_disconnect` et le poller : deadlock possible qui gèle toute…
- [x] **B17** — Rafraîchissement du jeton Spotify : échec silencieux, jeton périmé renvoyé, course entre poller et commandes
- [x] **B18** — Serveur OAuth permanent sur 127.0.0.1:8888, `state` non imposé, paramètres reflétés dans le HTML de réponse

### Étape 7 — Fonctions DJ exactes

_Critère de sortie : Touche 3 = cue 3, recherche « You'll », XML valide._

- [x] **C11** — Hot cues décalés d'un cran : le backend indexe en base 1, le front accepte index, index−1 et index+1 ; le… — _les cues existants sont convertis en base 0 automatiquement à la prochaine synchro Mixed In Key_
- [x] **C12** — Export Rekordbox XML invalide dès qu'un chemin contient `&`, `"` ou `<` ; URL non encodée pour les…
- [x] **C13** — Régression : `loadTracks()` sans argument réutilise le filtre courant
- [x] **C14** — Régression : glisser un tag sur un titre ne fait plus rien (`data-track-id` retiré)
- [x] **C15** — Le moteur Rust et la préécoute HTML peuvent jouer en même temps
- [x] **B27** — Aucun code n'écrit `waveform_data` : la waveform affichée est toujours le motif factice de 64 barres — _calcul à la demande (86 ms pour un MP3 de 2,4 Mo), mis en cache_
- [x] **B28** — FTS5 : la sanitisation rend introuvables les titres avec apostrophe, tiret ou slash, et `AND`, `OR`, `NOT`…
- [x] **B29** — Position de lecture = horloge murale × vitesse, périmée en fin de piste (pas d'horloge CoreAudio… — _position rodio exacte à vitesse normale ; estimation horloge × vitesse conservée quand le tempo est modifié_
- [x] **B30** — Associations de fichiers en `rank: Default` avec le parapluie `public.audio` ; bundle `com.crate.app`… — _Crate apparaît dans « Ouvrir avec » sans devenir le lecteur par défaut (question ouverte du rapport)_
- [x] **B31** — `StartupFile` : course au démarrage, plusieurs fichiers ouverts s'écrasent
- [x] **B32** — `delete_tracks_and_files` : corbeille via `osascript` par fichier, échecs silencieux, suppression…
- [ ] **B33** — I/O bloquantes, sous-processus (`beatportdl`, `lsof`, `osascript`) et rusqlite sous `std::sync::Mutex`… — _partiel : beatportdl hors du runtime async, lecture MIK et décodage de waveform hors du verrou ; les commandes restent synchrones sous le capot_
- [x] **B34** — Services instanciés deux fois : l'état géré par Tauri n'est pas celui des tâches de fond
- [x] **B35** — `get_duplicate_count` lance un scan complet des doublons à chaque événement `duplicates-updated`
- [x] **F1** — Touches 1 à 8 capturées partout, sans condition de vue ni de modificateur
- [x] **F2** — Espace en vue Player lance un titre récent au lieu de mettre en pause la préécoute ; logique dupliquée
- [x] **F3** — Espace ou Entrée sur une ligne focalisée déclenche la lecture et le raccourci global
- [x] **F11** — Cues et waveform jamais chargés pour un fichier externe : un UUID est envoyé à `get_track_cues`

### Étape 8 — Frontend robuste

_Critère de sortie : Erreurs backend visibles, pas d'appel IPC superflu._

- [x] **F4** — `withTimeout` : une initialisation de plus de 2,5 s perd les fonctions de nettoyage (écouteurs Tauri…
- [x] **F5** — Splash fermé par un timer de module à 1,5 s : course avec le chargement des réglages, flash possible de…
- [x] **F7** — L'effet du Toolbar déclenche un appel `get_duplicate_count` à chaque mutation de la bibliothèque
- [x] **F8** — Beatport : favoris et playlists « créés » uniquement en local avec un toast de succès trompeur ; panier… — _favoris et playlists restent locaux, mais annoncés comme tels_
- [x] **F9** — Effets de bord à l'import de modules dans `shared/` (timer Beatport de 3 min, timer du splash) et…
- [x] **F10** — Mutation d'une valeur `$derived` (`activeHeroTrack.is_in_library = true`)
- [x] **F12** — Course de réponses obsolètes dans le suivi de position (intervalle async appelant `getPlaybackState`…
- [x] **F13** — « Synchroniser avec Mixed In Key » du menu contextuel ignore la sélection et resynchronise toute la base
- [x] **F14** — Upgrader : la confiance s'affiche « 0.96% » au lieu de « 96 % » ; le bouton de remplacement reste actif… — _le « 0.96% » vu à l’audit venait des données fictives du harnais, pas de l’app ; bouton désactivé sans session Beatport_
- [x] **F15** — L'infobulle du badge Mixed In Key reste affichée après le départ du pointeur — _corrigé dans le composant commun `Tooltip` (toutes les infobulles)_
- [x] **I1** — Messages d'erreur backend perdus : Tauri rejette avec une chaîne, et les nouveaux stores testent… — _helper appliqué à tous les stores et composants (amont compris)_
- [x] **I2** — Deux réglages Beatport sans effet : le téléchargeur force `lossless` et lance toujours la synchro Mixed In Key — _réglages sans effet retirés de l’interface (format FLAC imposé par la vérification, synchro MIK assurée par le watcher)_
- [x] **I4** — `BeatportAuthState` porte deux conventions (camelCase côté TS, snake\_case côté Rust) ; la cohérence…
- [x] **I5** — `record_listen_event` accepte un événement complet depuis le webview mais n'est appelé nulle part
- [x] **I7** — `library-updated` émis sans écouteur ; paramètre `searchType` de la recherche Beatport ignoré
- [x] **I8** — Une vingtaine de champs `Option<T>` Rust typés `?: T` côté TS alors que la valeur réelle est `null` — _types Beatport ; les types album acceptaient déjà `null`_
- [x] **I9** — Options d'affichage stockées uniquement dans `localStorage` : ni sauvegarde, ni synchro

### Étape 9 — Hygiène et outillage

_Critère de sortie : CI verte, fmt/lint propres, builds multiplateformes._

- [ ] **Q2** — Build iOS/Android cassé : `services::beatport`, `services::stats` et deux commandes de bibliothèque ne… — _reporté : la vérification exige les cibles iOS/Android et Xcode, absents ; priorité basse (Crate n’est utilisé que sur ce Mac)_
- [x] **Q3** — Build Windows/Linux cassé : `RunEvent::Opened` n'existe que sur macOS, iOS et Android — _code corrigé ; compilation Linux vérifiée par le job Rust de la CI (Windows non vérifié)_
- [x] **Q4** — Clippy avec `-D warnings` : 37 erreurs (16 de code mort) ; cargo fmt : 35 fichiers
- [x] **Q5** — ESLint : 38 erreurs ; Prettier : 60 fichiers
- [x] **Q6** — Aucun workflow n'exécute `yarn test` ni `cargo test` — _jobs Linux : frontend (format, lint, types, Vitest) et Rust (clippy, tests) ; la matrice macOS/Windows de l’amont reste manuelle_
- [x] **Q7** — Tests absents sur les chemins à risque : remplacement de fichiers, purge, pollers, OAuth ; 3 tests lisent… — _tests ajoutés sur remplacement de fichier, synchro/purge, trackers d’écoute, import, export ; tests sur la vraie base MIK supprimés_
- [x] **Q8** — Vitest tourne sur Vite 8.2 alors que l'app utilise Vite 7.3 ; `test:coverage` sans fournisseur de…
- [ ] **Q9** — Icônes dev, staging et prod devenues identiques octet pour octet ; `.ico`, icônes Windows, iOS et Android…
- [x] **Q10** — CHANGELOG, version, README et site de documentation non mis à jour ; la doc des raccourcis contredit le… — _CHANGELOG, README et page des raccourcis à jour ; version 0.3.0 à poser au premier build personnel_
- [x] **Q11** — Fichiers à ne pas commiter : `SYNTHESE_DISCUSSION.md` (chemins personnels),… — _script Swift supprimé ; la synthèse est archivée dans `suivi/historique/` (dépôt privé) ; polices traitées avec D8_
- [x] **Q12** — Pas de `CLAUDE.md` : chaque assistant redécouvre les règles (feature `desktop`, flags clippy de la CI,…
- [x] **Q13** — `yarn dev` compile le Rust en `--release` : chaque modification coûte plusieurs minutes (amont)
- [ ] **Q14** — 87 alertes `yarn audit` (61 hautes) dans l'outillage transitif ; `cargo audit` non installé localement
- [x] **B10** — Numérotation des migrations divergente de l'amont (15 entrées, libellées 7 à 16, la 6 sautée) — _libellés corrigés (6 à 15), ordre inchangé : la base locale a déjà appliqué ces migrations_

### Étape 10 — Fondations visuelles

_Critère de sortie : Aucun élément invisible en clair ou sombre._

- [x] **D1** — `dark:` suit le thème de l'OS, pas celui de l'app : les « corrections mode clair » du build 40 sont… — _variante `dark:` rebranchée sur `[data-theme]` (une ligne CSS) ; le nettoyage des 43 usages se fera vue par vue (étape 11)_
- [x] **D2** — Environ 70 classes référencent des tokens inexistants (`surface-3`, `surface-4`, `stroke-strong`,…
- [x] **D6** — Modales plus hautes ou plus larges que la fenêtre : le pied du Duplicate Killer et son bouton « Supprimer…
- [x] **D8** — Polices Jost et DM Sans proposées dans les réglages mais non câblées (`[data-font='jost']` absent) ; 15… — _licences OFL référencées dans `static/fonts/LICENSES.md` (texte complet via le lien officiel)_
- [x] **D9** — Six icônes utilisées n'existent pas dans `Icon.svelte` ; le logo Beatport SVG chargé en `<img>` avec… — _nom d’icône non typé, mais un test Vitest échoue sur toute icône inconnue_

### Étape 11 — Conformité vue par vue

_Critère de sortie : Règles de design strictes respectées._

- [ ] **D3** — Thème clair cassé : cartes d'intégration de Pulse sombres en haut et claires en bas ; badges de l'en-tête…
- [x] **D4** — En-tête à la largeur minimale (1000 px) : les icônes chevauchent le contrôle segmenté… — _vérifié par mesure dans le harnais à 1000, 1280, 1440 et 1600 px : aucun chevauchement, tous les outils visibles_
- [x] **D5** — Hero du Player à hauteur fixe (`h-[225px]` pour environ 260 px de contenu) : à 1000×640 le transport est… — _transport jamais recouvert (vérifié à 1000×640) ; à cette taille la liste des récents garde ~2 lignes_
- [ ] **D7** — Tableau Beatport à 1000 px : la colonne titre se réduit à un caractère
- [ ] **D10** — Accessibilité : environ 25 boutons icône sans nom (transport, segments, badge MIK, actions des récents) ;…
- [ ] **D11** — Composants réinventés au lieu des communs : 4 contrôles segmentés, checkbox, select, spinner, tooltip et…
- [x] **D12** — Badge « Build 57 » permanent à côté du logo et libellés « PRO » sur les marques tierces

### Étape 12 — Traduction

_Critère de sortie : App en anglais : aucune chaîne française._

- [ ] **L1** — 17 composants nouveaux sans aucune traduction, environ 220 chaînes
- [ ] **L2** — 13 locales sans les 49 à 53 clés ajoutées par le fork
- [ ] **L3** — Nombres et unités formatés à l'anglaise en français (« 2,310 »), « plays » et « écoutes » mélangés dans Pulse
- [ ] **L4** — Messages d'erreur Rust en français
- [ ] **L5** — `register('en')` et `register('fr')` conservés après `addMessages` : l'initialisation repasse par le…
- [x] **L6** — Le README annonce 11 langues alors que le dépôt en contient 15
## Journal des sessions

| Date | Travail | Étapes |
| --- | --- | --- |
| 2026-09-25 | Audit complet : rapport, registre de ~180 défauts, harnais navigateur | — |
| 2026-09-26 | Sauvegardes, secrets retirés, commit de base, dépôt GitHub, documents de suivi | 1 |
| 2026-09-26 | Plus aucune suppression automatique ni écriture dans Mixed In Key ; config prod restaurée | 2 |
| 2026-09-26 | Session Beatport dans le Trousseau, fin du scraping DJ.Studio, secret Spotify jamais renvoyé au webview | 3 |
| 2026-09-26 | Upgrader sûr : dossier isolé, remplacement en place, FLAC décodé en entier, scoring corrigé | 4 |
| 2026-09-26 | Synchro Mixed In Key incrémentale et idempotente, cues stables, import/Serato/énergie corrigés | 5 |
| 2026-09-26 | Statistiques justes : une écoute Spotify = une ligne, pauses exclues, sets Rekordbox datés | 6 |
| 2026-09-26 | Fonctions DJ exactes : hot cues, XML, waveform réelle, recherche, position, régressions, corbeille | 7 |
| 2026-09-26 | Frontend robuste : erreurs visibles, init sans fuite, panier fiable, options d’affichage sauvegardées | 8 |
| 2026-09-26 | Hygiène : fmt/clippy/ESLint à zéro, CI Linux, hooks git rétablis | 9 |
| 2026-09-26 | Fondations visuelles : tokens, `dark:` lié au thème, modales bornées, polices, icônes | 10 |
