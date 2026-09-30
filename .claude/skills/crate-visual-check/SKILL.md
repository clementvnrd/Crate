---
name: crate-visual-check
description: Vérification visuelle de l'interface Crate avec Playwright CLI — captures en thème clair et sombre, plusieurs accents, tailles de fenêtre 1000×600 / 1400×900 / 1920×1080, français et anglais, mouvement réduit ; audit mesuré dans la page (contraste WCAG, boutons sans nom, chevauchements, modales hors fenêtre, colonnes écrasées, texte sous 12 px) ; session d'annotation avec le propriétaire. À utiliser avant de déclarer terminé un travail d'interface, pour un audit, ou pour comparer avec une maquette.
argument-hint: "[vue] [clair|sombre|tout]"
allowed-tools: Bash(playwright-cli:*) Bash(npx playwright:*) Bash(yarn dev:vite)
---

# Vérifier l'interface avec Playwright CLI

Outil : [microsoft/playwright-cli](https://github.com/microsoft/playwright-cli) (Apache 2.0). Il pilote un vrai navigateur par des commandes courtes et écrit instantanés et captures dans des fichiers, sans remplir le contexte. Référence complète : `playwright-cli --help`.

## Prérequis

1. **Playwright CLI installé.** Vérifier : `playwright-cli --version`. S'il manque, **demander au propriétaire** avant d'installer quoi que ce soit (installation globale) :
   ```bash
   npm install -g @playwright/cli@latest
   ```
2. **Un frontend qui tourne dans un navigateur.** Crate appelle le backend Tauri (`invoke`) dès le démarrage : ouvert tel quel dans un navigateur (`yarn dev:vite`, port 1420), il tombe sur l'écran de crash. Il faut un **harnais** qui remplace le backend par des données fictives (faux IPC via `@tauri-apps/api/mocks`, une quinzaine de pistes, réglages de thème et de langue). Celui de l'audit du 25 septembre n'a pas été versionné.
   - Si le harnais existe (chercher un script `harness` dans `package.json` ou un dossier `harness/`), le lancer et utiliser son URL.
   - Sinon, **le dire clairement** dans le compte rendu (« vérification visuelle impossible : pas de harnais ») et se rabattre sur : `yarn design:scan`, relecture du code, et une demande de capture au propriétaire (`yarn dev`, puis capture de la vue concernée en clair et en sombre). Ne jamais prétendre avoir vérifié visuellement.

Les captures et instantanés vont dans `.playwright-cli/` (ignoré par git).

## Réglages pilotés par localStorage

L'app lit ces clés au chargement (`app.html`) ; on les pose puis on recharge :

| Clé | Valeurs |
| --- | --- |
| `crate-theme` | `dark`, `light`, `system` |
| `crate-accent` | `blue`, `indigo`, `violet`, `purple`, `pink`, `rose`, `orange`, `amber`, `emerald`, `teal` |
| `crate-font` | `open-sans`, `jost`, `dm-sans`, `inter`, `nunito`, `fira-code`, `ibm-plex-mono`, `source-code-pro` |
| `crate-language` | `en`, `fr` (et les autres locales) |

## Matrice minimale avant « terminé »

| # | Thème | Accent | Fenêtre | Langue |
| --- | --- | --- | --- | --- |
| 1 | sombre | blue | 1400×900 | fr |
| 2 | clair | blue | 1400×900 | fr |
| 3 | sombre | orange | 1000×600 | en |
| 4 | clair | amber | 1000×600 | en |
| 5 | sombre | blue | 1920×1080 | fr |

Ajouter `set-reduced-motion reduce` sur une des lignes quand des animations sont en jeu, et une police monospace (`fira-code`) quand des largeurs de colonnes sont en jeu.

## Déroulé

```bash
# 1. ouvrir le harnais (URL à adapter)
playwright-cli open http://localhost:1420/
playwright-cli resize 1400 900

# 2. poser une combinaison et recharger
playwright-cli localstorage-set crate-theme light
playwright-cli localstorage-set crate-accent amber
playwright-cli localstorage-set crate-language fr
playwright-cli reload

# 3. aller sur la vue (refs lues dans l'instantané)
playwright-cli snapshot
playwright-cli click e12

# 4. capturer et mesurer
playwright-cli screenshot --filename=.playwright-cli/pulse-clair-amber-1400.png
playwright-cli run-code --filename=.claude/skills/crate-visual-check/scripts/ui-audit.js

# 5. recommencer pour chaque ligne de la matrice, puis fermer
playwright-cli close
```

Nommer les captures `<vue>-<thème>-<accent>-<largeur>[-<état>].png` pour pouvoir comparer avant/après.

### Ce que mesure `ui-audit.js`

Le script s'exécute dans la page et renvoie un rapport JSON :

| Champ | Signification | Défaut du registre |
| --- | --- | --- |
| `lowContrast` | Texte sous 4,5:1 (3:1 pour le grand texte), fond réel calculé à travers les couches transparentes | D3 |
| `unmeasuredContrast` | Textes posés sur une image ou un dégradé : à regarder sur la capture | D3 |
| `smallText` | Texte sous 12 px | règle 4 |
| `unnamedControls` | Bouton, lien ou champ sans nom accessible | D10 |
| `pointerOnly` | Élément au curseur « main » qui n'est ni bouton ni lien (`div` cliquable) | D10 |
| `overlaps` | Contrôles qui se chevauchent (barre d'outils à 1000 px) | D4 |
| `outOfWindow` | Modale, menu ou tooltip qui sort de la fenêtre | D6 |
| `crushedColumns` | Texte tronqué dans moins de 48 px (colonne écrasée) | D7 |
| `pageOverflowX` | Défilement horizontal de la page | règle 10 |

Une fenêtre de 0×0 (navigateur masqué ou sans taille) renvoie une erreur : donner une taille avec `resize` et relancer.

### Accessibilité au clavier

```bash
playwright-cli press Tab        # répéter et vérifier dans l'instantané que le focus avance de façon logique
playwright-cli snapshot         # l'arbre d'accessibilité montre les noms des boutons
playwright-cli find --regex "button \\[ref="   # boutons sans nom (un bouton nommé s'écrit button "Nom" [ref=…])
playwright-cli press Escape     # une modale doit se fermer et rendre le focus
```

## Retour du propriétaire

Pour une revue de design, ouvrir la vue et lancer le tableau d'annotation : le propriétaire encadre des zones et écrit ses remarques, on reçoit la capture annotée, l'instantané de la zone et les notes.

```bash
playwright-cli show --annotate
```

## Repli sans Playwright CLI

Si Playwright CLI n'est pas disponible mais que le harnais tourne, le navigateur intégré de l'app (outils `mcp__Claude_Browser__*`) permet la même vérification : `resize_window` pour la taille, `javascript_tool` pour poser les clés `localStorage` et exécuter le corps de `ui-audit.js`, `computer` pour les captures. Le préciser dans le compte rendu.

## Compte rendu

Pour chaque ligne de la matrice : captures produites (chemins), rapport `ui-audit.js` résumé (compteurs + constats), écarts visibles sur la capture. Terminer par ce qui n'a pas pu être vérifié.
