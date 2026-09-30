---
version: 1
name: Crate
description: "Gestionnaire de bibliothèque DJ pour desktop (Tauri + SvelteKit + Tailwind 4). Un outil de travail dense et calme : fond zinc presque noir (thème sombre par défaut) ou blanc (thème clair), une seule couleur d'accent choisie par l'utilisateur, des tableaux de pistes serrés où les chiffres sont en `tabular-nums`, et des badges de données (tonalité Camelot, énergie) qui sont la seule vraie source de couleur. L'interface s'efface derrière la musique."

# Les valeurs ci-dessous sont celles de apps/desktop/src/style.css.
# Dans le code, on n'écrit JAMAIS ces hex : on utilise la classe Tailwind du token (bg-surface-1, text-text-secondary…).
colors:
  dark:
    surface-0: "#09090b"   # fond de fenêtre (zinc-950)
    surface-1: "#18181b"   # panneaux, modales, menus (zinc-900)
    surface-2: "#27272a"   # contrôles, survol, éléments surélevés (zinc-800)
    surface-3: "#3f3f46"   # survol/appui d'un élément surélevé, squelettes (zinc-700)
    surface-4: "#52525b"   # rare : piste de progression, poignée (zinc-600)
    text-primary: "#fafafa"
    text-secondary: "#a1a1aa"
    text-tertiary: "#71717a"
    text-disabled: "#52525b"
    stroke: "#3f3f46"
    stroke-subtle: "#27272a"
    stroke-strong: "#52525b"
  light:
    surface-0: "#ffffff"
    surface-1: "#fafafa"
    surface-2: "#f4f4f5"
    surface-3: "#e4e4e7"
    surface-4: "#d4d4d8"
    text-primary: "#18181b"
    text-secondary: "#52525b"
    text-tertiary: "#71717a"
    text-disabled: "#a1a1aa"
    stroke: "#d4d4d8"
    stroke-subtle: "#e4e4e7"
    stroke-strong: "#a1a1aa"
  accent:                  # [data-accent], choisi dans Réglages > Affichage ; bleu par défaut
    brand-primary: "var(--brand-primary)"   # blue #3b82f6, indigo, violet, purple, pink, rose, orange, amber, emerald, teal
    brand-hover: "var(--brand-hover)"
    brand-muted: "var(--brand-muted)"       # accent à 20 % : fond d'élément actif
  state:                   # identiques dans les deux thèmes (voir « Lacunes connues »)
    danger: "#ef4444"
    warning: "#f59e0b"
    success: "#22c55e"
    info: "#3b82f6"
  data:                    # palettes de DONNÉES, centralisées, jamais recopiées dans un composant
    camelot: "shared/utils/camelot.ts (CAMELOT_COLORS, getCamelotColor) — roue Mixed In Key 11"
    energy: "shared/utils/energy.ts (getEnergyInfo) — niveaux 1 à 10"

typography:
  family: "var(--font-family) — [data-font] choisi par l'utilisateur ; Open Sans par défaut (app.html), aussi Jost, DM Sans, Inter, Nunito, Fira Code, IBM Plex Mono, Source Code Pro"
  mono: "font-mono de Tailwind (pile système) pour tonalités, BPM, durées, codes"
  # Échelle portée par le composant <Text variant=…> (common/Text.svelte)
  header-1: { size: 18px, class: text-lg, weight: 600, color: text-primary, element: h2 }
  header-2: { size: 14px, class: text-sm, weight: 600, color: text-primary, element: h3 }
  header-3: { size: 14px, class: text-sm, weight: 600, color: text-secondary, uppercase: true, tracking: wide }
  header-4: { size: 12px, class: text-xs, weight: 500, color: text-tertiary, uppercase: true, tracking: wide }
  header-table: { size: 12px, class: text-xs, weight: 500, color: text-tertiary, uppercase: true, tracking: wider }
  body-1: { size: 14px, class: text-sm, weight: 400, color: text-primary }
  body-2: { size: 14px, class: text-sm, weight: 500, color: text-primary }
  caption: { size: 12px, class: text-xs, weight: 400, color: text-tertiary }
  code: { size: 12px, class: "text-xs font-mono", weight: 400 }
  numbers: "tabular-nums sur toute colonne ou comparaison de nombres (BPM, durée, compteurs, stats)"

