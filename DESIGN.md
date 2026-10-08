---
version: 1
name: Crate
description: "Desktop DJ library manager (Tauri + SvelteKit + Tailwind 4). A dense, calm working tool: near-black zinc background (dark theme by default) or white (light theme), tight track tables where numbers are in `tabular-nums`, and data badges (Camelot key, energy). Colour comes from the accent chosen by the user, which drives the whole app, and from three scoped colour families that each own one view: Deck (Player: cyan and amber, neon glass), Pulse (statistics: one hue per metric, glass cards) and Beatport (neon green). The interface steps back behind the music."

# The values below are those of apps/desktop/src/style.css.
# In code, NEVER write these hex values: use the token's Tailwind class (bg-surface-1, text-text-secondary…).
colors:
  dark:
    surface-0: "#09090b"   # window background (zinc-950)
    surface-1: "#18181b"   # panels, modals, menus (zinc-900)
    surface-2: "#27272a"   # controls, hover, raised elements (zinc-800)
    surface-3: "#3f3f46"   # hover/press on a raised element, skeletons (zinc-700)
    surface-4: "#52525b"   # rare: progress track, handle (zinc-600)
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
  accent:                  # [data-accent], chosen in Settings > Display; blue by default
    brand-primary: "var(--brand-primary)"   # blue #3b82f6, indigo, violet, purple, pink, rose, orange, amber, emerald, teal
    brand-hover: "var(--brand-hover)"       # -600 shade, or -400 for blue, indigo, violet, purple, rose so that black text stays above 4.5:1
    brand-muted: "var(--brand-muted)"       # accent at 20%: background of the active element
    brand-on: "var(--brand-on)"             # text on an accent fill: black or white per accent, whichever passes 4.5:1 (black for all ten)
  state:                   # identical in both themes (see "Known gaps")
    danger: "#ef4444"
    warning: "#f59e0b"
    success: "#22c55e"
    info: "#3b82f6"
  data:                    # DATA palettes, centralised, never copied into a component
    camelot: "shared/utils/camelot.ts (CAMELOT_COLORS, getCamelotColor) — Mixed In Key 11 wheel"
    energy: "shared/utils/energy.ts (getEnergyInfo) — levels 1 to 10; `color` is the dark text, `lightColor` (-800/-900 shade of the same hue) the light text, picked with light-dark()"
  sources:                 # third-party service colours: identify a source, nothing else (see "Graphic charter")
                           # tokens --source-<name> (fill), -text (readable text, darker in light), -wash (card gradient top)
    spotify: "#1DB954 (--source-spotify; light text green-800)"
    beatport: "#00FF96 (--beatport)"
    mixed-in-key: "#00D2FF (--source-mik; light text cyan-700); toolbar sync button --source-mik-tint sky-500, --source-mik-bright sky-400, --source-mik-hover-wash sky-950 / sky-100; logo label and sync icon --source-mik-logo-text sky-400 / sky-700"
    rekordbox: "red-500 (--source-rekordbox; dark text red-400, light text red-700)"
    crate-local: "violet-500 (--source-crate; dark text violet-400, light text violet-700)"
  families:                # scoped colour families (see "Graphic charter"); tokens in style.css, section "Colour families"
                           # (owner decision CRA-141, 2026-10-01). Dark values are the palette shades the views always had.
    deck:                  # Player view, Duplicate Killer and Upgrader waveform previews
      live: "--deck-live cyan-400 (fills, both themes), --deck-live-tint cyan-500"
      live-text: "--deck-live-text cyan-400 dark / cyan-700 light; --deck-live-text-strong cyan-300 / cyan-700"
      on-live: "black"
      cue: "--deck-cue amber-400, --deck-cue-tint amber-500, --deck-cue-outline amber-200 dark / stroke-strong light"
      cue-text: "--deck-cue-text amber-400 / amber-700; --deck-cue-text-strong amber-300 / amber-700"
    pulse:                 # Pulse (statistics) view, one hue per metric: --pulse-<metric> (fill) and --pulse-<metric>-text
      listening-time: "emerald-500 fill, text emerald-400 / emerald-800 (bright and brighter steps for the heatmap)"
      plays: "purple-500 fill, text purple-400 / purple-700"
      artists: "pink-500 fill, text pink-400 / pink-700"
      dj-sessions: "amber-500 fill, text amber-400 / amber-700"
      keys: "cyan-500 fill, text cyan-400 / cyan-700"
      ranks: "gold amber-400 (text amber-300 / amber-800), silver slate-400 (slate-300 / zinc-600), bronze amber-600 (amber-600 / amber-800)"
    beatport:              # Beatport view, Beatport modals, Upgrader, Beatport settings tab
      neon: "--beatport #00FF96 (fills with black text), --beatport-hover #00e687"
      text: "--beatport-text emerald-400 / emerald-800; --beatport-text-strong #00FF96 / emerald-700; --beatport-status-text emerald-500 / emerald-700 (\"Connected\")"
      bright: "--beatport-bright emerald-400 (cart download hover, Upgrader arrow)"
      badge: "--beatport-badge emerald-500 / emerald-700 (the Upgrader count badge in the toolbar, white text)"
      body-text: "--beatport-body-text neutral-300 / zinc-700; --beatport-body-text-strong neutral-200 / zinc-800"
      tints: "--beatport-tint emerald-500, --beatport-deep emerald-600, --beatport-wash emerald-950 / emerald-100"
    warning:               # the Duplicate Killer toolbar alert, the Beatport session banner
      tokens: "--warning-tint amber-500, --warning-text amber-400 / amber-700, --warning-text-strong amber-300 / amber-800, --warning-text-soft amber-200 / amber-800, --warning-wash amber-950 / amber-100"
    danger:                # upstream red: destructive items and buttons, errors, the favourite heart
      tokens: "--danger-tint red-500 (tints), --danger-text red-400 / red-700 (hovers, heart), --danger-text-muted red-500 / red-700 (menu items, ghost-danger, error text and icons), --danger-fill red-600 and --danger-fill-hover red-700 (confirmed destructive action, white text), --danger-badge red-500 / red-700 (Duplicate Killer count badge, white text)"
    tools:                 # Duplicate Killer and the Upgrader: --tool-<role> (fill, both themes) and --tool-<role>-text
      match: "emerald-500, text emerald-400 / emerald-800 (identical audio, lossless file, nothing to clean)"
      fuzzy: "sky-500, text sky-400 / sky-800 (metadata match)"
      partial: "teal-500, text teal-300 / teal-800 (Upgrader match under 90%)"
      local: "amber-500, dot amber-400, text amber-400 / amber-800, strong amber-300 / amber-800 (the local file, the recommended keep)"
      energy: "amber-500, text amber-400 / amber-800 (energy chip)"
      delete: "rose-500, wash rose-950 / rose-100, text rose-400 / rose-700 (marked for deletion)"
      cues: "indigo-500, text indigo-300 / indigo-700 (cue count)"

