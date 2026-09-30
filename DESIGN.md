---
version: 1
name: Crate
description: "Desktop DJ library manager (Tauri + SvelteKit + Tailwind 4). A dense, calm working tool: near-black zinc background (dark theme by default) or white (light theme), a single accent colour chosen by the user, tight track tables where numbers are in `tabular-nums`, and data badges (Camelot key, energy) that are the only real source of colour. The interface steps back behind the music."

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
  numbers: "tabular-nums on every column or comparison of numbers (BPM, duration, counters, stats)"

rounded:
  control: "rounded-md (6px) — buttons, fields, select, menu items"
  small: "rounded (4px) — data badges, checkbox, tooltip, kbd"
  panel: "rounded-lg (8px) — cards, modals, dropdown menus, panels"
  pill: "rounded-full — pills, switches, sliders, avatars"
  forbidden: "rounded-xl, rounded-2xl, rounded-3xl"

elevation:
  flat: "no shadow: table rows, panels sitting on surface-0"
  shadow-sm: "element that protrudes slightly (active segment, badge)"
  shadow-lg: "menus, tooltips, open select, toasts"
  shadow-xl: "modals"
  forbidden: "glow, coloured drop-shadow, backdrop-blur, decorative gradients, blurred halos"

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

This document describes **how Crate must look**. It is read by coding assistants (the `design` agent in `.claude/agents/design.md`) before any interface work, and serves as a reference for the owner. It is extracted from the actual code (`apps/desktop/src/style.css`, `lib/components/common/`) and from the "strict design rules" of the [defect register](tracking/DEFECTS.md#design-responsive-and-accessibility).

## Overview

Crate is a **DJ tool**, not a marketing page. People spend hours in it sorting, listening, tagging and preparing sets: the interface must be quick to read, stable, dense without being stifling, and perfectly consistent from one view to the next.

- **High density, low variance, minimal motion.** In terms of "dials" (borrowed from taste-skill): `VARIANCE 2 · MOTION 2 · DENSITY 8`. Symmetry and predictable alignment, animation only to signal a change of state, a lot of information per screen.
- **Colour is rare and meaningful.** Surfaces are neutral (zinc). Colour comes from three sources only: the accent chosen by the user (selection, focus, primary action, now playing), state colours (error, warning, success), and data palettes (Camelot key, energy).
- **A single visual language.** Upstream had a consistent system; builds 40 to 57 added three parallel languages (cyan/amber in the Player and Pulse, neon green in Beatport, frosted glass). The direction is to **return to the upstream language** and bring the new views back into it, not to invent a fourth style.
- **Two themes at parity.** Dark by default, light complete. Every view must be right in both, with all ten accents.

## Colours

### Surfaces (elevation scale)

From deepest to highest: `surface-0` (window background) → `surface-1` (panels, modals, menus) → `surface-2` (controls, hover) → `surface-3` (hover on an already raised element, loading skeletons) → `surface-4` (rare). Do not skip a level and do not create an intermediate surface through opacity (`bg-surface-1/70` is forbidden: it is a leftover of frosted glass).

### Text

`text-primary` for content, `text-secondary` for metadata (artist, album), `text-tertiary` for labels and empty values ("-"), `text-disabled` for inactive items. Never palette greys (`text-zinc-400`) or hex values.

### Strokes

`stroke-subtle` between table rows, `stroke` for control and panel borders, `stroke-strong` for an outline that must be visible (secondary focus, pronounced separator).

### Accent

The accent is **chosen by the user** (`[data-accent]`). Consequence: a view never picks "its own" colour. `brand-primary` is used for the primary action, focus, and the selected or playing element; `brand-muted` (accent at 20%) is the background of the active element. For other opacities, `bg-brand-primary/10` works (`inline` theme), as do the `bg-brand-primary-5` and `bg-brand-primary-10` utilities. Every view must be checked with at least two accents (the default blue and a warm accent such as orange or amber).

### States

`danger`, `warning`, `success`, `info`: errors, warnings, confirmations. They are not used as decoration (no "emerald" or "violet" KPI card).

### Data palettes

The Camelot key colours (Mixed In Key 11 wheel) and energy colours encode information the DJ reads at a glance: they are legitimate. Rules:

1. They live **only** in `shared/utils/camelot.ts` and `shared/utils/energy.ts`; a component obtains them through `getCamelotColor()` / `getEnergyInfo()` and applies them with `style=`.
2. They colour the **badge**, not its surroundings (no fully tinted row, no glow).
3. The badge text keeps a contrast of at least 4.5:1 against its background (already provided by the `bg`/`text` pairs), in both themes.

## Typography

- **Always via `<Text variant=…>`** for standalone text. For text inside a control or a cell, the Tailwind scale classes (`text-xs`, `text-sm`, `text-base`, `text-lg`) are enough.
- **No arbitrary size** (`text-[10px]`, `text-[11px]`): 12 px (`text-xs`) is the minimum for a piece of data. The current arbitrary sizes (about 190) are debt.
- **Weights**: 400 and 500 for text, 600 for headings. `font-black` and `font-extrabold` are outside the system.
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

Flat by default. `shadow-sm` for what barely protrudes, `shadow-lg` for what floats (menus, tooltips, toasts), `shadow-xl` for modals. No `backdrop-blur`, glow, decorative gradient or blurred background halo. Depth comes from the surface scale.

## Shapes

| Element | Radius |
| --- | --- |
| Buttons, fields, select, menu items | `rounded-md` |
| Data badges, checkbox, tooltip, `kbd` | `rounded` |
| Cards, modals, dropdown menus, panels | `rounded-lg` |
| Pills, switches, sliders, avatars | `rounded-full` |

`rounded-xl`, `rounded-2xl` and `rounded-3xl` are outside the system (debt from builds 40 to 57).

## Components

The components in `lib/components/common/` are **mandatory**: `Button`, `IconButton`, `Text`, `Input`, `Select`, `Checkbox`, `ToggleSwitch`, `Slider`, `Modal`, `ConfirmModal`, `InputModal`, `ContextMenu`, `Tooltip`, `Spinner`, `Toast`, `Icon`. A need that is not covered is handled by **extending** the common component (new variant, new prop), not by copying it.

Components to extract (defect D11) and to use as soon as they exist: `SegmentedControl` (4 copies today), `KeyBadge` (Camelot badge copied 8 times), `EnergyBadge` (exists in `library/`, to be generalised).

- **Button**: `primary` for the primary action of a view or modal (only one), `secondary` by default, `ghost` in toolbars, `danger` for a confirmed destructive action.
- **Icon button**: always `IconButton` with an accessible name (translated `title` and `aria-label`). Active state: `bg-brand-muted text-brand-primary`.
- **Modal**: the common `Modal` (`<dialog>`, Escape, focus trap). Height bounded by the window, internal scrolling, fixed footer. A destructive action goes through `ConfirmModal`.
- **Track row**: dense grid, `border-b border-stroke-subtle`, selection as `bg-brand-muted`, the playing track signalled by its title in `text-brand-primary`.
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
- Let the user's accent carry the primary action, the selection and the focus.
- Use `tabular-nums` and `font-mono` for music data.
- Prefer spacing and thin separators to cards.

### Don't

- `dark:` (the theme is `[data-theme]`, not the OS), palette classes (`bg-zinc-800`, `text-emerald-400`), hex values in a component.
- `transition-all`, decorative infinite animations, `hover:-translate-y`, `group-hover:scale-110` on cards.
- Frosted glass, `backdrop-blur`, halos, glow, gradients, `shadow-2xl`.
- A colour "belonging" to a view (cyan for the Player, neon green for Beatport).
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
3. Any new semantic colour becomes a token declared for both themes in `style.css` (and in `app.html` if it is used by the startup screen), with contrast checked; three at most.
4. Run `yarn design:scan <path>` on the files touched: no new occurrence may appear.
5. Check visually (`crate-visual-check` skill) before declaring the work done.
6. Update this document if a rule changes.

## Known gaps

- **State colours not adapted to the light theme**: `warning` (#f59e0b) on white gives about 2.1:1, `success` about 2.3:1. For text, per-theme `-text` variants will be needed (linked to D3).
- **Debt measured** by `yarn design:scan` (see the register, defects D3, D7, D10, D11): about 580 palette classes and 300 hex values in the new views, 190 arbitrary sizes, 88 `xl` to `3xl` radii, 26 `backdrop-blur`, 89 `transition-all`, 43 leftover `dark:`.
- **Internal icon set**: `Icon.svelte` contains the hand-drawn paths inherited from upstream; it is the only authorised source, and missing icons are added to it rather than introducing a library.
- **No versioned visual harness**: the audit's browser harness (fake Tauri backend) is not in the repository; automatic visual checking depends on it.