rounded:
  control: "rounded-md (6px) — boutons, champs, select, items de menu"
  small: "rounded (4px) — badges de données, checkbox, tooltip, kbd"
  panel: "rounded-lg (8px) — cartes, modales, menus déroulants, panneaux"
  pill: "rounded-full — pastilles, interrupteurs, curseurs, avatars"
  forbidden: "rounded-xl, rounded-2xl, rounded-3xl"

elevation:
  flat: "aucune ombre : lignes de tableau, panneaux posés sur surface-0"
  shadow-sm: "élément qui dépasse légèrement (segment actif, badge)"
  shadow-lg: "menus, tooltips, select ouvert, toasts"
  shadow-xl: "modales"
  forbidden: "glow, drop-shadow coloré, backdrop-blur, dégradés décoratifs, halos flous"

spacing:
  base: "échelle Tailwind (4px)"
  row: "ligne de piste : px-3 py-1.5, gap-2, border-b border-stroke-subtle"
  control-gap: "gap-2 entre contrôles, gap-4 entre groupes"
  panel-padding: "p-4 (menus p-1/p-2, modales p-5/p-6)"

motion:
  duration: "150 à 200 ms"
  properties: "couleur, opacité, transform — lister les propriétés (transition-colors, transition-[…]), jamais transition-all"
  svelte: "fade/scale de svelte/transition pour apparitions ; durée ≤ 200 ms"
  infinite: "réservé aux spinners et indicateurs de chargement, toujours avec motion-reduce:animate-none"

window:
  min: "1000×600 (tauri.conf.json)"
  default: "1400×900"
  check: "1000×600, 1400×900, 1920×1080"

components:
  Button: { variants: [primary, secondary, ghost, danger, ghost-danger, outline], sizes: [sm, md, lg], rounded: control }
  IconButton: { sizes: [sm 24px, md 32px, lg 40px], active: "bg-brand-muted text-brand-primary", rule: "nom accessible obligatoire" }
  Text: { variants: [header-1, header-2, header-3, header-4, header-table, body-1, body-2, caption, code] }
  Input: { background: surface-2, border: stroke, rounded: control }
  Select: { trigger: surface-2, menu: "surface-1 shadow-lg rounded-lg z-50" }
  Modal: { element: "<dialog> natif", background: surface-1, border: stroke, rounded: panel, shadow: shadow-xl, sizes: [sm, md, lg, xl, 2xl, 3xl, 4xl], rule: "hauteur bornée à la fenêtre, pied fixe" }
  Tooltip: { background: surface-1, border: stroke, rounded: small, text: text-xs, portal: true }
  ContextMenu: { background: surface-1, rounded: control, shadow: shadow-lg }
  Checkbox: {}
  ToggleSwitch: {}
  Slider: {}
  Spinner: {}
  Toast: {}
  Icon: { source: "common/Icon.svelte (jeu interne) ; un test Vitest échoue sur un nom inconnu" }
  track-row: { layout: "grid, items-center, gap-2, px-3 py-1.5, text-sm", selected: "bg-brand-muted", playing: "titre en text-brand-primary" }
  key-badge: { size: "h-[22px] w-11", font: "font-mono text-xs font-bold", colors: "getCamelotColor(key)", rounded: small }
  energy-badge: { component: "library/EnergyBadge.svelte", colors: "getEnergyInfo(level)", rounded: small }
---

# Crate — système de design