typography:
  family: "var(--font-family) — [data-font] chosen by the user; Open Sans by default (app.html), also Jost, DM Sans, Inter, Nunito, Fira Code, IBM Plex Mono, Source Code Pro"
  mono: "Tailwind font-mono (system stack) for keys, BPM, durations, codes"
  # Scale carried by the <Text variant=…> component (common/Text.svelte)
  header-1: { size: 18px, class: text-lg, weight: 600, color: text-primary, element: h2 }
  header-2: { size: 14px, class: text-sm, weight: 600, color: text-primary, element: h3 }
  header-3: { size: 14px, class: text-sm, weight: 600, color: text-secondary, uppercase: true, tracking: wide }
  header-4: { size: 12px, class: text-xs, weight: 500, color: text-tertiary, uppercase: true, tracking: wide }
  header-table: { size: 12px, class: text-xs, weight: 500, color: text-tertiary, uppercase: true, tracking: wider }
  body-1: { size: 14px, class: text-sm, weight: 400, color: text-primary }
  body-2: { size: 14px, class: text-sm, weight: 500, color: text-primary }
  caption: { size: 12px, class: text-xs, weight: 400, color: text-tertiary }
  code: { size: 12px, class: "text-xs font-mono", weight: 400 }
  display: { size: 24px, class: "text-2xl", weight: 600, use: "only the Pulse KPI values and the title of the Player hero, album detail and Beatport artist header" }
  numbers: "tabular-nums on every column or comparison of numbers (BPM, duration, counters, stats)"

rounded:
  control: "rounded-md (6px) — buttons, fields, select, menu items"
  small: "rounded (4px) — data badges, checkbox, tooltip, kbd"
  panel: "rounded-lg (8px) — cards, modals, dropdown menus, panels"
  pill: "rounded-full — pills, switches, sliders, avatars"
  glass: "rounded-xl (12px) — only the neon-glass cards of Pulse and the Player's artwork and hero panels"
  forbidden: "rounded-2xl, rounded-3xl everywhere; rounded-xl outside the neon-glass material"

elevation:
  flat: "no shadow: table rows, panels sitting on surface-0"
  shadow-sm: "element that protrudes slightly (active segment, badge)"
  shadow-lg: "menus, tooltips, open select, toasts, neon-glass cards"
  shadow-xl: "modals"
  neon-glass: "Player and Pulse only: bg-surface-1/60–70 + backdrop-blur-xl; glow on the Player's live elements only (see Graphic charter)"
  forbidden: "shadow-2xl, decorative gradients, glow or coloured shadows on cards and buttons, and any glass or glow outside the neon-glass scope"

spacing:
  base: "Tailwind scale (4px)"
  row: "track row: px-3 py-1.5, gap-2, border-b border-stroke-subtle"
  control-gap: "gap-2 between controls, gap-4 between groups"
  panel-padding: "p-4 (menus p-1/p-2, modals p-5/p-6)"

motion:
  duration: "150 to 200 ms"
  properties: "colour, opacity, transform — list the properties (transition-colors, transition-[…]), never transition-all"
  svelte: "fade/scale from svelte/transition for appearances; duration ≤ 200 ms"
  infinite: "reserved for spinners and loading indicators, always with motion-reduce:animate-none"

window:
  min: "1000×600 (tauri.conf.json)"
  default: "1400×900"
  check: "1000×600, 1400×900, 1920×1080"

components:
  Button: { variants: [primary, secondary, ghost, danger, ghost-danger, outline], sizes: [sm, md, lg, bare], rounded: control, primary-text: "text-brand-on", tones: "primary: accent (default look); deck, beatport, spotify, rekordbox render the family look (bold, shape lg/xl/full, glow, press, lift), each only in its own scope", props: "shape, weight (bold, semibold), display (inline-flex, flex), glow, press, lift; fill replaces the primary fill for a non-accent primary (red confirm, Upgrader replace)" }
  IconButton: { sizes: [sm 24px, md 32px, lg 40px], active: "bg-brand-muted text-brand-primary", rule: "accessible name required" }
  Text: { variants: [header-1, header-2, header-3, header-4, header-table, body-1, body-2, caption, code] }
  Input: { background: surface-2, border: stroke, rounded: control }
  Select: { trigger: surface-2, menu: "surface-1 shadow-lg rounded-lg z-50" }
  Modal: { element: "native <dialog>", background: surface-1, border: stroke, rounded: panel, shadow: shadow-xl, sizes: [sm, md, lg, xl, 2xl, 3xl, 4xl, none], rule: "height bounded by the window, fixed footer", props: "panelClass and backdropClass for a family panel (Spotify connection), theme=\"dark\" for a panel that stays dark in both themes" }
  Tooltip: { background: surface-1, border: stroke, rounded: small, text: text-xs, portal: true }
  ContextMenu: { background: surface-1, rounded: control, shadow: shadow-lg }
  Checkbox: { appearance: "crate (default: accent box) or native (the system checkbox of the confirmation dialogs, Duplicate Killer and the Upgrader; label as children, labelClass, inputClass)" }
  SegmentedControl: { semantics: "radiogroup, one tab stop, arrow keys, Home/End", variants: "switcher (view switcher, sliding thumb), deck (Player, cyan fill), boxed (Pulse period)", rule: "a value that matches no option selects nothing", scrollable: "boxed and deck: one line, horizontal scroll with arrows and edge fades when the options do not fit (Pulse period)" }
  scroll-affordance: { arrows: "IconButton sm, chevron-left/right, text-text-secondary, out of the tab order, translated name, shown only on a side that hides content", fade: "alpha mask .scroll-affordance in style.css (transparent under the arrow, 1.25rem ramp), no colour", scrollbar: hidden, selection: "always scrolled into view, clear of the arrows", motion: "smooth scroll, instant under reduced motion" }
  ToggleSwitch: {}
  Slider: {}
  Spinner: { color: "muted (default) or current", icon: "refresh (default), refresh-cw, loader (Player views, toolbar export), arc (cloud sync), ring (CSS ring, size and colour from class)", motion: "motion-reduce:animate-none", rule: "every busy indicator is a Spinner; IconButton has busy" }
  Toast: {}
  Icon: { source: "common/Icon.svelte (internal set); a Vitest test fails on an unknown name" }
  track-row: { layout: "grid, items-center, gap-2, px-3 py-1.5, text-sm", selected: "bg-brand-muted", playing: "title in text-brand-primary" }
  key-badge: { component: "common/KeyBadge.svelte", variants: "one per view, reproducing today's look (cell, cell-compact, chip, chip-plain, tag, tag-wide, pill, pill-xs)", colors: "getCamelotColor(key), through keyBadge.ts only", neutral: "border-stroke bg-surface-3 when the key is not a Mixed In Key analysis (library)" }
  energy-badge: { component: "common/EnergyBadge.svelte", colors: "getEnergyInfo(level); text light-dark(lightColor, color)", variants: "badge (library: level symbol, glow), pill (Player hero: fixed ⚡)" }
---

# Crate — design system

> **Owner decision (CRA-141, 2026-10-01).** The owner likes today's look; the charter describes it and implements five points only: black or white text on an accent fill chosen per accent (`--brand-on`); named colour tokens for every family; the Beatport view following the light and dark themes; and four homogenisation touches (Pulse cards `rounded-xl`, no hover lift or zoom on the Pulse KPI cards, the Player's cyan in the Duplicate Killer and Upgrader waveform previews, a family colour on a toolbar shortcut only as a label or count badge). **Nothing else about the dark theme changes**: a token's dark value is the shade the view already had. Where a rule below still differs from the code, the deviation is listed in [Known deviations](#known-deviations-from-the-charter) and is not a licence to restyle.

