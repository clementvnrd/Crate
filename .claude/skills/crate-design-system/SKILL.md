---
name: crate-design-system
description: Règles de design de Crate (tokens, composants communs, thème [data-theme], accent utilisateur, i18n, accessibilité, densité d'outil DJ). À charger avant toute modification d'un composant Svelte, d'une vue ou de style.css, et avant tout avis sur l'apparence de l'app.
when_to_use: Toute tâche qui crée, modifie, relit ou juge l'interface desktop de Crate (apps/desktop/src).
paths:
  - apps/desktop/src/**
  - DESIGN.md
user-invocable: false
---

# Système de design de Crate

La référence complète est [DESIGN.md](../../../DESIGN.md) à la racine du dépôt. **Lis-le avant d'écrire une ligne d'interface.** Ce skill en est le condensé opérationnel.

## Lecture du contexte

Crate est un outil de travail de DJ (bibliothèque, lecteur, statistiques d'écoute, export USB), pas une page marketing. Cadrans de référence (vocabulaire taste-skill) :

| Cadran | Valeur | Conséquence |
| --- | --- | --- |
| `DESIGN_VARIANCE` | 2 | Symétrie, alignements prévisibles, structure de l'app inchangée |
| `MOTION_INTENSITY` | 2 | Survol, appui et transitions d'état de 150 à 200 ms ; rien d'autre ne bouge |
| `VISUAL_DENSITY` | 8 | Tableaux serrés, séparateurs de 1 px plutôt que des cartes, `tabular-nums` sur tous les chiffres |

Tous les réflexes « landing page » (hero, bento, halos, verre dépoli, typographie géante, cartes KPI lumineuses, marquee) sont **hors sujet** dans Crate.

## Les douze règles strictes

1. **Couleurs par tokens uniquement** : `bg-surface-0…4`, `text-text-primary|secondary|tertiary|disabled`, `border-stroke|stroke-subtle|stroke-strong`, `brand-primary|hover|muted`, `danger|warning|success|info`. Aucune classe de palette (`bg-zinc-800`, `text-emerald-400`), aucun hex dans un composant.
2. **Nouvelle couleur sémantique = nouveau token** déclaré pour les deux thèmes dans `style.css`, contraste vérifié ; trois au maximum (cue, live, marque).
3. **Jamais `dark:`** : le thème est `[data-theme]`, choisi dans l'app, pas l'OS.
4. **Typographie via `Text`** ou l'échelle Tailwind (`text-xs` à `text-lg`) ; jamais `text-[Npx]` ; 12 px minimum pour une donnée ; graisses 400/500/600.
5. **Rayons** : `rounded-md` contrôles, `rounded` badges, `rounded-lg` cartes/modales/menus, `rounded-full` pastilles. Jamais `rounded-xl` et au-delà.
6. **Élévation** : `shadow-sm`, `shadow-lg`, `shadow-xl` seulement. Pas de dégradé, glow, `backdrop-blur`, halo flou, surface semi-transparente.
7. **Composants communs obligatoires** (`lib/components/common/`) : `Button`, `IconButton`, `Text`, `Input`, `Select`, `Checkbox`, `ToggleSwitch`, `Slider`, `Modal`, `ConfirmModal`, `InputModal`, `ContextMenu`, `Tooltip`, `Spinner`, `Toast`, `Icon`. On étend un commun, on ne le recopie pas.
8. **Tout élément cliquable est un `<button>`** (ou `<a>`) avec un nom accessible traduit ; aucun `svelte-ignore a11y`.
9. **Zéro chaîne en dur** : `{$translate('section.cle')}` (`import { translate } from '$shared/i18n'`), clé ajoutée dans `shared/i18n/locales/en.json` et `fr.json` dans la même modification. Le français vouvoie (« vous »).
10. **Pas de hauteur fixe en pixels** sur une zone de contenu ; chaque vue et modale utilisable à 1000×600 (`flex-1 min-h-0`, `min-w-0`, `minmax(0, …)`).
11. **Mouvement** : 150 à 200 ms sur couleur, opacité ou transform, propriétés listées (jamais `transition-all`) ; animations infinies réservées aux spinners, avec `motion-reduce:animate-none`.
12. **Définition de terminé** : `yarn design:scan <fichiers>` sans nouvelle occurrence, vérification visuelle clair + sombre, deux accents, 1000×600 et 1400×900 (skill `crate-visual-check`), `yarn check:svelte` et `yarn test` verts.

## Table de conversion (dette → système)

| Trouvé dans le code | Remplacer par |
| --- | --- |
| `bg-black`, `bg-zinc-950` | `bg-surface-0` |
| `bg-zinc-900`, `bg-surface-1/70` + `backdrop-blur-*` | `bg-surface-1` (opaque) |
| `bg-zinc-800`, `bg-white/5` | `bg-surface-2` |
| `bg-zinc-700`, squelette `bg-white/10` | `bg-surface-3` |
| `text-white` sur fond neutre, `text-zinc-100` | `text-text-primary` (`text-white` reste permis sur un fond `brand-primary` ou `danger`) |
| `text-zinc-400`, `text-gray-400` | `text-text-secondary` |
| `text-zinc-500`, `text-zinc-600` | `text-text-tertiary` |
| `border-zinc-700`, `border-white/10` | `border-stroke` |
| `border-zinc-800`, `border-white/5` | `border-stroke-subtle` |
| `text-red-500`, `bg-red-600` | `text-danger`, `bg-danger` (ou `Button variant="danger"`) |
| `emerald`, `green` pour un succès | `success` |
| `sky`, `cyan`, `purple`, `emerald` décoratifs (Player, Pulse, Beatport, interrupteurs) | `brand-primary`, `brand-muted`, `bg-brand-primary/10` |
| `text-[10px]`, `text-[11px]` | `text-xs` |
| `rounded-xl`, `rounded-2xl` | `rounded-lg` |
| `transition-all` | `transition-colors`, `transition-opacity`, `transition-transform` ou `transition-[prop,prop]` |
| `shadow-2xl`, `shadow-emerald-500/10`, `drop-shadow-[…]` | `shadow-lg` / `shadow-xl`, ou rien |
| `font-black`, `font-extrabold` | `font-semibold` |
| `hover:-translate-y-0.5`, `group-hover:scale-110` sur des cartes | survol de couleur (`hover:bg-surface-2`) |
| `z-[9999]` | `z-50` (couches : 10 collant, 20/30 panneaux, 40 overlays, 50 menus/tooltips) |
| `div` avec `onclick` | `<button type="button">` ou `IconButton` |
| Couleur Camelot/énergie recopiée | `getCamelotColor()` / `getEnergyInfo()` de `shared/utils` |

## Exceptions légitimes

- **Palettes de données** (tonalité Camelot, énergie) : autorisées via `shared/utils/camelot.ts` et `energy.ts`, sur le badge uniquement.
- **Logos de marques tierces** (Mixed In Key, Beatport, Spotify) : leurs couleurs officielles restent dans le composant du logo, ligne annotée `design-scan-ignore` avec la raison.
- **`text-white`** sur un fond d'accent ou de danger.

## Direction

Le langage visuel de référence est **celui de l'amont** (sobre, zinc, piloté par l'accent). Les vues ajoutées aux builds 40 à 57 (Player, Pulse, Beatport, Upgrader, Duplicate Killer) sont à ramener vers ce langage, pas l'inverse. Ne jamais inventer une couleur « propre » à une vue.

## Pièges connus du dépôt

- `warning` et `success` n'ont pas assez de contraste en texte sur fond clair (lacune documentée dans DESIGN.md, défaut D3) : pour du texte d'état en thème clair, préférer une icône + `text-text-primary`, ou proposer un token `-text`.
- `app.html` recopie quelques couleurs pour l'écran de démarrage : toute modification de `surface-0`, `text-primary` ou des accents doit y être reportée.
- `Icon.svelte` est le seul jeu d'icônes ; une icône manquante s'y ajoute (le test `Icon.test.ts` échoue sur un nom inconnu).
- IPC Tauri et stores ne sont pas du ressort du design : ne jamais changer une commande, un nom de clé de réglage ou un store pour une raison visuelle.