Ce document décrit **comment Crate doit avoir l'air**. Il est lu par les assistants de code (agent `design` dans `.claude/agents/design.md`) avant tout travail sur l'interface, et sert de référence au propriétaire. Il est extrait du code réel (`apps/desktop/src/style.css`, `lib/components/common/`) et des « règles de design strictes » du [registre des défauts](suivi/REGISTRE-DEFAUTS.md#design-responsive-et-accessibilité).

## Vue d'ensemble

Crate est un **outil de DJ**, pas une page marketing. On y passe des heures à trier, écouter, taguer et préparer des sets : l'interface doit être rapide à lire, stable, dense sans être étouffante, et parfaitement cohérente d'une vue à l'autre.

- **Densité élevée, variance faible, mouvement minimal.** En termes de « cadrans » (repris de taste-skill) : `VARIANCE 2 · MOTION 2 · DENSITY 8`. Symétrie et alignements prévisibles, animations uniquement pour signaler un changement d'état, beaucoup d'information par écran.
- **La couleur est rare et signifiante.** Les surfaces sont neutres (zinc). La couleur vient de trois sources seulement : l'accent choisi par l'utilisateur (sélection, focus, action principale, lecture en cours), les couleurs d'état (erreur, avertissement, succès), et les palettes de données (tonalité Camelot, énergie).
- **Un seul langage visuel.** L'amont avait un système cohérent ; les builds 40 à 57 ont ajouté trois langages parallèles (cyan/ambre du Player et de Pulse, vert néon de Beatport, verre dépoli). La direction est de **revenir au langage de l'amont** et d'y ramener les nouvelles vues, pas d'inventer un quatrième style.
- **Deux thèmes à parité.** Sombre par défaut, clair complet. Toute vue doit être juste dans les deux, avec les dix accents.

## Couleurs

### Surfaces (échelle d'élévation)

Du plus profond au plus haut : `surface-0` (fond de fenêtre) → `surface-1` (panneaux, modales, menus) → `surface-2` (contrôles, survol) → `surface-3` (survol d'un élément déjà surélevé, squelettes de chargement) → `surface-4` (rare). On ne saute pas de niveau et on ne crée pas de surface intermédiaire en opacité (`bg-surface-1/70` est interdit : c'est un reste de verre dépoli).

### Texte

`text-primary` pour le contenu, `text-secondary` pour les métadonnées (artiste, album), `text-tertiary` pour les libellés et valeurs vides (« - »), `text-disabled` pour l'inactif. Jamais de gris de palette (`text-zinc-400`) ni de hex.

### Traits

`stroke-subtle` entre lignes de tableau, `stroke` pour les bordures de contrôles et panneaux, `stroke-strong` pour un contour qui doit se voir (focus secondaire, séparateur marqué).

### Accent

L'accent est **choisi par l'utilisateur** (`[data-accent]`). Conséquence : une vue ne choisit jamais « sa » couleur. `brand-primary` sert à l'action principale, au focus, à l'élément sélectionné ou en lecture ; `brand-muted` (accent à 20 %) sert de fond à l'élément actif. Pour d'autres opacités, `bg-brand-primary/10` fonctionne (thème `inline`), ainsi que les utilitaires `bg-brand-primary-5` et `bg-brand-primary-10`. Toute vue doit être vérifiée avec au moins deux accents (bleu par défaut et un accent chaud comme orange ou amber).

### États

`danger`, `warning`, `success`, `info` : erreurs, avertissements, confirmations. Ils ne servent pas de décoration (pas de carte KPI « émeraude » ou « violette »).

### Palettes de données

Les couleurs de tonalité Camelot (roue Mixed In Key 11) et d'énergie encodent une information que le DJ lit d'un coup d'œil : elles sont légitimes. Règles :

1. Elles vivent **uniquement** dans `shared/utils/camelot.ts` et `shared/utils/energy.ts` ; un composant les obtient par `getCamelotColor()` / `getEnergyInfo()` et les applique en `style=`.
2. Elles colorent le **badge**, pas son environnement (pas de ligne entière teintée, pas de glow).
3. Le texte du badge garde un contraste d'au moins 4,5:1 sur son fond (déjà prévu par les paires `bg`/`text`), dans les deux thèmes.

## Typographie

- **Toujours via `<Text variant=…>`** quand c'est du texte autonome. Pour du texte dans un contrôle ou une cellule, les classes Tailwind de l'échelle (`text-xs`, `text-sm`, `text-base`, `text-lg`) suffisent.
- **Pas de taille arbitraire** (`text-[10px]`, `text-[11px]`) : 12 px (`text-xs`) est le minimum pour une donnée. Les tailles arbitraires actuelles (environ 190) sont de la dette.
- **Graisses** : 400 et 500 pour le texte, 600 pour les titres. `font-black` et `font-extrabold` sont hors système.
- **Chiffres** : `tabular-nums` sur les colonnes et comparaisons (BPM, durées, compteurs, statistiques) ; `font-mono` pour les tonalités, BPM et codes.
- **Police** : choisie par l'utilisateur (`[data-font]`), Open Sans par défaut. Un composant ne force jamais une famille.
- **Majuscules** : réservées aux en-têtes de tableau et libellés de section (`header-3`, `header-4`, `header-table`), avec parcimonie. Pas d'étiquette en capitales au-dessus de chaque bloc.

## Mise en page

- **Structure de l'app** : barre d'outils en haut, barre latérale gauche redimensionnable, zone principale, panneau droit optionnel. On ne réinvente pas cette structure dans une vue.
- **Fenêtre minimale 1000×600.** Aucune zone de contenu à hauteur fixe en pixels ; les listes sont en `flex-1 min-h-0` avec défilement interne ; les enfants flex qui contiennent du texte ont `min-w-0` pour que `truncate` fonctionne.
- **Tableaux** : CSS Grid avec `minmax(0, …)` et une largeur minimale sur la colonne titre ; les colonnes secondaires se masquent avant que le titre ne rétrécisse.
- **Grilles plutôt que calculs** : `grid` et `gap-*`, jamais `w-[calc(33%-1rem)]`.
- **Cartes seulement si l'élévation a un sens.** Sinon on groupe par espacement ou par `border-t` / `divide-y`. Pas de carte dans une carte dans une carte.

## Élévation et profondeur

Plate par défaut. `shadow-sm` pour ce qui dépasse à peine, `shadow-lg` pour ce qui flotte (menus, tooltips, toasts), `shadow-xl` pour les modales. Pas de `backdrop-blur`, de glow, de dégradé décoratif ni de halo flou en arrière-plan. La profondeur vient de l'échelle de surfaces.

## Formes

| Élément | Rayon |
| --- | --- |
| Boutons, champs, select, items de menu | `rounded-md` |
| Badges de données, checkbox, tooltip, `kbd` | `rounded` |
| Cartes, modales, menus déroulants, panneaux | `rounded-lg` |
| Pastilles, interrupteurs, curseurs, avatars | `rounded-full` |

`rounded-xl`, `rounded-2xl` et `rounded-3xl` sont hors système (dette des builds 40 à 57).

## Composants

Les composants de `lib/components/common/` sont **obligatoires** : `Button`, `IconButton`, `Text`, `Input`, `Select`, `Checkbox`, `ToggleSwitch`, `Slider`, `Modal`, `ConfirmModal`, `InputModal`, `ContextMenu`, `Tooltip`, `Spinner`, `Toast`, `Icon`. Un besoin non couvert se traite en **étendant** le composant commun (nouvelle variante, nouvelle prop), pas en le recopiant.

Composants à extraire (défaut D11) et à utiliser dès qu'ils existent : `SegmentedControl` (4 copies aujourd'hui), `KeyBadge` (badge Camelot copié 8 fois), `EnergyBadge` (existe dans `library/`, à généraliser).

