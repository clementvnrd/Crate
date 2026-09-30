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
    brand-hover: "var(--brand-hover)"
    brand-muted: "var(--brand-muted)"       # accent at 20%: background of the active element
  state:                   # identical in both themes (see "Known gaps")
    danger: "#ef4444"
    warning: "#f59e0b"
    success: "#22c55e"
    info: "#3b82f6"
  data:                    # DATA palettes, centralised, never copied into a component
    camelot: "shared/utils/camelot.ts (CAMELOT_COLORS, getCamelotColor) — Mixed In Key 11 wheel"
    energy: "shared/utils/energy.ts (getEnergyInfo) — levels 1 to 10"
  sources:                 # third-party service colours: identify a source, nothing else (see "Graphic charter")
    spotify: "#1DB954"
    beatport: "#00FF96"
    mixed-in-key: "#00D2FF"
    rekordbox: "#EF4444"
    crate-local: "#8B5CF6"
  families:                # scoped colour families (see "Graphic charter"). Values as written in the code today;
                           # they are not tokens yet (proposed names in the charter), light-theme text values are targets
    deck:                  # Player view
      live: "cyan-400 #22d3ee (dark text and fills), cyan-500 #06b6d4 (fills); light text: cyan-700 #0e7490 (target)"
      on-live: "black"
      cue: "amber-400 #fbbf24 (fills), amber-300 #fcd34d (dark text); light text: amber-700 #b45309 (target)"
    pulse:                 # Pulse (statistics) view, one hue per metric; light text uses the -700 shade (target)
      listening-time: "emerald-400 #34d399"
      plays: "purple-400 #c084fc"
      artists: "pink-400 #f472b6"
      dj-sessions: "amber-400 #fbbf24"
      keys: "cyan-400 #22d3ee"
      ranks: "gold amber-300 #fcd34d, silver slate-300 #cbd5e1, bronze amber-600 #d97706"
    beatport:              # Beatport view, Beatport modals, Upgrader
      neon: "#00FF96 (fills with black text), hover #00e687"
      secondary: "emerald-400 #34d399 (dark text); light text: emerald-700 #047857 (target)"

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
  Button: { variants: [primary, secondary, ghost, danger, ghost-danger, outline], sizes: [sm, md, lg], rounded: control }
  IconButton: { sizes: [sm 24px, md 32px, lg 40px], active: "bg-brand-muted text-brand-primary", rule: "accessible name required" }
  Text: { variants: [header-1, header-2, header-3, header-4, header-table, body-1, body-2, caption, code] }
  Input: { background: surface-2, border: stroke, rounded: control }
  Select: { trigger: surface-2, menu: "surface-1 shadow-lg rounded-lg z-50" }
  Modal: { element: "native <dialog>", background: surface-1, border: stroke, rounded: panel, shadow: shadow-xl, sizes: [sm, md, lg, xl, 2xl, 3xl, 4xl], rule: "height bounded by the window, fixed footer" }
  Tooltip: { background: surface-1, border: stroke, rounded: small, text: text-xs, portal: true }
  ContextMenu: { background: surface-1, rounded: control, shadow: shadow-lg }
  Checkbox: {}
  ToggleSwitch: {}
  Slider: {}
  Spinner: {}
  Toast: {}
  Icon: { source: "common/Icon.svelte (internal set); a Vitest test fails on an unknown name" }
  track-row: { layout: "grid, items-center, gap-2, px-3 py-1.5, text-sm", selected: "bg-brand-muted", playing: "title in text-brand-primary" }
  key-badge: { size: "h-[22px] w-11", font: "font-mono text-xs font-bold", colors: "getCamelotColor(key)", rounded: small }
  energy-badge: { component: "library/EnergyBadge.svelte", colors: "getEnergyInfo(level)", rounded: small }
---

# Crate — design system

> **Provisional (CRA-141).** The owner likes today's look and asked to rethink the charter from scratch, because the first version prescribed changes to it. Until he answers, the sections *Graphic charter* and *Known deviations* record intent and measurements, and **nothing in them authorises a visible change** (radius, hover, toolbar colours, Beatport theme, waveform colours, tokens): where they prescribe one, it is suspended.

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
3. The badge text keeps a contrast of at least 4.5:1 against its background (already provided by the `bg`/`text` pairs), in both themes.

### Source colours

