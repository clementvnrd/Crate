# Web Interface Guidelines — adapted to Crate

Source: [vercel-labs/web-interface-guidelines](https://github.com/vercel-labs/web-interface-guidelines) (`command.md`), snapshot of 30 September 2026, used by the `web-design-guidelines` skill of [vercel-labs/agent-skills](https://github.com/vercel-labs/agent-skills). MIT licence, © 2025 Vercel Labs (text below).

Adaptation: Svelte 5 syntax (`onclick`, `onkeydown`, `bind:value`), Tauri desktop context (no SSR, no touch, no shareable URL), Crate-specific rules. Each rule carries a tag: **[A]** applicable as is, **[C]** adapted to Crate, **[—]** not applicable (kept for reference). For an up-to-date version, fetch the original: `https://raw.githubusercontent.com/vercel-labs/web-interface-guidelines/main/command.md` and report what is new to the owner.

## Accessibility

- [A] Icon buttons: `aria-label` (translated). In Crate: `IconButton` with `title` **and** an accessible name.
- [A] Form controls: `<label>` or `aria-label`.
- [C] Interactive elements: keyboard handling (`onkeydown`); waveform, heatmap, harmonic wheel and drag-and-drop have a keyboard alternative (arrows, Enter, context menu).
- [A] `<button>` for actions, `<a>` for navigation; never `<div onclick>` (nor `svelte-ignore a11y_click_events_have_key_events` to hide it).
- [A] Images: `alt` (or `alt=""` if decorative); artwork: `alt` = track title, translated.
- [A] Decorative icons: `aria-hidden="true"`.
- [A] Asynchronous updates (toasts, validation, analysis or export progress): `aria-live="polite"`.
- [A] Semantic HTML before ARIA (`<button>`, `<label>`, `<table>`, `<nav>`, `<main>`).
- [C] Hierarchical headings (`Text variant="header-1"` renders an `h2`); no skip link needed in a panel-based app, but focus must be able to reach the main area by keyboard.
- [—] `scroll-margin-top` on heading anchors.
- [A] Media controls (the Player's transport) usable by keyboard, with accessible names.

## Focus

- [A] Visible focus on every interactive element: `:focus-visible` (the global accent outline in `style.css`) or `focus-visible:ring-*`.
- [A] Never `outline-none` without a visible replacement. Beware: `style.css` sets `*:focus { outline: none }` and restores `*:focus-visible`; a component that adds `focus:outline-none` + `focus:ring-1` shows a ring on click (prefer `focus-visible:`).
- [A] `:focus-visible` rather than `:focus`.
- [A] `:focus-within` for compound controls (search bar with a clear button).
- [A] Sticky headers, bars and overlays do not cover the element that has focus.
- [C] Modals and panels: focus trap and focus returned to the originating element (the common `Modal` does it; home-made overlays do not, defect D10).

## Forms

- [C] Explicit `name`; `autocomplete="off"` on fields not tied to an account (search, rename) to avoid the password manager.
- [A] Correct `type` (`number`, `url`, `search`) and `inputmode`.
- [A] Never block pasting.
- [A] Clickable labels (`for` or wrapping label).
- [A] `spellcheck="false"` on codes, paths, URLs, identifiers.
- [A] Checkbox and radio: label and control share a single click area.
- [A] Submit button enabled until the request starts, then an indicator during the request.
- [A] Inline errors next to the field; focus on the first error.
- [C] Placeholders: sample value ending with `…` ("Search for a title, an artist…" / « Rechercher un titre, un artiste… »), never used as a label.
- [A] Warn before leaving with unsaved changes (track editor, settings).

## Animation

- [A] Respect `prefers-reduced-motion`: `motion-reduce:animate-none`, `motion-reduce:transition-none`.
- [A] Animate only `transform` and `opacity` (and colour for hovers).
- [A] Never `transition: all` / `transition-all`: list the properties.
- [A] Correct `transform-origin`.
- [A] SVG: transforms on a `<g>` with `transform-box: fill-box; transform-origin: center`.
- [A] Interruptible animations.
- [C] No decorative looping animation; only spinners and loading indicators loop.

## Typography

- [A] `…` and not `...` (including in `en.json` and `fr.json`).
- [C] Typographic quotes: « » in French, “ ” in English; never straight `"` in visible text.
- [A] Non-breaking spaces: `10&nbsp;MB` (`10&nbsp;Mo` in French), `⌘&nbsp;K`; in French, a non-breaking space before `: ; ! ?` and as the thousands separator.
- [A] Loading states end with `…`: "Loading…", "Analysing…" (« Chargement… », « Analyse… »).
- [A] `tabular-nums` for columns and number comparisons.
- [A] `text-wrap: balance` / `text-pretty` on headings that wrap.

## Content

- [A] Text containers robust to long content: `truncate`, `line-clamp-*`, `break-words` (very long track titles, label names, multiple remixers).
- [A] Flex children with `min-w-0` to allow truncation.
- [A] Empty states handled: no broken interface for an empty string or list (empty library, stats with no plays, empty playlist).
- [A] User data: plan for short, medium and very long.

## Images

- [A] `<img>` with explicit `width` and `height` (or a fixed-size container) to avoid layout shifts.
- [A] Off-screen: `loading="lazy"` (artwork in long lists).
- [—] `fetchpriority="high"` on the critical image (no LCP in a local app).

## Performance

- [A] Lists of more than 50 items: virtualised (`@tanstack/virtual-core`, already used by `TrackList`) or `content-visibility: auto`.
- [A] No layout reads during rendering (`getBoundingClientRect`, `offsetHeight`…); batch DOM reads and writes.
- [A] Controlled fields cheap on every keystroke (search with debounce).
- [C] Fonts embedded in `static/fonts` with `font-display: swap`; the Google Fonts import in `style.css` is a leftover to be removed eventually (offline app).
- [—] CDN `preconnect`, video rather than GIF.

## Navigation and state

- [C] No shareable URL in a Tauri app: important state (view, filters, sort, columns, panel widths) is **persisted** in the settings or the store, and restored at launch.
- [A] Destructive actions: `ConfirmModal` or an undo window, never immediate (delete, format a device, replace a file).

## Interaction

- [—] `touch-action`, `-webkit-tap-highlight-color` (desktop).
- [A] `overscroll-behavior: contain` in scrolling modals, panels and menus.
- [A] During a drag: no text selection (already a global `user-select: none`), dragged element `inert`.
- [A] Gestures (drag, mouse wheel on a slider): click and keyboard alternative.
- [C] `autofocus` only on the main field of a modal (rename, create a playlist).

## Layout

- [—] `env(safe-area-inset-*)`.
- [A] No stray scrollbar: fix the overflow rather than hide it (`overflow-x-hidden` as a last resort).
- [A] Flex and grid rather than JavaScript measurements.

## Theme

- [C] `color-scheme` is set by `[data-theme]` in `style.css`: do not override it.
- [—] `<meta name="theme-color">`.
- [A] Native `<select>`: explicit `background-color` and `color` (the common `Select` avoids the problem).

## Language

- [A] Dates and times: `Intl.DateTimeFormat` (through the svelte-i18n locale), never a hard-coded format.
- [A] Numbers: `Intl.NumberFormat` (« 2 310 » in French, not « 2,310 », defect L3).
- [C] Language: the app setting (`crate-language`), otherwise `navigator.languages`.
- [A] Brand names, codes and identifiers (Camelot `8A`, BPM, file names): `translate="no"`.

## Hydration

- [—] Not applicable: `adapter-static` without server rendering.

## Hover and interactive states

- [A] Buttons and links have a `hover:` state.
- [A] Interactive states increase contrast: hover, press and focus more pronounced than rest.

## Copywriting

- [A] Active voice: "Export the playlist" and not "The playlist will be exported" (« Exporter la playlist » / « La playlist sera exportée »).
- [C] Case: Title Case for headings and buttons in English (upstream convention); sentence case in French (« Créer une playlist »).
- [A] Numerals for counts: "8 tracks" and not "eight tracks" (« 8 pistes » / « huit pistes »).
- [A] Precise labels: "Delete 12 Duplicates" rather than "Continue" (« Supprimer 12 doublons » / « Continuer »).
- [A] Error messages say how to fix the problem, not just what is wrong.
- [C] Second person (« vous » in French, like all of `fr.json`; "you" in English), no first person.
- [A] `&` rather than "and" when space is short (English).

## Anti-patterns to flag

- `user-scalable=no` or `maximum-scale=1`.
- Blocked pasting.
- `transition-all`.
- `outline-none` without a `focus-visible` replacement.
- Navigation through `onclick` without `<a>`.
- Clickable `<div>` or `<span>`.
- Images without dimensions.
- Large lists without virtualisation.
- Fields without a label.
- Icon buttons without an accessible name.
- Hard-coded date or number formats.
- Unjustified `autofocus`.
- Gesture-only action without a click and keyboard alternative.

## Source licence

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