- **Bouton** : `primary` pour l'action principale d'une vue ou d'une modale (une seule), `secondary` par défaut, `ghost` dans les barres d'outils, `danger` pour une action destructive confirmée.
- **Bouton icône** : toujours `IconButton` avec un nom accessible (`title` et `aria-label` traduits). État actif : `bg-brand-muted text-brand-primary`.
- **Modale** : `Modal` commun (`<dialog>`, Échap, piège de focus). Hauteur bornée à la fenêtre, défilement interne, pied fixe. Une action destructive passe par `ConfirmModal`.
- **Ligne de piste** : grille dense, `border-b border-stroke-subtle`, sélection en `bg-brand-muted`, piste en lecture signalée par le titre en `text-brand-primary`.
- **Badge de tonalité** : `font-mono text-xs font-bold`, `rounded`, couleurs de `getCamelotColor()`.

## États d'interface

Chaque vue et chaque composant qui charge des données gère **quatre états** : chargement (squelette `bg-surface-3` qui a la forme du contenu final ; spinner seulement pour une action courte), vide (message utile + action pour remplir), erreur (message qui dit quoi faire, visible, jamais un échec silencieux), et nominal. Les interactions ont survol, appui, focus visible (`:focus-visible`, contour de l'accent défini globalement) et désactivé.

## Accessibilité

- Tout élément cliquable est un `<button>` (ou `<a>` pour une navigation) avec un nom accessible. Pas de `div` cliquable ni de `svelte-ignore a11y`.
- Focus visible partout ; jamais `outline-none` sans remplacement.
- Contraste : 4,5:1 pour le texte, 3:1 pour les éléments d'interface, **dans les deux thèmes**.
- Tout ce qui se fait à la souris se fait au clavier (waveform, heatmap, glisser-déposer : alternative clavier ou menu).
- `prefers-reduced-motion` respecté : toute animation infinie porte `motion-reduce:animate-none`.

## Texte et langue

- **Zéro chaîne en dur** : chaque texte visible passe par `{$translate('section.cle')}` (store `translate` de `$shared/i18n`), avec la clé ajoutée dans `shared/i18n/locales/en.json` **et** `fr.json` dans la même modification.
- Le français **vouvoie** l'utilisateur, comme tout `fr.json`.
- Français : points de suspension `…` (pas `...`), guillemets « », espace insécable avant `: ; ! ?` et dans les nombres (« 2 310 »), nombres et dates via `Intl.NumberFormat` / `Intl.DateTimeFormat`.
- Ton : direct, concret, voix active. Libellés de bouton précis (« Supprimer les doublons » plutôt que « Continuer »). Messages d'erreur qui disent comment s'en sortir. Pas d'exclamation, pas de « Oups ».

## À faire et à éviter

### À faire

- Partir d'un composant commun et des tokens ; vérifier le rendu en clair et en sombre, avec deux accents, à 1000×600.
- Laisser l'accent de l'utilisateur porter l'action principale, la sélection et le focus.
- Utiliser `tabular-nums` et `font-mono` pour les données musicales.
- Préférer l'espacement et les séparateurs fins aux cartes.

### À éviter

- `dark:` (le thème est `[data-theme]`, pas l'OS), classes de palette (`bg-zinc-800`, `text-emerald-400`), hex dans un composant.
- `transition-all`, animations infinies décoratives, `hover:-translate-y`, `group-hover:scale-110` sur des cartes.
- Verre dépoli, `backdrop-blur`, halos, glow, dégradés, `shadow-2xl`.
- Une couleur « propre » à une vue (cyan pour le Player, vert néon pour Beatport).
- Emojis dans l'interface, icônes dessinées à la main hors `Icon.svelte`.
- Hauteurs fixes en pixels sur les zones de contenu, `h-screen`.
- `z-[9999]` : couches `z-10` (contenu collant), `z-20`/`z-30` (panneaux), `z-40` (overlays), `z-50` (menus, tooltips).

## Comportement responsive

Crate est une app desktop : pas de points de rupture mobiles dans l'app desktop (l'app mobile est un projet séparé, `apps/mobile`). Les tailles à vérifier sont celles de la fenêtre :

| Taille | Ce qui doit tenir |
| --- | --- |
| 1000×600 (minimum) | Barre d'outils sans chevauchement, colonne titre lisible, modales entières, transport du Player visible |
| 1400×900 (défaut) | Rendu de référence |
| 1920×1080 | Pas de grands vides, contenus larges bornés |

## Guide d'itération

1. Lire ce fichier, puis le composant commun le plus proche du besoin.
2. Travailler un composant à la fois, le nommer par son nom de composant ou de token.
3. Toute nouvelle couleur sémantique devient un token déclaré pour les deux thèmes dans `style.css` (et `app.html` si elle sert à l'écran de démarrage), contraste vérifié ; trois au maximum.
4. Lancer `yarn design:scan <chemin>` sur les fichiers touchés : aucune nouvelle occurrence ne doit apparaître.
5. Vérifier visuellement (skill `crate-visual-check`) avant de déclarer terminé.
6. Mettre à jour ce document si une règle change.

## Lacunes connues

- **Couleurs d'état non adaptées au thème clair** : `warning` (#f59e0b) sur blanc fait environ 2,1:1, `success` environ 2,3:1. Pour du texte, il faudra des variantes `-text` par thème (lié à D3).
- **Dette mesurée** par `yarn design:scan` (voir le registre, défauts D3, D7, D10, D11) : environ 580 classes de palette et 300 hex dans les nouvelles vues, 190 tailles arbitraires, 88 rayons `xl` à `3xl`, 26 `backdrop-blur`, 89 `transition-all`, 43 `dark:` résiduels.
- **Jeu d'icônes interne** : `Icon.svelte` contient les tracés à la main hérités de l'amont ; c'est la seule source autorisée, on y ajoute les icônes manquantes plutôt que d'introduire une bibliothèque.
- **Pas de harnais visuel versionné** : le harnais navigateur de l'audit (faux backend Tauri) n'est pas dans le dépôt ; la vérification visuelle automatique en dépend.