The official colours of the services a listen or a track comes from: Spotify `#1DB954`, Beatport `#00FF96`, Mixed In Key `#00D2FF`, Rekordbox `#EF4444`, Crate local `#8B5CF6`. They **identify a source** and nothing else: a logo, a source chip or badge, a segment of the Pulse source bar, and the service's own connect button (Spotify's green button with black text). They are never a selection, hover, focus or state colour. Like data palettes, they should live in one shared map (today they are copied in `StatsSourceBar`, `StatsTopTracks`, `StatsIntegrations`, `TrackInfo`).

## Graphic charter: colour families and their limits

Owner decision (Linear CRA-115, 2026-09-30): Crate keeps several colour families, and each one gets precise limits, so that the whole stays consistent, professional and durable. This section is written **from the visual language that exists today**; where the code breaks it, the code is what changes (see [Known deviations](#known-deviations-from-the-charter)).

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
| Common components (`common/`) | yes | no | no | no | no | yes |

A family colour never sits next to another family's colour in the same element (no cyan and neon green in one row, no gradient between two families). Entry points in the shell may carry the destination family's colour **only as a label or a count badge**, never as the fill or border of the button.

### A. Accent — Crate core

- **Tokens**: `brand-primary`, `brand-hover`, `brand-muted` (20%), `bg-brand-primary/10`, `bg-brand-primary-5`, `bg-brand-primary-10`. Ten accents (`[data-accent]`), blue by default; the value is the same in both themes.
- **Roles**: primary action fill (`Button variant="primary"`), selected row (`bg-brand-muted`), playing track title in the library (`text-brand-primary`), active icon button (`bg-brand-muted text-brand-primary`), progress of the bottom player's seek bar, logo, focus outline (`*:focus-visible`, global).
- **Text on an accent fill** must reach 4.5:1. With today's values white text on the fill reaches it for none of the ten accents (2.06 to 4.47), black text for all ten (4.70 to 9.78). The foreground to use is an open decision (see [Known gaps](#known-gaps)); until it is taken, only `Button` puts text on an accent fill.
- **Accent as text** on a light surface stays under 4.5:1 for every accent (2.06 to 4.28): in the light theme the accent marks with a fill, an outline or an icon, and text next to it stays `text-primary`.

### B. Deck — the Player's neon glass

The only family with a material of its own: a translucent hero over the blurred artwork, and light that glows on what is live.

| Role | Dark | Light | Proposed token |
| --- | --- | --- | --- |
| Live fill: played waveform bars, main play button, active segment, playhead progress | `cyan-400` / `cyan-500` | same fills | `--deck-live` |
| Text on a live fill | black | black | — |
| Live text and icons: time readout, BPM, now-playing row, EQ bars, "Album" label | `cyan-400` (`cyan-300` for BPM) | `cyan-700` (today `cyan-600`, 3.53:1) | `--deck-live-text` |
| Now-playing row background | `cyan-500/10` | `cyan-500/10` | — |
| Cue fill: hot-cue pins, filled pads | `amber-400` | `amber-400` pin with a `stroke-strong` outline (the fill alone is 1.52:1) | `--deck-cue` |
| Cue text: pad labels, cue badges | `amber-300` / `amber-400` | `amber-700` (today `amber-500`/`amber-600`, 1.84 and 3.05:1) | `--deck-cue-text` |

- **Neon-glass material**: `bg-surface-1/60` + `backdrop-blur-xl` on the hero and on the sticky header of the recent tracks list; the ambient layer is the artwork itself, blurred (`blur-[100px]`, opacity 15% light, 30% dark), decorative and `aria-hidden`.
- **Glow**: only on four live elements — played waveform bars, playhead, cue pins, main play button — in the family hue, at most 8 px of blur (`shadow-[0_0_6px_…]`, `shadow-lg shadow-cyan-400/40`). No glow on text, rows, cards or secondary buttons.
- **Two hues only.** Cyan and amber; anything else in the Player uses the accent, the states or the data palettes (Camelot and energy badges).
- **Never**: Deck cyan for the library's now playing, the bottom player bar, the toolbar's export button or a common component; amber for a warning (warnings use `warning`).

### C. Pulse — glass dashboard

| Metric | Hue (dark text, chip, chart) | Light text | Where |
| --- | --- | --- | --- |
| Listening time, activity heatmap | `emerald-400` / `emerald-500` fills | `emerald-700` | KPI card, heatmap, BPM distribution |
| Plays | `purple-400` | `purple-700` | KPI card |
| Artists | `pink-400` | `pink-700` | KPI card, top artists |
| DJ sessions (Rekordbox) | `amber-400` | `amber-700` | KPI card |
| Keys | `cyan-400` | `cyan-700` | harmonic wheel |
| Rank 1, 2, 3 | gold `amber-300`, silver `slate-300`, bronze `amber-600` | `amber-700`, `zinc-600`, `amber-800` | top tracks, top artists |

- **One hue per metric, fixed.** A metric keeps its hue in its KPI card, its chart and its list; a new metric gets a hue only by an owner decision and an entry in this table.
- **Hue as tint**: the chip or icon background is the hue at 15% (`bg-emerald-500/15`), its border at 30%; a card is never fully tinted and never gradient-filled.
- **Glass cards**: the top-level cards of Pulse are `bg-surface-1/70` + `backdrop-blur-xl` + `border-stroke/60` + `shadow-lg` + `rounded-xl`, one level only (no glass inside glass, the heatmap tooltip included). The sticky header is `bg-surface-1/90` + `backdrop-blur-xl`.
- **Halo**: one static blurred orb (`blur-2xl`, hue at 10%) in the corner of each of the four KPI cards; it does not grow or change on hover.
- **Accent in Pulse**: the header icon, the period selector, the refresh button and focus stay on the accent.
- **Source colours** appear in Pulse on source chips, the source bar and the integration cards (border at 25%, icon); the cards' background stays a neutral surface in both themes.

### D. Beatport — neon green

| Role | Dark | Light | Proposed token |
| --- | --- | --- | --- |
| Primary action fill (log in, buy, add to cart, play) | `#00FF96`, hover `#00e687` | same | `--beatport` |
| Text on the fill | black (15.74:1) | black | — |
| Active navigation, secondary text, counters | `emerald-400` on `emerald-500/20` | `emerald-700` on `emerald-500/15` | `--beatport-text` |
| Now-playing row | left border `#00FF96`, background `emerald-500/10` | same | — |

- **Surfaces: open owner decision (CRA-141, question 4).** The Beatport view is hard-coded dark (`#121418`, `#181a20`, `#0e1014`, `#252830`, `#2e323d`) in both themes. Whether it keeps that identity or follows the theme (neutral tokens `surface-0…2` and `stroke`, no visual change in the dark theme) is the owner's choice; until it is taken the surfaces are left as they are, and every family foreground still has to reach its contrast target on the surface it actually sits on.
- **Flat**: no glass, no glow (`shadow-[#00FF96]/20`), no gradient, no hover scale.
- `#00FF96` is never text on a light surface (1.28:1).

### Rules shared by all families

| Topic | Rule for every family |
| --- | --- |
| Surfaces | Neutral tokens only (`surface-0…4`); a family colours chips, fills, text and borders, never a surface. No hard-coded surface hex. Opacity on a surface (`bg-surface-1/70`) exists only in the neon-glass material. |
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

What the table says: **the dark theme holds for every family**, except `text-tertiary` (3.67:1) and white text on accent and danger fills; **the light theme fails for every family hue used as text**, which is why each family has a light text value (the `-700` shades, all measured between 4.81 and 6.69).

### How a new view stays inside the charter

1. Decide which family the view belongs to. By default it is **Accent**; joining Deck, Pulse or Beatport means its purpose matches the family's purpose. A new family is an owner decision.
2. Use only the roles of that family's table, with their dark **and** light values; states, data palettes and source colours keep their own rules.
3. Keep focus, Crate's own controls and every common component on the accent.
4. Surfaces, radius, type, spacing and motion come from the shared rules; glass, glow and halos only where the family allows them.
5. Check the contrast of every new foreground and background pair against the table above (4.5:1 text, 3:1 UI), in both themes.
6. Check the combination table: no colour of another family, no family colour in the shell beyond a label or a count badge.
7. Run `yarn design:scan <files>`: until the family tokens exist, the scan reports family palette classes; a new one is acceptable only if it is a role of the view's family, and never outside its scope.

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

The components in `lib/components/common/` are **mandatory**: `Button`, `IconButton`, `Text`, `Input`, `Select`, `Checkbox`, `ToggleSwitch`, `Slider`, `Modal`, `ConfirmModal`, `InputModal`, `ContextMenu`, `Tooltip`, `Spinner`, `Toast`, `Icon`. A need that is not covered is handled by **extending** the common component (new variant, new prop), not by copying it.

Components to extract (defect D11) and to use as soon as they exist: `SegmentedControl` (4 copies today), `KeyBadge` (Camelot badge copied 8 times), `EnergyBadge` (exists in `library/`, to be generalised).

- **Button**: `primary` for the primary action of a view or modal (only one), `secondary` by default, `ghost` in toolbars, `danger` for a confirmed destructive action.
- **Icon button**: always `IconButton` with an accessible name (translated `title` and `aria-label`). Active state: `bg-brand-muted text-brand-primary`.
- **Modal**: the common `Modal` (`<dialog>`, Escape, focus trap). Height bounded by the window, internal scrolling, fixed footer. A destructive action goes through `ConfirmModal`.
- **Track row**: dense grid, `border-b border-stroke-subtle`, selection as `bg-brand-muted`, the playing track signalled by its title in `text-brand-primary` (in the Player and Beatport views, by the family's now-playing role).
- **Key badge**: `font-mono text-xs font-bold`, `rounded`, colours from `getCamelotColor()`.

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
3. Any new semantic colour becomes a token declared for both themes in `style.css` (and in `app.html` if it is used by the startup screen), with contrast checked; three at most. The family tokens proposed in the charter are an owner decision; a new family or a new metric hue too.
4. Run `yarn design:scan <path>` on the files touched: no new occurrence may appear.
5. Check visually (`crate-visual-check` skill) before declaring the work done.
6. Update this document if a rule changes.

## Known gaps

- **State colours not adapted to the light theme**: as text on `surface-1`, `warning` gives 2.06:1, `success` 2.18:1, `danger` 3.61:1, `info` 3.52:1. For text, per-theme `-text` variants will be needed (linked to D3).
- **Family colours are not tokens yet**: the Deck, Pulse and Beatport roles are written as palette classes and hex values, with light-theme text values that the code does not use yet. Proposed tokens (owner decision, beyond the "three at most" rule): `--deck-live`, `--deck-live-text`, `--deck-cue`, `--deck-cue-text`, `--beatport`, `--beatport-text`, one `--pulse-<metric>` pair per metric, and a shared source-colour map in `shared/utils`.
- **Text on accent fills**: white text on `brand-primary` stays under 4.5:1 for all ten accents (2.06 to 4.47); black text passes for all ten (4.70 to 9.78). Options: black text everywhere, or a per-accent `--brand-on` token. Owner decision.
- **`text-tertiary` in the dark theme**: 3.67:1 on `surface-1` and 3.08:1 on `surface-2`, under 4.5:1 for the labels and empty values it carries; a lighter value (for example `#8a8a93`: 5.18 and 4.35:1) would need checking against `text-secondary` so the two stay distinct.
- **Debt measured** by `yarn design:scan apps/desktop/src` on 2026-09-30 (see the register, defects D3, D7, D10, D11): 279 palette classes, 105 hex values, 188 arbitrary sizes, 89 `xl` to `3xl` radii, 64 blur, glow or coloured shadows, 12 gradients, 89 `transition-all`, 43 `dark:`, 43 infinite animations without `motion-reduce`. Part of the palette and hex counts are now legitimate family roles; the rest is debt.
- **Other documents still describe the former direction** ("bring every view back to the accent"): the `crate-design-system`, `crate-ui-build` and `crate-ui-audit` skills and `anti-slop-product.md` ("one colour per card"). This file is authoritative until they are updated.
- **Internal icon set**: `Icon.svelte` contains the hand-drawn paths inherited from upstream; it is the only authorised source, and missing icons are added to it rather than introducing a library.
- **Visual harness**: `yarn harness` (port 1430, fake Tauri backend, `apps/desktop/harness/README.md`) and `yarn test:e2e` measure every view in light and dark, at 1000×600 and 1400×900, in English and French; the counts are a ratchet in `e2e/baseline.json`. Contrast in this document is still computed from token values, the harness measures what is actually rendered.

### Known deviations from the charter

Where the code breaks the charter today; each one is tracked in the register and fixed view by view, not here.

| Deviation | Where | Register |
| --- | --- | --- |
| Family hues as text in the light theme (`-400` shades, `#00FF96`, source colours), dark gradient tops on the Pulse integration cards (Beatport's always-dark surfaces are an open decision, CRA-141) | Pulse, Beatport, Player, toolbar tool badges, view switcher | D3 |
| Glass, glow, halos, gradients and `rounded-2xl`/`3xl` beyond the recipe (hover lift and scale on KPI cards, glow on Beatport buttons, glass tooltip inside a glass card) | Pulse, Beatport, Upgrader, Duplicate Killer | D3, D11 |
| Family colours outside their scope: sky switch in `ToggleSwitch`, cyan export button and sky Mixed In Key badge in the toolbar, `#00E5FF` instead of the Deck cyan in the waveform previews, purple harmonic-match chip in the Player, rose instead of `danger` in Duplicate Killer | common, toolbar, modals, Player | D11 |
| Family borders replacing the accent focus outline; infinite animations without reduced motion | Beatport, Player, Pulse | D10 |
| Beatport table columns without `minmax(0, …)` | Beatport | D7 |
