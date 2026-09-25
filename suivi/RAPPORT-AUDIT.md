> **Instantané figé de l’audit du 25 septembre 2026.** Ce document décrit l’état du code *avant* les corrections. L’avancement vivant est dans [AVANCEMENT.md](AVANCEMENT.md) et le journal des modifications dans [CHANGELOG.md](../CHANGELOG.md). Version éditable : [Crate — Audit complet & Vision](https://claude.ai/code/artifact/9a600818-17fb-44e6-b7c1-6924af9c11f5).

# Crate — Audit complet & Vision


## Résumé exécutif

Verdict : Crate est une excellente base et le fork a de très bonnes idées, mais il n'est pas sûr à utiliser tel quel sur votre vraie bibliothèque. Il faut d'abord consolider, puis polir, et seulement ensuite ajouter des fonctions.

- **Le socle amont est solide** : chiffrement, migrations, contrat IPC, système de design et CI sont de niveau professionnel.
- **Le fork fonctionne** : l'app démarre, tourne 25 minutes sans erreur, les 371 tests passent et le contrat backend ↔ frontend est parfaitement aligné.
- **Mais il peut détruire des données** : purge automatique des titres absents de Mixed In Key, écritures dans la base de Mixed In Key, upgrader qui vide des dossiers et perd les cues.
- **Et plusieurs choses affichées sont fausses** : hot cues décalés, waveform factice, statistiques Spotify doublées, recherche qui ignore les apostrophes.
- **Un mot de passe Beatport est en clair dans le code** et 27 800 lignes de travail ne sont dans aucun commit.

| À retenir | Valeur |
| --- | --- |
| Défauts critiques uniques | 15 |
| Lignes ajoutées non commitées | environ 27 800 |
| Contrôles CI qui échoueraient | 5 sur 8 |

**Recommandation** : scénario A (consolider, environ 3 semaines) puis scénario B (polir et unifier, environ 2 mois). La première journée sert uniquement à mettre le travail en sûreté : changer le mot de passe, sauvegarder les bases, commiter sur une branche locale, couper la purge. Le détail de chaque défaut et de son correctif est dans le document compagnon, le registre des défauts.

## Ce qu'est Crate aujourd'hui

Crate est un gestionnaire de bibliothèque DJ de bureau, version 0.2.9, bâti sur Tauri 2 : un backend Rust, un frontend SvelteKit en Svelte 5 (runes), une base SQLite chiffrée par SQLCipher. Le dépôt est un fork personnel de `blackboxaudio/crate`, branche `develop` alignée sur `origin/develop`.

| Couche | Technologie | Volume mesuré le 25 sept. 2026 |
| --- | --- | --- |
| Backend | Rust, rusqlite + SQLCipher, rodio/symphonia, tokio | 54 260 lignes, 125 commandes IPC, 16 migrations SQL |
| Frontend desktop | SvelteKit, Svelte 5, Tailwind | 33 560 lignes, 139 composants |
| Code partagé | TypeScript (API IPC, stores, types, i18n, utils) | 13 833 lignes |
| Mobile | SvelteKit (squelette iOS/Android) | 15 lignes de source |
| Langues | JSON svelte-i18n | 15 fichiers de locale (le README en annonce 11) |
| Tests | cargo test, Vitest | 227 tests Rust, 144 tests Vitest |

**Périmètre hérité de l'amont** : import et extraction de métadonnées (MP3, FLAC, WAV, AIFF, M4A), tags et catégories, playlists manuelles et intelligentes, analyse (waveform, clé, BPM, énergie), découverte Bandcamp/SoundCloud/YouTube/Discogs avec suivi d'artistes et de labels, lecture avec cues, export USB Pioneer CDJ/XDJ avec génération de base Rekordbox, synchronisation incrémentale d'appareils, édition de métadonnées en masse, thèmes et accents, sauvegardes, synchronisation cloud optionnelle, mises à jour automatiques.

**Périmètre ajouté par le fork** (non commité, détaillé à la section suivante) : lecteur autonome avec association des fichiers audio macOS, lecture directe de la base Mixed In Key 11 (`Collection11.mikdb`, cues et énergie), remplacement MP3 vers FLAC via Beatport, tableau de bord de statistiques d'écoute multi-sources « Crate Pulse » (Spotify, écoutes locales, Mixed In Key, sessions Rekordbox), détection de doublons, vue albums, recherche plein texte FTS5, hot cues 1 à 8, assistant de mix harmonique, raccourcis clavier DJ, export Rekordbox XML.

**Environnement réel du propriétaire** : base `crate.db` de 2,6 Mo (bibliothèque de quelques centaines de titres), 419 pochettes en cache, sauvegarde automatique de 73 Mo, Rekordbox 7, Mixed In Key 11 et Spotify installés sur la machine. La taille modeste de la bibliothèque rend les optimisations « 10 000 titres » moins urgentes que la fiabilité et la finition.

## Ce qui a été fait dans le fork personnel

Tout le travail des builds 36 à 57 existe dans le code et fonctionne (l'app installée le 14 septembre correspond exactement à l'arbre de travail actuel), mais rien n'est commité : 174 chemins en attente, zéro commit d'avance sur `origin/develop`.

| Thème | Builds | Ce qui existe dans le code | Fichiers clés |
| --- | --- | --- | --- |
| Lecteur autonome et lecteur par défaut macOS | 36 à 40 | Vue `PlayerView`, fichiers récents, association des types audio dans `tauri.conf.json`, script Swift `set_default_player.swift`, gestion du fichier ouvert au démarrage | `components/player/PlayerView.svelte`, `services/standalone.rs`, `commands/standalone.rs` |
| Intégration Mixed In Key 11 | 1 puis 56 | Lecture de `Collection11.mikdb`, synchronisation des cues et de l'énergie, watcher de fichier, signets macOS | `services/library/mik_db.rs`, `mik.rs`, `macos_bookmark.rs` |
| Beatport Quality Upgrader | 41 à 44 | Recherche multi-passes, scoring, téléchargement FLAC, validateur d'intégrité, cache SQLite, migrations 11 et 12 | `services/beatport/`, `components/upgrader/`, `components/beatport/` |
| Crate Pulse (statistiques) | 45 à 48 | Table `listen_events`, trackers Spotify (OAuth PKCE loopback), Rekordbox, Mixed In Key, lecteur local ; KPI, tops, roue Camelot, heatmap ; migration 13 | `services/stats/`, `components/stats/` |
| Doublons et albums | non datés | Détection de doublons avec groupes ignorés, vue albums du lecteur | `services/duplicate.rs`, `services/album.rs` |
| Refonte DJ Pro | 57 | FTS5 (migration 16), waveform à la demande, position audio par échantillons, hot cues 1 à 8, mix harmonique, raccourcis, export Rekordbox XML | `db/schema.rs`, `services/library/query.rs`, `services/audio/mod.rs`, `services/export/rekordbox_xml.rs` |
| Outillage | 56 à 57 | Vitest, testing-library, jsdom, 11 fichiers de tests TS, i18n synchrone en/fr | `vitest.config.ts`, `shared/i18n/index.ts` |

| État git au 25 sept. 2026 | Valeur |
| --- | --- |
| Fichiers suivis modifiés | 100 (+4 835 / −1 035 lignes) |
| Chemins non suivis | 74 |
| Code nouveau non suivi | environ 14 000 lignes |
| Commits d'avance sur l'amont | 0 |
| Branche, stash, sauvegarde du travail | aucune |

Deux changements de configuration sont passés sous silence dans la synthèse précédente : `tauri.prod.conf.json` ne produit plus que le bundle `.app` et désactive les artefacts de mise à jour, et les icônes des trois variantes (dev, staging, prod) ont été remplacées. Le `CHANGELOG.md` n'a pas été touché.

## Points forts

Le socle amont est de très bonne qualité, et le fork a apporté de vraies bonnes idées : le problème n'est pas la vision, c'est la finition et la sûreté.

**Ce que l'amont fait bien et qu'il faut préserver**

- **Sécurité des données** : SQLCipher partout, abstraction de clé avec garde de compilation, Keychain iOS et Keystore Android correctement paramétrés.
- **Surface Tauri minimale** : permissions scopées, pas de plugin shell ni http côté webview, protocole d'assets limité au dossier de données.
- **Migrations transactionnelles** : chaque migration et son numéro de version sont validés dans la même transaction.
- **Vrai système de design** : trois surfaces, trois niveaux de texte, couleur de marque pilotée par l'accent choisi, composants communs (Text, Button, Modal, Tooltip…).
- **CI complète** : lint TypeScript et Rust, svelte-check desktop et mobile, builds macOS et Windows, vérification iOS/Android, audit de dépendances, release verrouillée par le CHANGELOG.

**Ce que le fork a bien fait**

- **Contrat IPC impeccable** : les 230 commandes sont enregistrées, les 226 appels frontend trouvent leur commande, zéro désaccord d'arguments, tout passe par `shared/api`.
- **Types miroirs** : les nouveaux modèles Rust (stats, upgrader, doublons, albums, lecteur) correspondent champ par champ aux interfaces TypeScript.
- **Waveform à la demande** : `get_tracks` ne transporte plus le blob, un seul consommateur gardé.
- **Correctifs réels** : le crash i18n au démarrage et la boucle CPU du watcher Mixed In Key sont bien résolus (CPU mesuré à 0,1 % au repos).
- **Garde-fous métier** : validation FLAC (signature et taille) avant suppression d'un MP3, lecture de la base Mixed In Key en lecture seule pour la synchronisation, PKCE avec `state` persisté pour Spotify, Rekordbox sans clé embarquée.
- **Tests ajoutés** : 144 tests Vitest verts en 3 secondes, 52 nouveaux tests Rust majoritairement en base mémoire.
- **Produit** : Crate Pulse est visuellement réussi en thème sombre, le Duplicate Killer et l'Upgrader présentent des comparaisons claires, la vue Player avec pads de cues est agréable à utiliser.

## Santé technique mesurée

L'app compile, démarre et tourne sans crash sur votre machine, mais 5 des 8 contrôles de la CI amont échoueraient si l'arbre était commité tel quel. Tout ce qui suit a été exécuté le 25 septembre 2026.

| Contrôle | Résultat | Détail |
| --- | --- | --- |
| Vitest | OK | 144 tests sur 11 fichiers, tous verts |
| svelte-check desktop et mobile | OK | 0 erreur, 2 avertissements d'accessibilité |
| cargo check (desktop, release) | OK | 15 avertissements |
| cargo test | OK mais dangereux | 227 sur 227 ; 2 tests écrivent dans vos vraies bases Crate et Mixed In Key |
| cargo clippy avec -D warnings (flags de la CI) | Échec | 37 erreurs, surtout du code mort |
| ESLint | Échec | 38 erreurs (clés manquantes dans les `each`, `any`) |
| Prettier et cargo fmt | Échec | 60 fichiers TS et 35 fichiers Rust non formatés |
| Build iOS/Android | Échec | 10 erreurs : modules nouveaux non protégés par la feature `desktop` |
| Build Windows/Linux | Échec (analyse) | `RunEvent::Opened` n'existe que sur macOS |
| Contrat IPC | OK | 230 commandes enregistrées, 226 invoquées, 0 appel orphelin, 0 désaccord d'arguments |
| Événements | OK | 22 sur 23 appariés ; `library-updated` émis sans écouteur |
| i18n | Partiel | 17 nouveaux composants sans aucune traduction ; 13 locales sur 15 n'ont pas 49 à 53 clés |

**Exécution réelle de `/Applications/Crate.app`** pendant 25 minutes avec logs détaillés :

- Aucun panic, aucune erreur, CPU à 0,1 % au repos.
- La synchronisation Mixed In Key s'exécute 4 fois en 14 secondes au démarrage, puis encore 11 minutes plus tard. Chaque passage réécrit vos 276 titres sans détecter de changement, relit les fichiers audio et relance la recherche de pochettes. Chaque passage déclenche aussi la purge stricte décrite dans le registre des défauts.
- Une « réparation » des écoutes Spotify tourne à chaque démarrage. Elle transforme des passages courts en écoutes complètes.
- Le serveur OAuth Spotify écoute en permanence sur le port 8888 dès le lancement.

**Rendu visuel** : j'ai construit un harnais qui fait tourner le frontend dans un navigateur avec un faux backend Tauri et 16 pistes fictives. Il a permis de capturer chaque vue en thème clair et sombre, en français et en anglais, à 1000×640, 1400×900 et 1920×1080. Les défauts visuels constatés sont détaillés dans le registre. Le harnais est réutilisable pour des tests automatiques.

## Faiblesses principales

Le fork a été construit vite et en largeur : chaque fonction marche dans le cas nominal, mais plusieurs peuvent détruire des données, et l'ensemble a perdu la cohérence visuelle et linguistique de l'amont. Les six audits ont produit environ 250 constats bruts ; le registre des défauts les regroupe et les classe.

**Les sept problèmes qui comptent le plus pour vous**

1. **Risque de perte de données dans votre bibliothèque.** Au démarrage et à chaque focus de fenêtre, tout titre Crate absent de Mixed In Key est supprimé. L'upgrader peut vider des sous-dossiers de votre dossier musique et perd les cues, tags, playlists et notes du titre remplacé.
2. **Écritures dans la base de Mixed In Key.** Crate supprime des lignes dans `Collection11.mikdb`, une base Core Data tierce, sans sauvegarde ni accord. Deux tests Rust font la même chose sur vos vraies bases.
3. **Un mot de passe Beatport en clair dans le code source.** Il est aussi réécrit sur disque à chaque téléchargement. Il doit être changé avant tout commit.
4. **Statistiques faussées.** Les écoutes Spotify sont comptées deux fois, et les passages de moins de 30 secondes deviennent des écoutes complètes à chaque démarrage. Le tracker Mixed In Key compte un fichier ouvert comme une écoute.
5. **Fonctions DJ fausses.** Les hot cues sont décalés d'un cran (la touche 3 saute au cue 2). La « waveform réelle » n'a pas de source : aucun code n'écrit `waveform_data`, c'est donc un motif factice qui s'affiche. L'export Rekordbox XML produit un fichier invalide dès qu'un chemin contient `&`. La recherche FTS5 ne trouve plus les titres avec apostrophe ou tiret.
6. **Régressions de l'amont.** Retirer le dernier filtre de tag ne vide plus la liste, le glisser-déposer d'un tag sur un titre ne fait plus rien, et le moteur Rust peut jouer en même temps que la préécoute.
7. **Incohérence visuelle et linguistique.** Trois langages visuels cohabitent, aucune nouvelle vue ne suit la couleur d'accent, le thème clair est cassé sur Crate Pulse, et toutes les nouvelles vues restent en français quand l'app est en anglais.

**Risque juridique à connaître** : le téléchargement FLAC ne passe pas par vos achats Beatport. Il pilote l'outil tiers `beatportdl`, qui récupère les flux de l'abonnement streaming, avec le client OAuth de la documentation Beatport. C'est contraire aux conditions de Beatport. Pour un usage strictement personnel, c'est votre décision ; ce code ne doit en revanche jamais être poussé sur un dépôt public.

**Affirmations de la synthèse précédente qui sont fausses** : la position audio n'est pas asservie à l'horloge CoreAudio, c'est une horloge murale multipliée par la vitesse. Il y a 15 migrations, pas 16. Le watcher Mixed In Key n'est pas « en lecture pure ». Le volume ajouté est d'environ 27 800 lignes, pas 14 000.

## Ce que Crate pourrait devenir

Crate a tout pour devenir le cockpit personnel d'un DJ : la bibliothèque de référence, l'outil de préparation et le tableau de bord de ce que l'on écoute et joue réellement. Il ne remplace ni Rekordbox (mixer) ni Mixed In Key (analyser) : il les relie et il mesure. La bonne direction n'est pas d'ajouter des fonctions, le fork en a déjà trop pour sa maturité, mais de rendre chacune irréprochable et cohérente.

**Trois piliers, alignés sur ce que vous aimez**

| Pilier | Promesse | Ce qui existe déjà | Ce qui manque |
| --- | --- | --- | --- |
| Bibliothèque irréprochable | Tout au même endroit, rangé, sans doublon, en qualité maximale, cohérent avec Mixed In Key et Rekordbox | Import, tags, playlists intelligentes, doublons, upgrader FLAC, lecture de Collection11.mikdb, FTS5 | Rangement physique des fichiers, rapport de divergences clé/BPM/cues entre les trois outils, empreinte acoustique pour les doublons |
| Statistiques personnelles | Savoir ce que l'on écoute et ce que l'on joue, sur toutes les sources, avec des insights et un historique fiable | Crate Pulse : Spotify, local, Mixed In Key, sessions Rekordbox, heatmap, roue Camelot | Récapitulatifs (semaine, mois, année), entonnoir découverte → bibliothèque → joué en set, export des données, fiabilité du comptage entre sources |
| Préparation DJ | Construire un set, vérifier l'enchainement harmonique et énergétique, exporter proprement | Hot cues, mix harmonique, raccourcis, export USB et XML | Vue « set » avec courbe clé/énergie, suggestion du morceau suivant fondée sur vos sets réels, écriture des cues vers Mixed In Key |

**Trois scénarios**

| Scénario | Ampleur | Durée indicative | Contenu | Résultat |
| --- | --- | --- | --- | --- |
| A. Consolider | Peu de changements | 2 à 3 semaines | Commiter, corriger les défauts critiques et majeurs, traduire les nouvelles vues, réparer le responsive et le thème clair, aucune nouvelle fonction | Une version personnelle 0.3.0 stable et honnête |
| B. Polir et unifier | Changements moyens | 1 à 2 mois après A | Un seul langage visuel piloté par la couleur d'accent, Crate Pulse v2 (récapitulatifs, exports, entonnoir), synchronisation Mixed In Key incrémentale, upgrader sécurisé, mode « set » | L'app que vous décrivez : jolie, cohérente, qui fonctionne |
| C. Réinventer | Beaucoup de changements | 3 à 6 mois après B | Sources de statistiques enfichables, compagnon mobile pour les stats, empreinte acoustique, tagging assisté (énergie, ambiance), fenêtre Pulse détachable | Un produit complet, au prix d'un chantier long et risqué pour un projet personnel |

Recommandation : A puis B. Le scénario A est un préalable non négociable (14 000 lignes non commitées, défauts critiques ouverts). Le scénario B est celui qui maximise le plaisir d'usage quotidien pour une personne qui aime « les petites tech qui fonctionnent ». Le scénario C ne se justifie que si l'app devient un projet partagé ou public.

**Idées à fort rapport valeur/effort pour vous**

| Idée | Pilier | Pourquoi c'est bien pour vous | Effort |
| --- | --- | --- | --- |
| « Votre semaine » et « Votre année » : récapitulatif automatique avec top titres, artistes, clés, heures de pointe, et image partageable | Stats | Le plaisir Stats.fm, mais sur toutes vos sources y compris vos sets | M |
| Entonnoir découverte → bibliothèque → joué en set | Stats | Mesure ce que vos découvertes deviennent réellement | M |
| Timeline des sessions Rekordbox avec tracklist, flux de clés et courbe d'énergie | Stats + Préparation | Relire un set comme un match, apprendre de ses enchainements | M |
| Export CSV/JSON de tout l'historique d'écoute | Stats | Vos données vous appartiennent, sauvegarde indépendante | S |
| Rapport de divergences Crate / Mixed In Key / Rekordbox (clé, BPM, cues, fichiers manquants) | Bibliothèque | Tout organisé et cohérent sur l'ordinateur | M |
| Rangement physique assisté : règle de nommage et d'arborescence avec simulation avant déplacement | Bibliothèque | La bibliothèque sur disque devient aussi propre que dans l'app | M |
| Playlists intelligentes fondées sur les stats (« jamais joué en set », « top 30 jours », « découvert ce mois ») | Bibliothèque + Stats | Réunit organisation et statistiques dans un même geste | S |
| Mode « set » : sélection, ordre, vérification harmonique et énergétique, export XML/USB en un clic | Préparation | Le mix harmonique devient un flux de travail complet | L |
| Suggestion du morceau suivant fondée sur vos enchainements réels (sessions Rekordbox) | Préparation + Stats | Personnel, pas générique | M |
| Tests de bout en bout sur le harnais navigateur construit pour cet audit | Fiabilité | Chaque vue vérifiée automatiquement en clair, sombre et petite fenêtre | M |

**Ce qu'il faudrait éviter** : ouvrir de nouveaux chantiers (mobile, IA, nouvelles sources) avant que les vues existantes partagent un langage visuel, une langue et une base de tests. Chaque fonction ajoutée par le fork a été livrée avec son propre style, ses propres chaînes en dur et ses propres raccourcis : c'est ce qui rend l'ensemble moins « petite tech qui fonctionne » qu'il ne le mérite.

## Plan d'implémentation par phases

Le plan suit le scénario recommandé : les phases 0 à 3 forment le scénario A (environ 3 semaines), les phases 4 et 5 le scénario B (environ 2 mois). L'ordre est imposé par le risque : on arrête d'abord ce qui peut détruire des données, on corrige ensuite ce qui est faux, et on embellit en dernier.

| Phase | Objectif | Travaux principaux | Effort | Critère de sortie |
| --- | --- | --- | --- | --- |
| 0. Mise en sûreté | Ne plus rien perdre | Changer le mot de passe Beatport et le retirer du code ; sauvegarder `crate.db` et `Collection11.mikdb` ; créer une branche locale et commiter un instantané complet ; désactiver la purge stricte et les écritures dans Mixed In Key ; marquer `#[ignore]` les deux tests sur bases réelles ; restaurer `tauri.prod.conf.json` | 1 jour | Tout le travail est dans git, aucune opération destructive ne s'exécute au démarrage |
| 1. Arrêter les pertes de données | Chaque opération sur fichiers est sûre | Mixed In Key strictement en lecture ; purge remplacée par un rapport de titres manquants avec confirmation ; upgrader qui remplace le fichier en gardant l'identifiant du titre (cues, tags, playlists conservés) ; téléchargement dans un dossier temporaire isolé ; validation FLAC par décodage complet ; suppression de `flatten_and_clean_destination` | 3 à 4 jours | Tests d'intégration sur base et dossiers temporaires couvrant purge, upgrade et suppression |
| 2. Justesse | Ce qui s'affiche est vrai | Hot cues en base 0 partout ; waveform générée à l'analyse ou lue depuis Mixed In Key ; recherche FTS5 avec échappement correct et repli ; export XML échappé et cues conformes à la spécification ; stats Spotify dédoublonnées, suppression de la « réparation », gestion des pauses ; trois régressions amont ; deux panics IPC ; synchronisation Mixed In Key unique et incrémentale | 4 à 5 jours | Chaque correctif a un test ; recherche « You'll » et export XML avec `&` vérifiés |
| 3. Hygiène et historique propre | Un dépôt sain | Format et lint TS/Rust, code mort, protection `desktop` des modules, correctif Windows ; découpage en 9 commits thématiques ; CHANGELOG, `CLAUDE.md`, icônes régénérées par environnement, polices inutiles retirées | 2 à 3 jours | CI verte sur la branche, version 0.3.0 personnelle installée |
| 4. Unification visuelle et langues | Une seule app, jolie et cohérente | Fondations (tokens manquants, variante `dark:` liée au thème de l'app, polices, icônes) ; mise en conformité Player, Pulse, Beatport, Upgrader, Doublons ; traduction de toutes les chaînes dans les 15 langues ; utilisable à 1000×600 ; accessibilité clavier | 10 à 14 jours | Captures clair et sombre, 2 accents, 2 tailles de fenêtre, sans défaut ; scan CI anti-couleurs en dur |
| 5. Crate Pulse v2 et préparation DJ | Le produit que vous aimez | Récapitulatifs semaine/mois/année ; export CSV/JSON ; entonnoir découverte → set ; playlists intelligentes fondées sur les stats ; rapport de divergences Crate/Mixed In Key/Rekordbox ; mode « set » ; tests de bout en bout sur le harnais navigateur | 4 à 6 semaines | Chaque fonction livrée traduite, testée et conforme aux règles de design |

Les phases 0 et 1 ne sont pas négociables avant d'utiliser l'app au quotidien avec votre vraie bibliothèque. La phase 4 peut commencer en parallèle de la phase 3 sur les fondations visuelles, car elle ne touche pas au backend.

## Décisions à prendre

Cinq décisions vous reviennent avant de lancer la phase 1 ; les autres choix techniques peuvent suivre les recommandations du registre.

| Décision | Options | Recommandation |
| --- | --- | --- |
| Scénario global | A seul, A puis B, ou A puis B puis C | A puis B |
| Téléchargement Beatport (upgrader) | Garder en privé sur une branche jamais poussée ; le brancher uniquement sur vos achats ; le retirer | Branche privée le temps de décider, et jamais sur un dépôt public |
| Rôle de Mixed In Key | Source de vérité qui peut supprimer des titres Crate ; source d'enrichissement en lecture seule | Lecture seule : Crate enrichit, ne supprime jamais sans confirmation |
| Langue de l'interface | Tout traduire dans les 15 langues ; français et anglais seulement | Français et anglais complets, les autres langues en repli anglais |
| Langage visuel | Revenir au système amont piloté par l'accent ; créer une identité « DJ » propre avec ses tokens | Système amont étendu de trois tokens (cue, live, marque) |

**Questions ouvertes**

- [ ] Le dépôt distant `blackboxaudio/crate` vous appartient-il, ou faut-il créer votre propre dépôt privé pour pousser ce travail ?
- [ ] Utilisez-vous Crate sur d'autres machines (Windows, iPhone) ou uniquement sur ce Mac ? Si c'est ce Mac seul, les correctifs Windows et mobile passent en priorité basse.
- [ ] Faut-il conserver l'historique Spotify déjà enregistré, sachant qu'une partie est faussée par le double comptage ? Une réimportation depuis l'export officiel Spotify est possible.
- [ ] Voulez-vous que Crate devienne le lecteur audio par défaut de macOS, ou seulement qu'il apparaisse dans « Ouvrir avec » ?
