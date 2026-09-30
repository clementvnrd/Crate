# Web Interface Guidelines — adaptées à Crate

Source : [vercel-labs/web-interface-guidelines](https://github.com/vercel-labs/web-interface-guidelines) (`command.md`), instantané du 30 septembre 2026, utilisé par le skill `web-design-guidelines` de [vercel-labs/agent-skills](https://github.com/vercel-labs/agent-skills). Licence MIT, © 2025 Vercel Labs (texte ci-dessous).

Adaptation : syntaxe Svelte 5 (`onclick`, `onkeydown`, `bind:value`), contexte Tauri desktop (pas de SSR, pas de tactile, pas d'URL partageable), règles propres à Crate. Chaque règle porte une étiquette : **[A]** applicable telle quelle, **[C]** adaptée à Crate, **[—]** sans objet (gardée pour mémoire). Pour une version à jour, récupérer l'original : `https://raw.githubusercontent.com/vercel-labs/web-interface-guidelines/main/command.md` et signaler les nouveautés au propriétaire.

## Accessibilité

- [A] Boutons icône : `aria-label` (traduit). Dans Crate : `IconButton` avec `title` **et** nom accessible.
- [A] Contrôles de formulaire : `<label>` ou `aria-label`.
- [C] Éléments interactifs : gestion clavier (`onkeydown`) ; waveform, heatmap, roue harmonique et glisser-déposer ont une alternative clavier (flèches, Entrée, menu contextuel).
- [A] `<button>` pour les actions, `<a>` pour la navigation ; jamais `<div onclick>` (ni `svelte-ignore a11y_click_events_have_key_events` pour le masquer).
- [A] Images : `alt` (ou `alt=""` si décoratives) ; pochettes : `alt` = titre de la piste, traduit.
- [A] Icônes décoratives : `aria-hidden="true"`.
- [A] Mises à jour asynchrones (toasts, validation, progression d'analyse ou d'export) : `aria-live="polite"`.
- [A] HTML sémantique avant ARIA (`<button>`, `<label>`, `<table>`, `<nav>`, `<main>`).
- [C] Titres hiérarchiques (`Text variant="header-1"` rend un `h2`) ; pas de lien d'évitement nécessaire dans une app à panneaux, mais le focus doit pouvoir atteindre la zone principale au clavier.
- [—] `scroll-margin-top` sur ancres de titres.
- [A] Contrôles média (transport du Player) utilisables au clavier, avec noms accessibles.

## Focus

- [A] Focus visible sur tout élément interactif : `:focus-visible` (le contour d'accent global de `style.css`) ou `focus-visible:ring-*`.
- [A] Jamais `outline-none` sans remplacement visible. Attention : `style.css` met `*:focus { outline: none }` et rétablit `*:focus-visible` ; un composant qui ajoute `focus:outline-none` + `focus:ring-1` affiche un anneau au clic (préférer `focus-visible:`).
- [A] `:focus-visible` plutôt que `:focus`.
- [A] `:focus-within` pour les contrôles composés (barre de recherche avec bouton d'effacement).
- [A] En-têtes, barres et overlays collants ne recouvrent pas l'élément qui a le focus.
- [C] Modales et panneaux : piège de focus et retour du focus à l'élément d'origine (le `Modal` commun le fait ; les overlays maison non, défaut D10).

## Formulaires

- [C] `name` explicite ; `autocomplete="off"` sur les champs non liés à un compte (recherche, renommage) pour éviter le gestionnaire de mots de passe.
- [A] `type` correct (`number`, `url`, `search`) et `inputmode`.
- [A] Ne jamais bloquer le collage.
- [A] Libellés cliquables (`for` ou libellé englobant).
- [A] `spellcheck="false"` sur codes, chemins, URL, identifiants.
- [A] Checkbox et radio : libellé et contrôle partagent une seule zone de clic.
- [A] Bouton de validation actif jusqu'au début de la requête, puis indicateur pendant la requête.
- [A] Erreurs en ligne près du champ ; focus sur la première erreur.
- [C] Placeholders : exemple de valeur terminé par `…` (« Rechercher un titre, un artiste… »), jamais utilisés comme libellé.
- [A] Avertir avant de quitter avec des modifications non enregistrées (éditeur de piste, réglages).

## Animation

- [A] Respecter `prefers-reduced-motion` : `motion-reduce:animate-none`, `motion-reduce:transition-none`.
- [A] N'animer que `transform` et `opacity` (et couleur pour les survols).
- [A] Jamais `transition: all` / `transition-all` : lister les propriétés.
- [A] `transform-origin` correct.
- [A] SVG : transformations sur un `<g>` avec `transform-box: fill-box; transform-origin: center`.
- [A] Animations interruptibles.
- [C] Aucune animation décorative en boucle ; seuls les spinners et indicateurs de chargement bouclent.

## Typographie

- [A] `…` et non `...` (y compris dans `en.json` et `fr.json`).
- [C] Guillemets typographiques : « » en français, “ ” en anglais ; jamais `"` droits dans un texte visible.
- [A] Espaces insécables : `10&nbsp;Mo`, `⌘&nbsp;K` ; en français, espace insécable avant `: ; ! ?` et comme séparateur de milliers.
- [A] États de chargement terminés par `…` : « Chargement… », « Analyse… ».
- [A] `tabular-nums` pour les colonnes et comparaisons de nombres.
- [A] `text-wrap: balance` / `text-pretty` sur les titres qui passent à la ligne.

## Contenus

- [A] Conteneurs de texte robustes aux contenus longs : `truncate`, `line-clamp-*`, `break-words` (titres de pistes à rallonge, noms de labels, remixeurs multiples).
- [A] Enfants flex avec `min-w-0` pour permettre la troncature.
- [A] États vides gérés : pas d'interface cassée pour une chaîne ou une liste vide (bibliothèque vide, stats sans écoute, playlist vide).
- [A] Données utilisateur : prévoir court, moyen et très long.

## Images

- [A] `<img>` avec `width` et `height` explicites (ou conteneur de taille fixe) pour éviter les sauts de mise en page.
- [A] Hors écran : `loading="lazy"` (pochettes dans les longues listes).
- [—] `fetchpriority="high"` sur l'image critique (pas de LCP dans une app locale).

## Performance

- [A] Listes de plus de 50 éléments : virtualisées (`@tanstack/virtual-core`, déjà utilisé par `TrackList`) ou `content-visibility: auto`.
- [A] Pas de lecture de mise en page pendant le rendu (`getBoundingClientRect`, `offsetHeight`…) ; regrouper lectures et écritures DOM.
- [A] Champs contrôlés bon marché à chaque frappe (recherche avec anti-rebond).
- [C] Polices embarquées dans `static/fonts` avec `font-display: swap` ; l'import Google Fonts de `style.css` est un reste à supprimer à terme (app hors ligne).
- [—] `preconnect` CDN, vidéo plutôt que GIF.

## Navigation et état

- [C] Pas d'URL partageable dans une app Tauri : l'état important (vue, filtres, tri, colonnes, largeur des panneaux) est **persisté** dans les réglages ou le store, et restauré au lancement.
- [A] Actions destructives : `ConfirmModal` ou fenêtre d'annulation, jamais immédiates (supprimer, formater un appareil, remplacer un fichier).

## Interaction

- [—] `touch-action`, `-webkit-tap-highlight-color` (desktop).
- [A] `overscroll-behavior: contain` dans les modales, panneaux et menus qui défilent.
- [A] Pendant un glisser : pas de sélection de texte (déjà `user-select: none` global), élément glissé `inert`.
- [A] Gestes (glisser, molette sur un curseur) : alternative clic et clavier.
- [C] `autofocus` uniquement sur le champ principal d'une modale (renommer, créer une playlist).

## Mise en page

- [—] `env(safe-area-inset-*)`.
- [A] Pas de barre de défilement parasite : corriger le débordement plutôt que masquer (`overflow-x-hidden` en dernier recours).
- [A] Flex et grid plutôt que mesures JavaScript.

## Thème

- [C] `color-scheme` est posé par `[data-theme]` dans `style.css` : ne pas le surcharger.
- [—] `<meta name="theme-color">`.
- [A] `<select>` natif : `background-color` et `color` explicites (le `Select` commun évite le problème).

## Langue

- [A] Dates et heures : `Intl.DateTimeFormat` (via la locale svelte-i18n), jamais de format codé en dur.
- [A] Nombres : `Intl.NumberFormat` (« 2 310 » en français, pas « 2,310 », défaut L3).
- [C] Langue : réglage de l'app (`crate-language`), sinon `navigator.languages`.
- [A] Noms de marques, codes et identifiants (Camelot `8A`, BPM, noms de fichiers) : `translate="no"`.

## Hydratation

- [—] Sans objet : `adapter-static` sans rendu serveur.

## Survol et états interactifs

- [A] Boutons et liens ont un état `hover:`.
- [A] Les états interactifs augmentent le contraste : survol, appui et focus plus marqués que le repos.

## Rédaction

- [A] Voix active : « Exporter la playlist » et non « La playlist sera exportée ».
- [C] Casse : Title Case pour les titres et boutons en anglais (convention de l'amont) ; casse de phrase en français (« Créer une playlist »).
- [A] Chiffres pour les comptes : « 8 pistes » et non « huit pistes ».
- [A] Libellés précis : « Supprimer 12 doublons » plutôt que « Continuer ».
- [A] Les messages d'erreur disent comment corriger, pas seulement ce qui ne va pas.
- [C] Deuxième personne (« vous » en français, comme tout `fr.json` ; « you » en anglais), pas de première personne.
- [A] `&` plutôt que « et » quand la place manque (anglais).

## Anti-motifs à signaler

- `user-scalable=no` ou `maximum-scale=1`.
- Collage bloqué.
- `transition-all`.
- `outline-none` sans remplacement `focus-visible`.
- Navigation par `onclick` sans `<a>`.
- `<div>` ou `<span>` cliquables.
- Images sans dimensions.
- Grandes listes sans virtualisation.
- Champs sans libellé.
- Boutons icône sans nom accessible.
- Formats de date ou de nombre codés en dur.
- `autofocus` injustifié.
- Action uniquement gestuelle sans alternative clic et clavier.

## Licence de la source

```
MIT License

Copyright (c) 2025 Vercel Labs

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
