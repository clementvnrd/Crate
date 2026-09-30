# Crate — règles pour les assistants de code

Fork personnel de `blackboxaudio/crate` (remote `upstream`), poussé sur le dépôt privé `origin`. Le propriétaire (Clément) écrit en français : documents, CHANGELOG et messages en français (voir §9 pour la question ouverte sur l'anglais).

## 0. Standing rules (priorité maximale)

### 0.1 Capturer les instructions au moment où elles sont données

Dans **chaque chat**, dès que Clément dit quelque chose qui ressemble à une règle sur la façon de construire, lancer, relire ou communiquer sur le projet — une convention, une contrainte, une préférence, un « ne fais jamais X », un « à partir de maintenant » — l'inscrire dans ce fichier **dans la même réponse**, avant ou pendant le travail. **La placer dans la section à laquelle elle appartient, en remplaçant ce qu'elle rend obsolète** — jamais en dessous d'une ancienne règle qu'elle contredit. S'il est ambigu qu'il s'agisse d'une règle durable ou d'un cas ponctuel, l'ajouter en §9 « Open questions » plutôt que de l'ignorer. Les consignes clairement ponctuelles (« juste pour ce test », « pour l'instant ») **ne sont pas** ajoutées ici.

### 0.2 Comment parler à Clément — français, orienté objectif, termes techniques en anglais

Les réponses sont **en français** et répondent à « qu'est-ce que ça change pour moi et qu'est-ce que je fais maintenant », jamais à étaler du détail technique.

**Tout terme technique reste en anglais** (Clément, 2026-09-25) — *commit, push, branch, merge, build, feature flag, migration, schema, IPC, store, hook, lint, snapshot, backup, parsing, fixture…* Écrire « le build desktop passe », pas « la compilation de bureau réussit ». Les titres de section et en-têtes de tableau suivent la même règle. Pourquoi : Clément ne doit jamais avoir à retraduire mentalement du français vers l'anglais.

**Structure de toute réponse substantielle :**

1. **Ouvrir sur l'essentiel** — la situation réelle en une ou deux phrases : débloqué ? bloqué ? qu'est-ce qui a changé ? Jamais de sortie d'outil ou de liste de fichiers en ouverture.
2. **Numéroter les sections**, chacune avec un titre qui énonce son point.
3. **Expliquer le *pourquoi*, pas seulement le *quoi*.** Après chaque fait technique, ajouter la conséquence en clair (« Ce que cela signifie : … »).
4. **Tableaux avant/après** dès que quelque chose a changé, avec une colonne *« Pourquoi c'est mieux »*.
5. **Conclure sur le livrable concret** — ce qui est produit, comment le lancer, quel résultat prouve que ça marche.

**Toute commande à lancer indique depuis quel dossier** (Clément, 2026-09-30) : `yarn …` depuis la racine du dépôt (`~/Coding Projects/crate`), `cargo …` depuis `src-tauri/`. Pourquoi : une commande sans son emplacement est ambiguë et finit lancée au mauvais endroit.

**À éviter :** faire d'une sortie brute, de chemins ou de numéros de ligne la substance d'une réponse (ce sont des preuves *à l'intérieur* d'une phrase qui énonce déjà le point) ; des tableaux denses de paramètres sans interprétation ; ouvrir sur des réserves avant le message principal ; être laconique au point d'être cryptique.

**Le registre attendu** (modèle de réponse de Clément, 2026-09-22, abrégé) :

> Excellente nouvelle ! Tout s'éclaire et la situation est désormais parfaitement débloquée. […] Voici le résumé clair et structuré de ce qui s'est passé, de ce qui a changé, et de ce qu'il te reste à faire.
>
> **1. La grande nouveauté : le problème est résolu !** […]
> *Ce que cela signifie : …*
>
> **2. Ce qui a changé entre hier et aujourd'hui**
> | Sujet | Avant | Maintenant | Pourquoi c'est mieux |
>
> **3. Ta mission : ce qu'il faut livrer** […]