This document describes **how Crate must look**. It is read by coding assistants (the `design` agent in `.claude/agents/design.md`) before any interface work, and serves as a reference for the owner. It is extracted from the actual code (`apps/desktop/src/style.css`, `lib/components/common/`) and from the "strict design rules" of the [defect register](tracking/DEFECTS.md#design-responsive-and-accessibility).

## Overview

Crate is a **DJ tool**, not a marketing page. People spend hours in it sorting, listening, tagging and preparing sets: the interface must be quick to read, stable, dense without being stifling, and perfectly consistent from one view to the next.

- **High density, low variance, minimal motion.** In terms of "dials" (borrowed from taste-skill): `VARIANCE 2 · MOTION 2 · DENSITY 8`. Symmetry and predictable alignment, animation only to signal a change of state, a lot of information per screen.
- **Colour is meaningful and always scoped.** Surfaces are neutral (zinc). Colour comes from the accent chosen by the user (selection, focus, primary action, now playing), state colours (error, warning, success), data palettes (Camelot key, energy), source colours (Spotify, Beatport…), and three colour families that each own one view.
- **One system, four colour families.** Owner decision (CRA-115, 2026-09-30): the colour languages added in builds 40 to 57 are **kept**, not merged into the accent. The accent stays Crate's own colour and drives the whole app; the Player (Deck: cyan and amber, neon glass), Pulse (one hue per metric, glass cards) and Beatport (neon green) keep their identity **inside their own scope**. Everything else (surfaces, type, shapes, spacing, states, motion, contrast) is shared. The limits are set by the [Graphic charter](#graphic-charter-colour-families-and-their-limits); a fifth family is an owner decision.
- **Two themes at parity.** Dark by default, light complete. Every view must be right in both, with all ten accents.

## Colours

### Surfaces (elevation scale)

From deepest to highest: `surface-0` (window background) → `surface-1` (panels, modals, menus) → `surface-2` (controls, hover) → `surface-3` (hover on an already raised element, loading skeletons) → `surface-4` (rare). Do not skip a level and do not create an intermediate surface through opacity: `bg-surface-1/70` exists only in the neon-glass material of the Player and Pulse ([Graphic charter](#graphic-charter-colour-families-and-their-limits)) and is forbidden everywhere else.

### Text

`text-primary` for content, `text-secondary` for metadata (artist, album), `text-tertiary` for labels and empty values ("-"), `text-disabled` for inactive items. Never palette greys (`text-zinc-400`) or hex values.

### Strokes

`stroke-subtle` between table rows, `stroke` for control and panel borders, `stroke-strong` for an outline that must be visible (secondary focus, pronounced separator).

### Accent

The accent is **chosen by the user** (`[data-accent]`). Consequence: outside the three family scopes of the [Graphic charter](#graphic-charter-colour-families-and-their-limits), a view never picks "its own" colour; inside them, a family takes over only the roles the charter lists, and focus stays on the accent everywhere. `brand-primary` is used for the primary action, focus, and the selected or playing element; `brand-muted` (accent at 20%) is the background of the active element. For other opacities, `bg-brand-primary/10` works (`inline` theme), as do the `bg-brand-primary-5` and `bg-brand-primary-10` utilities. Every view must be checked with at least two accents (the default blue and a warm accent such as orange or amber).

### States

`danger`, `warning`, `success`, `info`: errors, warnings, confirmations, in every view and every family. They are not used as decoration, and a family hue never stands in for a state (Pulse's emerald means "listening time", not "success"; the Player's amber means "cue", not "warning").

### Data palettes

The Camelot key colours (Mixed In Key 11 wheel) and energy colours encode information the DJ reads at a glance: they are legitimate. Rules:

1. They live **only** in `shared/utils/camelot.ts` and `shared/utils/energy.ts`; a component obtains them through `getCamelotColor()` / `getEnergyInfo()` and applies them with `style=`.
2. They colour the **badge**, not its surroundings (no fully tinted row, no glow).
3. The badge text keeps a contrast of at least 4.5:1 against its background, in both themes. The energy palette gives each level a dark text colour (`color`) and a light one (`lightColor`, the `-800` or `-900` shade of the same hue), and `EnergyBadge` picks between them with `light-dark()`, which follows the `color-scheme` each `[data-theme]` block sets (so a badge inside a panel that stays dark keeps its dark shade). Light theme: 5.74 to 7.34:1 on the usual surfaces; the worst case of each level, over `surface-0` to `surface-3` and a selected row with any accent, goes from 4.64 to 5.85:1. `light-dark()` needs WebKit 17.5 or later, which every supported macOS has (Crate runs on this Mac only, CRA-122).

### Source colours

The official colours of the services a listen or a track comes from: Spotify `#1DB954`, Beatport `#00FF96`, Mixed In Key `#00D2FF`, Rekordbox `#EF4444`, Crate local `#8B5CF6`. They **identify a source** and nothing else: a logo, a source chip or badge, a segment of the Pulse source bar, and the service's own connect button (Spotify's green button with black text). They are never a selection, hover, focus or state colour. They are tokens (`--source-spotify`, `--source-mik`, `--source-rekordbox`, `--source-crate`, and `--beatport`), each with a `-text` value that stays readable in the light theme; the Pulse source bar still carries its segment colours as hex values in a script map (`StatsSourceBar`).

## Graphic charter: colour families and their limits

Owner decision (Linear CRA-115, 2026-09-30): Crate keeps several colour families, and each one gets precise limits, so that the whole stays consistent, professional and durable. This section is written **from the visual language that exists today**, and its roles are tokens in `apps/desktop/src/style.css` (section "Colour families"): a component writes `text-deck-live-text`, never `text-cyan-400`. The remaining deviations are listed in [Known deviations](#known-deviations-from-the-charter).

A family is four things: a **purpose** (what its colour means), a **scope** (the only places it may appear), **roles** (which element takes which colour), and **light and dark values**. Everything that is not a colour (surfaces, type, shapes, spacing, states, motion, contrast) is shared by all families and described in [Rules shared by all families](#rules-shared-by-all-families).

### Family map

| Family | Purpose | Scope: the only places it appears | Must never appear in |
| --- | --- | --- | --- |
| **A. Accent** (Crate core, upstream) | Crate's own colour, chosen by the user: primary action, selection, focus, now playing in the library | Everywhere: toolbar, sidebar, Library, playlists, tags, discovery, Settings, onboarding, every modal, the bottom player bar, every common component; **focus rings in every view, including family views** | Nowhere forbidden, but inside a family view it keeps only focus, controls that belong to Crate (period selector, "import to library", refresh) and common components |
| **B. Deck** (Player, neon glass) | The performance surface: cyan is "the audio that is playing", amber is "a cue" | The Player view (`PlayerView`, `AlbumGridView`, `AlbumDetailView`); the waveform previews of Duplicate Killer and Upgrader; the label of the Player segment in the view switcher | Library rows (playing there is the accent), bottom player bar, Pulse, Beatport, Settings, toolbar tools, common components |
| **C. Pulse** (statistics, glass cards) | Listening statistics: each metric has one hue, the same in its KPI card, its chart and its list | The Pulse view (`components/stats/`) | Anywhere outside Pulse; never as a state colour |
| **D. Beatport** (neon green) | "This comes from Beatport": catalogue, streaming, cart, purchase, upgrade | The Beatport view (`components/beatport/`), the Beatport modals, the Beatport Upgrader, the Beatport settings tab, the label of the Beatport segment in the view switcher, the upgrader count badge in the toolbar | Player, Pulse (except the Beatport source colour), Library, Settings outside its tab, common components |

States, data palettes and source colours are not families: they cross every scope with their own rules (above).

### Allowed combinations

| Where | Accent | Deck | Pulse hues | Beatport | Source colours | States, data palettes |
| --- | --- | --- | --- | --- | --- | --- |
| Shell (toolbar, sidebar, view switcher) | yes | Player segment label only | no | Beatport segment label and upgrader count badge only | logos (Mixed In Key badge) | yes |
| Library, playlists, discovery, Settings, generic modals | yes | no | no | Beatport settings tab only | logos, source badge | yes |
| Player view | focus, Crate actions | yes | no | no | one source badge per track (e.g. "Beatport" on a streamed track) | yes |
| Pulse view | header icon, period selector, refresh, focus | no | yes | no | yes (source chips, source bar, integration cards) | yes |
| Beatport view and modals, Upgrader | focus | waveform preview only | no | yes | Beatport logo | yes |
| Duplicate Killer | yes | waveform preview only | no | no | no | yes (`danger` for what will be deleted) |
| Common components (`common/`) | yes | only through an explicit `tone` or `variant`, used in the Player | no | only through an explicit `tone`, used in Beatport's scope | only through an explicit `tone` (Spotify's own connect button) | yes |

A family colour never sits next to another family's colour in the same element (no cyan and neon green in one row, no gradient between two families). Entry points in the shell may carry the destination family's colour **only as a label or a count badge**, never as the fill or border of the button.

### A. Accent — Crate core

- **Tokens**: `brand-primary`, `brand-hover`, `brand-muted` (20%), `bg-brand-primary/10`, `bg-brand-primary-5`, `bg-brand-primary-10`. Ten accents (`[data-accent]`), blue by default; the value is the same in both themes.
- **Roles**: primary action fill (`Button variant="primary"`), selected row (`bg-brand-muted`), playing track title in the library (`text-brand-primary`), active icon button (`bg-brand-muted text-brand-primary`), progress of the bottom player's seek bar, logo, focus outline (`*:focus-visible`, global).
- **Text on an accent fill** is `text-brand-on` (owner decision CRA-141): each `[data-accent]` block sets `--brand-on` to black or white, whichever reaches the higher contrast on that fill. Black wins for all ten accents (4.70 to 9.78:1; white gives 2.15 to 4.47). Every text on an accent fill uses it: buttons, the count badges (following, filter, drag preview, merge releases), the `Checkbox` check mark, tag chips and the date picker. On hover the fill takes `brand-hover`, the `-600` shade, except for blue, indigo, violet, purple and rose, where the darker shade put black text under 4.5:1: those five lighten to their `-400` shade instead (7.04 to 8.26:1). `theme.test.ts` recomputes both for every accent, so a new accent cannot ship with unreadable text at rest or on hover.
- **Accent as text** on a light surface stays under 4.5:1 for every accent (2.06 to 4.28): in the light theme the accent marks with a fill, an outline or an icon, and text next to it stays `text-primary`.

### B. Deck — the Player's neon glass

The only family with a material of its own: a translucent hero over the blurred artwork, and light that glows on what is live.

| Role | Dark | Light | Token |
| --- | --- | --- | --- |
| Live fill: played waveform bars, main play button, playhead progress, waveform previews of Duplicate Killer and the Upgrader | `cyan-400` | same | `--deck-live` |
| Live tint: active segment, now-playing row (`/10`), chip backgrounds | `cyan-500` | same | `--deck-live-tint` |
| Text on a live fill | black | black | — |
| Live text and icons: time readout, now-playing row, EQ bars, "Album" label | `cyan-400` | `cyan-700` | `--deck-live-text` |
| Strong live text: BPM, hover of live text | `cyan-300` | `cyan-700` | `--deck-live-text-strong` |
| Cue fill: hot-cue pins, filled pads | `amber-400` (tint `amber-500`) | same | `--deck-cue`, `--deck-cue-tint` |
| Cue pin outline | `amber-200` | `stroke-strong` (the amber fill alone is 1.52:1) | `--deck-cue-outline` |
| Cue text: pad labels, cue badges | `amber-400` (strong `amber-300`) | `amber-700` | `--deck-cue-text`, `--deck-cue-text-strong` |

- **Neon-glass material**: `bg-surface-1/60` + `backdrop-blur-xl` on the hero and on the sticky header of the recent tracks list; the ambient layer is the artwork itself, blurred (`blur-[100px]`, opacity 15% light, 30% dark), decorative and `aria-hidden`.
- **Glow**: only on four live elements — played waveform bars, playhead, cue pins, main play button — in the family hue, at most 8 px of blur (`shadow-[0_0_6px_…]`, `shadow-lg shadow-cyan-400/40`). No glow on text, rows, cards or secondary buttons.
- **Two hues only.** Cyan and amber; anything else in the Player uses the accent, the states or the data palettes (Camelot and energy badges).
- **Never**: Deck cyan for the library's now playing, the bottom player bar, the toolbar's export button or a common component; amber for a warning (warnings use `warning`).

### C. Pulse — glass dashboard

| Metric | Fill (chip, chart) | Dark text | Light text | Tokens | Where |
| --- | --- | --- | --- | --- | --- |
| Listening time, activity heatmap | `emerald-500` (heatmap steps `emerald-400`, `emerald-300`) | `emerald-400` | `emerald-800` | `--pulse-listening`, `-bright`, `-brighter`, `-text` | KPI card, heatmap, BPM distribution |
| Plays | `purple-500` | `purple-400` | `purple-700` | `--pulse-plays`, `-text` | KPI card |
| Artists | `pink-500` | `pink-400` | `pink-700` | `--pulse-artists`, `-text` | KPI card, top artists |
| DJ sessions (Rekordbox) | `amber-500` | `amber-400` | `amber-700` | `--pulse-sessions`, `-text` | KPI card |
| Keys | `cyan-500` | `cyan-400` | `cyan-700` | `--pulse-keys`, `-text` | harmonic wheel |
| Rank 1, 2, 3 | gold `amber-400`, silver `slate-400`, bronze `amber-600` (`amber-700` fill) | `amber-300`, `slate-300`, `amber-600` | `amber-800`, `zinc-600`, `amber-800` | `--pulse-gold`, `--pulse-silver`, `--pulse-bronze`, `-fill`, `-text` | top tracks, top artists |

Listening time and gold use the `-800` shade in the light theme because their text sits on its own tint (`/15`, `/20`), where `-700` gives 4.4:1. The BPM distribution bars run from `--pulse-listening` to `--pulse-listening-end` (`teal-400`). The source bar's legend dots take the same token as their segment.

- **One hue per metric, fixed.** A metric keeps its hue in its KPI card, its chart and its list; a new metric gets a hue only by an owner decision and an entry in this table.
- **Hue as tint**: the chip or icon background is the hue at 15% (`bg-pulse-listening/15`), its border at 30%; a card is never fully tinted and never gradient-filled.
- **Glass cards**: the top-level cards of Pulse are `bg-surface-1/70` + `backdrop-blur-xl` + `border-stroke/60` + `shadow-lg` + `rounded-xl`, one level only (no glass inside glass, the heatmap tooltip included). The sticky header is `bg-surface-1/90` + `backdrop-blur-xl`.
- **Halo**: one static blurred orb (`blur-2xl`, hue at 10%) in the corner of each of the four KPI cards. The KPI cards do not lift and their icon does not zoom on hover (CRA-141); the hover keeps today's border, shadow and halo change.
- **Accent in Pulse**: the header icon, the period selector, the refresh button and focus stay on the accent.
- **Source colours** appear in Pulse on source chips, the source bar and the integration cards (border at 25%, icon, and a gradient top from the source's `-wash` token, which is the neutral `surface-1` in the light theme). The Spotify connection modal keeps its dark green glass panel in both themes (`Modal theme="dark"`, `--source-spotify-panel`); its links, codes and manual-code button use `--source-spotify-link-tint` and `--source-spotify-link-text` (emerald), its warning the warning tokens, and the card's "Disconnect" the danger tokens.

### D. Beatport — neon green

| Role | Dark | Light | Token |
| --- | --- | --- | --- |
| Primary action fill (log in, buy, add to cart, play) | `#00FF96`, hover `#00e687` | same | `--beatport`, `--beatport-hover` |
| Text on the fill | black (15.74:1) | black | — |
| Tints, borders | `emerald-500` (`emerald-600` for the Upgrader's replace buttons) | same | `--beatport-tint`, `--beatport-deep` |
| Upgrader count badge in the toolbar (white text) | `emerald-500` | `emerald-700` (white 5.36:1) | `--beatport-badge` |
| Washes: now-playing row, header, progress | `emerald-950` | `emerald-100` | `--beatport-wash` |
| Active navigation, secondary text, counters | `emerald-400` | `emerald-800` (on its own `/15` tint `-700` gives 4.3:1) | `--beatport-text` |
| "Connected" status in the sidebar | `emerald-500` | `emerald-700` | `--beatport-status-text` |
| Bright fill: the cart's download hover, the Upgrader's arrow | `emerald-400` | same | `--beatport-bright` |
| Neon text: playing title, links, codes | `#00FF96` | `emerald-700` | `--beatport-text-strong` |
| Body copy of the gateway, login and cart panels | `neutral-300` (emphasis `neutral-200`) | `zinc-700` (`zinc-800`) | `--beatport-body-text`, `--beatport-body-text-strong` |
| Now-playing row | left border `#00FF96`, background `beatport-tint/10` | same | — |

- **Surfaces follow the theme (owner decision CRA-141).** The Beatport view, its modals and panels use the neutral tokens (`surface-0…4`, `stroke…`, `text-…`). Their dark values are the view's own dark surfaces (`#121418`, `#181a20`, `#0e1014`, `#252830`, `#2e323d`, white text): elements that carry `data-surface="beatport"` (the cart drawer, the login modal, the logged-out gateway card) get them through a dark-only scope in `style.css`, so the dark theme is unchanged and the light theme is Crate's light theme.
- **Flat**: no glass, no glow (`shadow-[#00FF96]/20`), no gradient, no hover scale.
- `#00FF96` is never text on a light surface (1.28:1).

### Library tools — Duplicate Killer and the Upgrader

Not a family: these two modals use the accent, the Deck cyan for their waveform previews and the Beatport family for the Upgrader's Beatport side. Their own chips and markers are role tokens, `--tool-<role>` for the fill (tints at 10 to 30%, both themes) and `--tool-<role>-text` for the text, with a light value at `-800` because the text sits on its own tint:

| Role | Fill | Dark text | Light text | Token |
| --- | --- | --- | --- | --- |
| Identical audio, lossless file, nothing left to clean | `emerald-500` | `emerald-400` | `emerald-800` | `--tool-match` |
| Metadata match | `sky-500` | `sky-400` | `sky-800` | `--tool-fuzzy` |
| Upgrader match under 90% | `teal-500` | `teal-300` | `teal-800` | `--tool-partial` |
| The local file (Upgrader's "Current"), the recommended keep | `amber-500`, dot `amber-400` | `amber-400`, strong `amber-300` | `amber-800` | `--tool-local` |
| Energy chip | `amber-500` | `amber-400` | `amber-800` | `--tool-energy` |
| Marked for deletion | `rose-500`, wash `rose-950` (light `rose-100`) | `rose-400` | `rose-700` | `--tool-delete` |
| Cue count | `indigo-500` | `indigo-300` | `indigo-700` | `--tool-cues` |

### Rules shared by all families

| Topic | Rule for every family |
| --- | --- |
| Surfaces | Neutral tokens only (`surface-0…4`); a family colours chips, fills, text and borders, never a surface. No hard-coded surface hex (a view with its own dark surfaces scopes the neutral tokens, as `data-surface="beatport"` does). Opacity on a surface (`bg-surface-1/70`) exists only in the neon-glass material. |
| Tints | Family hue at 10–20% for backgrounds, 20–40% for borders; hover raises the tint by one step (`/10` → `/20`). |
| Borders | `stroke-subtle` between rows, `stroke` around controls and cards; a family border (hue at 20–40%) only on the family's own chips and active elements. |
| Radius | The shared table in [Shapes](#shapes); `rounded-xl` only for the neon-glass cards and the Player artwork; `rounded-2xl` and `rounded-3xl` nowhere. |
| Elevation and glass | `shadow-sm`, `shadow-lg`, `shadow-xl` as in [Elevation](#elevation-and-depth). Glass only in Deck and Pulse as defined above; glow only on the Deck's four live elements; halos only on the Pulse KPI cards. No `shadow-2xl`, no coloured shadow on buttons or cards, no decorative gradient (functional gradients stay: slider fill, image scrim for legibility). |
| Typography | Same scale and weights (400, 500, 600; 700 only in key badges); `text-xs` minimum; one display size (`text-2xl font-semibold tabular-nums`) for KPI values and hero titles; no `font-black` or `font-extrabold`. |
| Spacing and density | Rows `px-3 py-1.5`, `gap-2` between controls, cards `p-4` (glass cards `p-5`); a family never loosens the library's density. |
| Hover | A surface step (`hover:bg-surface-2`) or a tint step; no translate or scale on hover. |
| Press | Colour change; `active:scale-95` only on the Deck's round transport buttons. |
| Focus | The accent outline (`*:focus-visible`) in every family; a family border colour never replaces it (`focus:border-emerald-500 focus:outline-none` is not a focus style). |
| Selected | The family's active colour in its scope (Deck: `cyan-500/10` row; Beatport: `emerald-500/20` item; elsewhere `bg-brand-muted`). |
| Now playing | Library and bottom bar: accent; Player: Deck cyan; Beatport: neon green. |
| Disabled | `opacity-50 cursor-not-allowed`, same in every family. |
| Motion | 150–200 ms on listed properties; infinite animations only for spinners, the now-playing EQ bars and live dots (Spotify playing, Mixed In Key detected), each with `motion-reduce:animate-none` (or a `prefers-reduced-motion` rule for keyframes in `<style>`). |
| Contrast | 4.5:1 for text, 3:1 for interface elements (icons, pins, outlines that carry meaning), in both themes, for every family. |

### Measured contrast

WCAG 2.x ratios computed from the token and palette values in the code (2026-09-30), not measured on screen. Surfaces: `surface-1` (#18181b dark, #fafafa light); the Pulse glass card is computed as `surface-1` at 70% over `surface-0` (#131316 dark, #fcfcfc light). ✗ marks a pair under its target where it is used today.

| Family | Pair | Dark | Light |
| --- | --- | --- | --- |
| Shared | `text-primary` on `surface-1` | 16.97 | 16.97 |
| Shared | `text-secondary` on `surface-1` | 6.91 | 7.41 |
| Shared | `text-tertiary` on `surface-1` / on `surface-2` | 3.67 ✗ / 3.08 ✗ | 4.63 / 4.40 ✗ |
| Shared | `stroke-strong` on `surface-1` (UI) | 2.29 | 2.46 |
| Accent | accent as text on `surface-1` (blue; range of the ten) | 4.82 (3.97–8.25) | 3.52 ✗ (2.06–4.28) |
| Accent | white on accent fill, `Button primary` (blue; range) | 3.68 ✗ (2.15–4.47) | same |
| Accent | black on accent fill (blue; range) | 5.71 (4.70–9.78) | same |
| Accent | accent focus outline on `surface-1` (UI; range) | 3.97–8.25 | 2.06 ✗ (amber) – 4.28 |
| Accent | `text-primary` on `brand-muted` row (blue) | 13.20 | 13.54 |
| Deck | live text: `cyan-400` dark, `cyan-600` light today | 9.80 | 3.53 ✗ (`cyan-700`: 5.13) |
| Deck | black on `cyan-400` (play button) | 11.62 | 11.62 |
| Deck | `cyan-500` fill on `surface-1` (UI) | 7.30 | 2.33 ✗ |
| Deck | `amber-400` cue pin on `surface-2` (UI) | 8.92 | 1.52 ✗ |
| Deck | cue badge: `amber-300` dark, `amber-600` light | 12.29 | 3.05 ✗ (`amber-700`: 4.81) |
| Deck | cue pad label on 15% amber: `amber-400` dark, `amber-500` light | 8.12 | 1.84 ✗ |
| Pulse | `emerald-400` / `purple-400` / `pink-400` text on glass | 9.65 / 7.02 / 7.00 | 1.87 ✗ / 2.58 ✗ / 2.58 ✗ |
| Pulse | `amber-400` / `cyan-400` text on glass | 11.11 / 10.26 | 1.63 ✗ / 1.76 ✗ |
| Pulse | ranks gold / silver / bronze on glass | 12.86 / 12.49 / 5.82 | 1.41 ✗ / 1.45 ✗ / 3.11 ✗ |
| Pulse | light targets `emerald-700` / `purple-700` / `pink-700` / `amber-700` / `cyan-700` | — | 5.25 / 6.69 / 5.78 / 4.81 / 5.13 |
| Pulse | `text-tertiary` on glass | 3.84 ✗ | 4.71 |
| Sources | Spotify / Beatport / Mixed In Key as text | 6.85 / 13.28 / 9.84 | 2.48 ✗ / 1.28 ✗ / 1.73 ✗ |
| Sources | Rekordbox / Crate local as text | 4.71 / 4.18 ✗ | 3.61 ✗ / 4.06 ✗ |
| Sources | black on Spotify fill | 8.12 | 8.12 |
| Beatport | black on `#00FF96` | 15.74 | 15.74 |
| Beatport | `#00FF96` text | 13.28 | 1.28 ✗ |
| Beatport | `emerald-400` on `emerald-500/20` (active item) | 6.66 | 1.53 ✗ (`emerald-700` text: 5.25) |
| States | `danger` / `warning` / `success` / `info` as text | 4.71 / 8.25 / 7.78 / 4.82 | 3.61 ✗ / 2.06 ✗ / 2.18 ✗ / 3.52 ✗ |
| States | white on `danger` fill | 3.76 ✗ | 3.76 ✗ |

What the table says: **the dark theme holds for every family**, except `text-tertiary` (3.67:1) and white text on accent and danger fills; **the light theme fails for every family hue used as text**. The table records the palette values as they were before [D3]; the family tokens now carry a light text value for every role (`-700` shades, `-800` where the text sits on its own tint, `green-800` for Spotify), and `apps/desktop/src/theme.test.ts` checks every `-text` token at 4.5:1 or more on `surface-0`, `surface-1` and `surface-2` in both themes, and `--brand-on` on every accent.

The last [D3] pairs, measured on the harness in the light theme (2026-10-08) with a canvas that composites every layer, before and after; the dark values did not change:

| Element | Light before | Light after |
| --- | --- | --- |
| Energy badge text on its tint, ten levels (library, Player hero) | 1.34 ✗ to 2.88 ✗ | 5.74 to 7.34 (worst case with a selected row: 4.64) |
| Mixed In Key "IN KEY" label and sync icon on the toolbar button | 2.48 ✗ / 2.00 ✗ (hover 1.12 ✗) | 5.37 (hover 5.40) |
| White on the Duplicate Killer / Upgrader count badges | 3.81 ✗ / 2.47 ✗ | 6.42 / 5.36 |
| Danger menu items, Beatport favourite heart (active) | 3.65 ✗ | 6.15 |
| Beatport sidebar favourites heart (UI) | 2.39 ✗ to 2.89 ✗ | 5.32 to 6.42 |

### How a new view stays inside the charter

1. Decide which family the view belongs to. By default it is **Accent**; joining Deck, Pulse or Beatport means its purpose matches the family's purpose. A new family is an owner decision.
2. Use only the roles of that family's table, with their dark **and** light values; states, data palettes and source colours keep their own rules.
3. Keep focus, Crate's own controls and every common component on the accent.
4. Surfaces, radius, type, spacing and motion come from the shared rules; glass, glow and halos only where the family allows them.
5. Check the contrast of every new foreground and background pair against the table above (4.5:1 text, 3:1 UI), in both themes.
6. Check the combination table: no colour of another family, no family colour in the shell beyond a label or a count badge.
7. Run `yarn design:scan <files>`: a family role is written with its token (`bg-deck-live`, `text-pulse-plays-text`, `bg-beatport`); a palette class or hex value is debt, never a new role.

## Typography

- **Always via `<Text variant=…>`** for standalone text. For text inside a control or a cell, the Tailwind scale classes (`text-xs`, `text-sm`, `text-base`, `text-lg`) are enough.
- **No arbitrary size** (`text-[10px]`, `text-[11px]`): 12 px (`text-xs`) is the minimum for a piece of data. The current arbitrary sizes (about 190) are debt.
- **Weights**: 400 and 500 for text, 600 for headings, 700 only inside key badges. `font-black` and `font-extrabold` are outside the system.
- **Display size**: `text-2xl font-semibold tabular-nums` is the one size above `text-lg`, reserved for the Pulse KPI values and the title of the Player hero, the album detail and the Beatport artist header.
- **Numbers**: `tabular-nums` on columns and comparisons (BPM, durations, counters, statistics); `font-mono` for keys, BPM and codes.
- **Font**: chosen by the user (`[data-font]`), Open Sans by default. A component never forces a family.
- **Uppercase**: reserved for table headers and section labels (`header-3`, `header-4`, `header-table`), sparingly. No all-caps label above every block.

## Layout

- **App structure**: toolbar at the top, resizable left sidebar, main area, optional right panel. Do not reinvent this structure inside a view.
- **Minimum window 1000×600.** No content area with a fixed pixel height; lists are `flex-1 min-h-0` with internal scrolling; flex children that contain text have `min-w-0` so that `truncate` works.
- **Tables**: CSS Grid with `minmax(0, …)` and a minimum width on the title column; secondary columns are hidden before the title shrinks.
- **Grids rather than calculations**: `grid` and `gap-*`, never `w-[calc(33%-1rem)]`.
- **Cards only when elevation means something.** Otherwise group by spacing or with `border-t` / `divide-y`. No card inside a card inside a card.

## Elevation and depth

Flat by default. `shadow-sm` for what barely protrudes, `shadow-lg` for what floats (menus, tooltips, toasts), `shadow-xl` for modals. Depth comes from the surface scale.

The one exception is the **neon-glass material**, kept by owner decision and confined to two views: the Player (translucent hero over the blurred artwork, glow on its four live elements) and Pulse (glass cards, one static halo per KPI card). Its exact recipe and limits are in the [Graphic charter](#b-deck--the-players-neon-glass). Everywhere else: no `backdrop-blur`, glow, coloured shadow, decorative gradient or blurred halo; and nowhere `shadow-2xl`.

## Shapes

| Element | Radius |
| --- | --- |
| Buttons, fields, select, menu items | `rounded-md` |
| Data badges, checkbox, tooltip, `kbd` | `rounded` |
| Cards, modals, dropdown menus, panels | `rounded-lg` |
| Pills, switches, sliders, avatars | `rounded-full` |
| Neon-glass cards (Pulse), Player artwork and hero panels | `rounded-xl` |

`rounded-xl` anywhere else, `rounded-2xl` and `rounded-3xl` are outside the system (debt from builds 40 to 57).

## Components

The components in `lib/components/common/` are **mandatory**: `Button`, `IconButton`, `Text`, `Input`, `Select`, `Checkbox`, `ToggleSwitch`, `Slider`, `Modal`, `ConfirmModal`, `InputModal`, `ContextMenu`, `Tooltip`, `Spinner`, `Toast`, `Icon`, `SegmentedControl`, `KeyBadge`, `EnergyBadge`. A need that is not covered is handled by **extending** the common component (new variant, new prop), not by copying it.

A common component is on the accent by default. When a family view needs its own look on a shared control, the component takes an explicit prop (`Button tone="deck" | "beatport" | "spotify" | "rekordbox"`, `SegmentedControl variant`, `KeyBadge variant`, `Checkbox appearance`, `Spinner icon`, `Modal panelClass`) that reproduces that view's look exactly: the owner keeps today's look (CRA-141), so extraction shares the code and the behaviour, never imposes a new look. A view uses only its own family's tone, and the tones are written with the family tokens.

- **Button**: `primary` for the primary action of a view or modal (only one, text `text-brand-on`), `secondary` by default, `ghost` in toolbars, `danger` for a confirmed destructive action. With a family `tone`, `size="bare"` plus `class` give the padding, and `shape` (`lg`, `xl`, `full`), `weight`, `display`, `glow`, `press`, `lift` reproduce the view's radius, weight, coloured shadow, press and hover scale; `shape` on the accent tone gives the same family layout in Crate's colour (the Player's "Add to library"). `fill` replaces the primary fill when a primary action is not on the accent (red confirmation, the Upgrader's replace buttons): a fill passed through `class` loses to the accent fill in the generated CSS.
- **Checkbox**: always `Checkbox`. `appearance="native"` keeps the system checkbox where a view had one (confirmation dialogs, Relocate, Duplicate Killer, the Upgrader), with its label as `children`.
- **Spinner**: every busy indicator is `Spinner` (no hand-written `animate-spin`): `refresh`, `refresh-cw` and `loader` turn an icon of the set, `arc` is the cloud sync circle, `ring` a CSS ring sized and coloured by `class`. `IconButton busy` swaps its icon for the spinner.
- **Segmented control**: always `SegmentedControl` (view switcher, Player display mode, Pulse period). It is a radio group: one tab stop, arrow keys move the selection, a translated `ariaLabel` names the group. `switcher` for the view switcher (a segment label may carry the destination family's colour, `labelTone`), `deck` for the Player, `boxed` for Pulse's period selector. A control whose options may outgrow their row (Pulse's period presets) takes `scrollable` and follows the scroll affordance below; the parent lets it shrink (`min-w-0`, `flex-1` with a zero basis) so the row never wraps.
- **Scroll affordance** (a row of options wider than its space): the row stays on one line and scrolls horizontally; it never wraps and never makes the page scroll sideways. An arrow (`IconButton` `sm`, `chevron-left` / `chevron-right`, translated name, `tabindex="-1"` because the arrow keys already move through the options) appears only on a side that hides content, and pages by one view. Under it, the content fades out through an alpha mask (`.scroll-affordance` in `style.css`: transparent under the arrow, then a 1.25rem ramp), so the fade needs no colour and reads the same in both themes; the scrollbar is hidden. The selected option is always scrolled fully into view, clear of the arrows, including when the selection changes from the keyboard or from outside. Scrolling is smooth, and instant under `prefers-reduced-motion`. Without overflow the control looks exactly like its plain version. Today in `SegmentedControl scrollable`; another horizontal list reuses the class and the helpers of `segmented.ts` rather than inventing its own.
- **Icon button**: always `IconButton` with an accessible name (translated `title` and `aria-label`). Active state: `bg-brand-muted text-brand-primary`.
- **Modal**: the common `Modal` (`<dialog>`, Escape, focus trap). Height bounded by the window, internal scrolling, fixed footer. A destructive action goes through `ConfirmModal`. A family panel (the Spotify connection) keeps its look through `size="none"`, `panelClass`, `backdropClass` and `theme="dark"`. A closed `<dialog>` still counts as a child for `space-y-*`: place a modal where it does not become the last child of a spaced stack.
- **Track row**: dense grid, `border-b border-stroke-subtle`, selection as `bg-brand-muted`, the playing track signalled by its title in `text-brand-primary` (in the Player and Beatport views, by the family's now-playing role).
- **Key badge**: always `KeyBadge`, colours from `getCamelotColor()` through `keyBadge.ts`, with the `variant` of its view (library `cell` with `analysis="mik" | "other"`, Beatport `cell-compact`, Duplicate Killer `chip`, Upgrader `chip-plain`, Pulse `tag` and `tag-wide`, Player `pill` and `pill-xs`). The variants keep today's sizes (9 to 12 px); bringing them to the 12 px minimum is suspended with the charter (CRA-141). A Vitest guard fails when a component calls `getCamelotColor()` or `getEnergyInfo()` outside the badges without a stated reason.
- **Energy badge**: always `EnergyBadge`, colours from `getEnergyInfo()`: `badge` in the library, `pill` in the Player hero; the text takes the level's `lightColor` in the light theme.

## Interface states

Every view and every component that loads data handles **four states**: loading (a `bg-surface-3` skeleton shaped like the final content; a spinner only for a short action), empty (a useful message + an action to fill it), error (a message that says what to do, visible, never a silent failure), and nominal. Interactions have hover, pressed, visible focus (`:focus-visible`, accent outline defined globally) and disabled states.

## Accessibility

- Every clickable element is a `<button>` (or an `<a>` for navigation) with an accessible name. No clickable `div` and no `svelte-ignore a11y`.
- Visible focus everywhere; never `outline-none` without a replacement.
- Contrast: 4.5:1 for text, 3:1 for interface elements, **in both themes**.
- Everything that can be done with the mouse can be done with the keyboard (waveform, heatmap, drag and drop: keyboard alternative or menu).
- `prefers-reduced-motion` is respected: every infinite animation carries `motion-reduce:animate-none`.

## Text and language

- **Zero hard-coded strings**: every visible text goes through `{$translate('section.key')}` (the `translate` store from `$shared/i18n`), with the key added to `shared/i18n/locales/en.json` **and** `fr.json` in the same change.
- French addresses the user formally (*vous*), like all of `fr.json`.
- French: ellipsis `…` (not `...`), « » quotation marks, non-breaking space before `: ; ! ?` and inside numbers (« 2 310 »), numbers and dates via `Intl.NumberFormat` / `Intl.DateTimeFormat`.
- Tone: direct, concrete, active voice. Precise button labels ("Delete duplicates", « Supprimer les doublons », rather than "Continue"). Error messages that say how to get out of the situation. No exclamation marks, no "Oops" (« Oups »).

## Do's and don'ts

### Do

- Start from a common component and the tokens; check the rendering in light and dark, with two accents, at 1000×600.
- Let the user's accent carry the primary action, the selection and the focus; inside a family view, let the family carry only the roles the charter gives it.
- Use `tabular-nums` and `font-mono` for music data.
- Prefer spacing and thin separators to cards.

### Don't

- `dark:` for new code: per-theme values belong in tokens (since D1 the variant follows `[data-theme]`, so the existing usages are correct but are debt), palette classes (`bg-zinc-800`) and hex values in a component, except the family roles listed in the charter until their tokens exist.
- `transition-all`, decorative infinite animations, `hover:-translate-y`, `group-hover:scale-110` on cards.
- Frosted glass, `backdrop-blur`, halos and glow outside the neon-glass scope; decorative gradients and `shadow-2xl` anywhere.
- A family colour outside its scope (Deck cyan in the toolbar, Beatport green in the Library, a Pulse hue in a modal), two families in one element, or a colour "belonging" to a view that has no family.
- Emojis in the interface, hand-drawn icons outside `Icon.svelte`.
- Fixed pixel heights on content areas, `h-screen`.
- `z-[9999]`: layers are `z-10` (sticky content), `z-20`/`z-30` (panels), `z-40` (overlays), `z-50` (menus, tooltips).

## Responsive behaviour

Crate is a desktop app: no mobile breakpoints in the desktop app (the mobile app is a separate project, `apps/mobile`). The sizes to check are window sizes:

| Size | What must hold |
| --- | --- |
| 1000×600 (minimum) | Toolbar without overlap, readable title column, modals fully visible, Player transport visible |
| 1400×900 (default) | Reference rendering |
| 1920×1080 | No large empty areas, wide content bounded |

## Iteration guide

1. Read this file, then the common component closest to the need.
2. Work on one component at a time, and refer to it by its component or token name.
3. Any new semantic colour becomes a token declared for both themes in `style.css` (and in `app.html` if it is used by the startup screen), with contrast checked; three at most outside the families. A family role gets its token in the "Colour families" section of `style.css`, registered in its `@theme inline` block, with a `-text` value that `theme.test.ts` checks; a new family or a new metric hue is an owner decision.
4. Run `yarn design:scan <path>` on the files touched: no new occurrence may appear.
5. Check visually (`crate-visual-check` skill) before declaring the work done.
6. Update this document if a rule changes.

## Known gaps

- **State colours not adapted to the light theme**: as text on `surface-1`, `warning` gives 2.06:1, `success` 2.18:1, `danger` 3.61:1, `info` 3.52:1. Only the Duplicate Killer toolbar alert has `--warning-text` so far; the other state texts still need per-theme `-text` variants (linked to D3).
- **Count badges of the toolbar, dark theme**: the Duplicate Killer (`--danger-badge`) and Upgrader (`--beatport-badge`) count badges keep white text on `red-500` and `emerald-500` in the dark theme, 3.8 and 2.5:1, because the dark look is frozen (CRA-141). The light theme darkens the fill instead (`red-700` 6.42:1, `emerald-700` 5.36:1, D3).
- **Upstream red (danger and like)**: on the `--danger-*` tokens since D3 (2026-10-08), so the light theme reads `red-700`. Two places keep palette classes on purpose: the remove button of a Player album card (`hover:text-red-400` on a `bg-black/60` overlay, dark in both themes) and the error toast (`bg-red-600/40` under `text-primary`, like the green, amber and blue toasts). In the dark theme `--danger-text-muted` stays the upstream `red-500`, 3.90:1 on `surface-2` (frozen dark look, allowed by `theme.test.ts`).
- **The e2e audit does not read `oklch()` colours**: `ui-audit.js` parses `rgb()` and `rgba()` only, and counts the rest as unmeasured, so its `lowContrast` count misses every Tailwind palette colour (Tailwind 4 writes them in `oklch`). Family tokens resolve to the same `oklch` values; contrast of family text is checked by `theme.test.ts` and by a canvas-based measurement, not by the ratchet.
- **`text-tertiary` in the dark theme**: 3.67:1 on `surface-1` and 3.08:1 on `surface-2`, under 4.5:1 for the labels and empty values it carries; a lighter value (for example `#8a8a93`: 5.18 and 4.35:1) would need checking against `text-secondary` so the two stay distinct.
- **Debt measured** by `yarn design:scan apps/desktop/src` (see the register, defects D3, D7, D10, D11; `yarn design:scan` gives today's counts). After the family tokens (2026-10-01) the palette classes and hex values left are debt, not family roles.
- **Other documents still describe the former direction** ("bring every view back to the accent"): the `crate-design-system`, `crate-ui-build` and `crate-ui-audit` skills and `anti-slop-product.md` ("one colour per card"). This file is authoritative until they are updated.
- **Internal icon set**: `Icon.svelte` contains the hand-drawn paths inherited from upstream; it is the only authorised source, and missing icons are added to it rather than introducing a library.
- **Visual harness**: `yarn harness` (port 1430, fake Tauri backend, `apps/desktop/harness/README.md`) and `yarn test:e2e` measure every view in light and dark, at 1000×600 and 1400×900, in English and French; the counts are a ratchet in `e2e/baseline.json`. Contrast in this document is still computed from token values, the harness measures what is actually rendered.

### Known deviations from the charter

Where the code breaks the charter today; each one is tracked in the register and fixed view by view, not here.

| Deviation | Where | Register |
| --- | --- | --- |
| `text-white` and `bg-black/40–70` inside the Spotify connection modal (a panel that is dark in both themes) and on the Upgrader's artwork overlay; the harmonic wheel's `#3b82f6` fallback for a key without a colour (legible in both themes: token debt, not a contrast defect) | Pulse, Upgrader | D11 |
| Glass, glow, halos, gradients and `rounded-2xl`/`3xl` beyond the recipe (glow on Beatport buttons, `rounded-2xl`/`3xl` on Beatport and Spotify panels, glass tooltip inside a glass card) | Pulse, Beatport, Upgrader, Duplicate Killer | D11 |
| Family colours outside their scope: sky switch in `ToggleSwitch` (also 2.5 to 2.7:1 as a fill on the light surfaces, below 3:1), purple harmonic-match chip in the Player | common, Player | D11, D13 |
| Family borders replacing the accent focus outline; infinite animations without reduced motion | Beatport, Player, Pulse | D10 |
| Beatport table columns without `minmax(0, …)` | Beatport | D7 |
