---
name: crate-design-system
description: Crate's design rules (tokens, common components, [data-theme] theme, user accent, i18n, accessibility, DJ-tool density). Load before any change to a Svelte component, a view or style.css, and before giving any opinion on the app's appearance.
when_to_use: Any task that creates, modifies, reviews or judges Crate's desktop interface (apps/desktop/src).
paths:
  - apps/desktop/src/**
  - DESIGN.md
user-invocable: false
---

# Crate design system

The full reference is [DESIGN.md](../../../DESIGN.md) at the root of the repository. **Read it before writing a single line of interface.** This skill is its operational digest.

## Reading the context

Crate is a DJ's working tool (library, player, listening statistics, USB export), not a marketing page. Reference dials (taste-skill vocabulary):

| Dial | Value | Consequence |
| --- | --- | --- |
| `DESIGN_VARIANCE` | 2 | Symmetry, predictable alignments, app structure unchanged |
| `MOTION_INTENSITY` | 2 | Hover, press and state transitions of 150 to 200 ms; nothing else moves |
| `VISUAL_DENSITY` | 8 | Tight tables, 1 px separators rather than cards, `tabular-nums` on all figures |

All "landing page" reflexes (hero, bento, halos, frosted glass, giant typography, glowing KPI cards, marquee) are **out of place** in Crate.

## The twelve strict rules

1. **Colours through tokens only**: `bg-surface-0…4`, `text-text-primary|secondary|tertiary|disabled`, `border-stroke|stroke-subtle|stroke-strong`, `brand-primary|hover|muted`, `danger|warning|success|info`. No palette class (`bg-zinc-800`, `text-emerald-400`), no hex in a component.
2. **New semantic colour = new token** declared for both themes in `style.css`, contrast checked; three at most (cue, live, brand).
3. **Never `dark:`**: the theme is `[data-theme]`, chosen in the app, not by the OS.
4. **Typography through `Text`** or the Tailwind scale (`text-xs` to `text-lg`); never `text-[Npx]`; 12 px minimum for a piece of data; weights 400/500/600.
5. **Radii**: `rounded-md` controls, `rounded` badges, `rounded-lg` cards/modals/menus, `rounded-full` pills. Never `rounded-xl` or above.
6. **Elevation**: `shadow-sm`, `shadow-lg`, `shadow-xl` only. No gradient, glow, `backdrop-blur`, blurred halo, semi-transparent surface.
7. **Common components are mandatory** (`lib/components/common/`): `Button`, `IconButton`, `Text`, `Input`, `Select`, `Checkbox`, `ToggleSwitch`, `Slider`, `Modal`, `ConfirmModal`, `InputModal`, `ContextMenu`, `Tooltip`, `Spinner`, `Toast`, `Icon`. Extend a common component, do not copy it.
8. **Every clickable element is a `<button>`** (or `<a>`) with a translated accessible name; no `svelte-ignore a11y`.
9. **Zero hard-coded strings**: `{$translate('section.key')}` (`import { translate } from '$shared/i18n'`), key added to `shared/i18n/locales/en.json` and `fr.json` in the same change. French uses the formal « vous ».
10. **No fixed pixel height** on a content area; every view and modal usable at 1000×600 (`flex-1 min-h-0`, `min-w-0`, `minmax(0, …)`).
11. **Motion**: 150 to 200 ms on colour, opacity or transform, listed properties (never `transition-all`); infinite animations reserved for spinners, with `motion-reduce:animate-none`.
12. **Definition of done**: `yarn design:scan <files>` with no new occurrence, visual check in light + dark, two accents, 1000×600 and 1400×900 (skill `crate-visual-check`), `yarn check:svelte` and `yarn test` green.

## Conversion table (debt → system)

| Found in the code | Replace with |
| --- | --- |
| `bg-black`, `bg-zinc-950` | `bg-surface-0` |
| `bg-zinc-900`, `bg-surface-1/70` + `backdrop-blur-*` | `bg-surface-1` (opaque) |
| `bg-zinc-800`, `bg-white/5` | `bg-surface-2` |
| `bg-zinc-700`, `bg-white/10` skeleton | `bg-surface-3` |
| `text-white` on a neutral background, `text-zinc-100` | `text-text-primary` (`text-white` remains allowed on a `brand-primary` or `danger` background) |
| `text-zinc-400`, `text-gray-400` | `text-text-secondary` |
| `text-zinc-500`, `text-zinc-600` | `text-text-tertiary` |
| `border-zinc-700`, `border-white/10` | `border-stroke` |
| `border-zinc-800`, `border-white/5` | `border-stroke-subtle` |
| `text-red-500`, `bg-red-600` | `text-danger`, `bg-danger` (or `Button variant="danger"`) |
| `emerald`, `green` for a success | `success` |
| Decorative `sky`, `cyan`, `purple`, `emerald` (Player, Pulse, Beatport, switches) | `brand-primary`, `brand-muted`, `bg-brand-primary/10` |
| `text-[10px]`, `text-[11px]` | `text-xs` |
| `rounded-xl`, `rounded-2xl` | `rounded-lg` |
| `transition-all` | `transition-colors`, `transition-opacity`, `transition-transform` or `transition-[prop,prop]` |
| `shadow-2xl`, `shadow-emerald-500/10`, `drop-shadow-[…]` | `shadow-lg` / `shadow-xl`, or nothing |
| `font-black`, `font-extrabold` | `font-semibold` |
| `hover:-translate-y-0.5`, `group-hover:scale-110` on cards | colour hover (`hover:bg-surface-2`) |
| `z-[9999]` | `z-50` (layers: 10 sticky, 20/30 panels, 40 overlays, 50 menus/tooltips) |
| `div` with `onclick` | `<button type="button">` or `IconButton` |
| Copied Camelot/energy colour | `getCamelotColor()` / `getEnergyInfo()` from `shared/utils` |

## Legitimate exceptions

- **Data palettes** (Camelot key, energy): allowed through `shared/utils/camelot.ts` and `energy.ts`, on the badge only.
- **Third-party brand logos** (Mixed In Key, Beatport, Spotify): their official colours stay in the logo component, line annotated `design-scan-ignore` with the reason.
- **`text-white`** on an accent or danger background.

## Direction

The reference visual language is **the upstream one** (sober, zinc, driven by the accent). The views added in builds 40 to 57 (Player, Pulse, Beatport, Upgrader, Duplicate Killer) are to be brought back to this language, not the other way round. Never invent a colour "specific" to a view.

## Known pitfalls of the repository

- `warning` and `success` do not have enough contrast as text on a light background (gap documented in DESIGN.md, defect D3): for status text in the light theme, prefer an icon + `text-text-primary`, or propose a `-text` token.
- `app.html` copies a few colours for the splash screen: any change to `surface-0`, `text-primary` or the accents must be carried over there.
- `Icon.svelte` is the only icon set; a missing icon is added there (the `Icon.test.ts` test fails on an unknown name).
- Tauri IPC and stores are not design's business: never change a command, a settings key name or a store for a visual reason.