### 0.3 Garder ce fichier léger — hygiene check après chaque tâche majeure (Clément, 2026-09-29)

Après chaque **tâche majeure** — une série de corrections du registre, un push, une décision prise, une refonte de vue — et avant le rapport final, relire ce fichier en entier et vérifier quatre points :

1. **Rien n'est périmé.** Chaque version, date, statut et « reste à faire » correspond aux documents et à l'état réel. Les points réglés sont supprimés, pas barrés.
2. **Rien de volatil n'est codé en dur.** Hashes de commit, nombre de tests, progression, « état au … » vivent dans `suivi/AVANCEMENT.md`, `suivi/REGISTRE-DEFAUTS.md` et `CHANGELOG.md`, ou se lisent par une commande (`yarn suivi`, `git log`). Ce fichier contient des règles durables et des faits contraignants, et pointe vers l'endroit où vivent les éléments mouvants.
3. **Rien n'est dupliqué ni contradictoire** entre sections, ni entre ce fichier et `DESIGN.md` / `suivi/` — « un fait, un endroit ».
4. **Chaque ajout est dans la bonne section**, en remplaçant ce qu'il rend obsolète plutôt qu'ajouté en dessous.

Terminer le rapport par une ligne : « CLAUDE.md : checked — N changes » ou « checked — no change ». Supprimer ou reformuler une règle donnée par Clément se **propose**, ne se fait jamais en silence.

## Suivi obligatoire

- Toute correction référence un identifiant du registre (`suivi/REGISTRE-DEFAUTS.md`) : `fix(zone): description [B12]`.
- Dans le **même commit** : cocher la case dans `suivi/AVANCEMENT.md`, lancer `yarn suivi`, ajouter l'entrée au `CHANGELOG.md` (section « Fork personnel »).
- Mettre à jour le README et les autres `.md` si le comportement documenté change.

## Règles techniques

- Rust desktop (depuis `src-tauri/`) : la feature `desktop` n'est pas par défaut. `cargo test --features desktop`, `cargo clippy --features desktop -- -D warnings`.
- Tout module desktop-only est protégé par `#[cfg(feature = "desktop")]` (le build mobile utilise `--no-default-features --features mobile`).
- IPC Tauri : paramètres Rust en snake_case, clés JS en camelCase ; un rejet IPC est une **chaîne**, jamais un `Error`.
- Migrations : uniquement ajoutées en fin de `schema::get_migrations()`, jamais renumérotées après publication.
- **Jamais** de test qui ouvre `~/Library/...` ou les vraies bases Crate/Mixed In Key : `tempfile`/`:memory:` uniquement.
- **Mixed In Key est en lecture seule** : aucune écriture dans `Collection11.mikdb`.
- **Aucun secret** dans le code (identifiants, jetons, mots de passe).
- Frontend : thème via `[data-theme]` (pas de `dark:`), couleurs par tokens, composants communs (`Button`, `Modal`, `Tooltip`…), aucune chaîne en dur (clé i18n `en.json` + `fr.json`).
- Design : `DESIGN.md` fait foi. Tout travail d'interface (audit, correction D*, refonte, nouvelle vue, maquette) se délègue à l'agent `design` (`.claude/agents/design.md`) ; `yarn design:scan <fichiers>` ne doit montrer aucune nouvelle occurrence.
- Tests (depuis la racine) : `yarn test` (Vitest), `yarn check:svelte`.

## 9. Open questions

- **Documents `.md` en anglais ?** (règle 2026-09-25 venue d'un autre projet : « tout `.md` est écrit en anglais, les documents français sont réécrits à leur prochaine mise à jour »). Elle contredit la règle actuelle de Crate (documents et CHANGELOG en français) et casserait `yarn suivi`, dont le script `scripts/suivi.mjs` lit les titres français `### Étape N — …` de `suivi/AVANCEMENT.md`. En attente de la décision de Clément ; d'ici là, le français reste la règle.
