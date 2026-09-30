# "AI-generated" interface signatures — dense desktop app version

Source: [Leonxlnx/taste-skill](https://github.com/Leonxlnx/taste-skill) (skills `taste-skill` v2, `redesign-skill`, `minimalist-skill`, `image-to-code-skill`), MIT licence, © 2026 Leonxlnx. Taste-skill targets landing pages and itself declares dashboards and dense interfaces out of its scope; this document keeps only what applies to an app like Crate, and translates it into Crate's rules.

These patterns are the typical traces of an interface produced by an assistant without art direction. Crate's builds 40 to 57 contain many of them (Player, Pulse, Beatport, Upgrader, Duplicate Killer). An audit flags them; a redesign removes them.

## Surfaces and colour

- **Frosted glass**: `backdrop-blur-*` + semi-transparent background (`bg-surface-1/70`). In Crate: an opaque surface from the scale.
- **Decorative halos and blurred blobs**: `absolute … rounded-full bg-emerald-500/10 blur-2xl`. To be removed.
- **Glow**: `shadow-[0_0_8px_rgba(…)]`, `drop-shadow-[0_0_3px_…]`, `shadow-emerald-500/10`. To be removed.
- **One colour per card** (emerald, violet, amber, cyan for four KPIs). A single accent colour, the user's; the distinction comes from the icon and the label.
- **Gradients** on text, buttons or backgrounds (`bg-gradient-to-r from-cyan-400 to-sky-500`). To be removed (except an official brand logo).
- **Pure black `#000` and pure white on a dark background**, hard-coded: use the surface and text scale.
- **Mixed warm and cool greys** (`stone` + `zinc`): a single family, the tokens'.
- **Oversaturated accent** and default "AI purple": the accent is the one chosen by the user, never imposed by a view.

## Typography

- **`font-black` / `font-extrabold` on figures** "to look premium": `font-semibold` is enough, hierarchy comes from size and colour.
- **Shouting headings** (`text-3xl`+ in a tool view): Crate's scale stops at `text-lg` for view headings; a key figure can go up to `text-2xl`.
- **Arbitrary micro-text** (`text-[10px]`, `text-[9px]`): unreadable and off the scale; `text-xs` minimum.
- **Letter-spaced capital labels above every block**: one per area at most, through `Text variant="header-3|header-4"`.
- **Proportional figures** in columns: `tabular-nums`.
- **ASCII ellipsis** "..." and straight quotes: `…`, « », “ ”.

## Layout

- **Card inside a card inside a card**: a single level of framing; the rest through spacing and separators.
- **All data in cards**: at high density, figures breathe in a simple layout separated by 1 px rules.
- **Mixed radii** (`rounded-2xl` for cards, `rounded-md` for buttons, `rounded-xl` for icons): a single system (see DESIGN.md).
- **Fixed heights** (`h-[225px]`) that cut content off at 1000×600, or leave large gaps at 1920×1080.
- **Width calculations** (`w-[calc(33.333%-2px)]`) where a grid is enough.
- **`z-[9999]`** and random z-indexes.
- **Elements stuck to the edge or misaligned**: align shared elements (headings, values, actions) across neighbouring columns; 1 to 2 px optical correction for icons next to text.

## Motion

- **Cards that lift on hover** (`hover:-translate-y-0.5`, `group-hover:scale-110`): not in a tool; colour hover.
- **`transition-all duration-300`** everywhere: slow and costly; 150 to 200 ms on listed properties.
- **Decorative infinite animations** (`animate-pulse` on a logo, a badge, a "live" dot): reserved for loading, with `motion-reduce`.
- **Animation for no reason**: every animation must signal a state (action feedback, state change, appearance). Otherwise, remove it.

## Content

- **Hard-coded strings** (often in French in code that is supposed to be English: « Temps d'Écoute »): i18n key.
- **Falsely precise or invented figures** in mockups or empty states: real data or a clear sample label.
- **Copywriting clichés**: "Discover", "Boost", "Seamless", "Next-gen", exclamation marks, "Oops!" (« Découvrez », « Boostez », « Oups ! » in French). Simple, precise text.
- **"PRO" labels, version badges, decorative coloured dots** in front of navigation items: to be removed (a coloured dot is only allowed for a real state, such as "connected").
- **Emojis** in the interface: replace with an icon from `Icon.svelte`.
- **Title Case on everything in French**: sentence case in French.

## Forgotten states

- **Only the nominal state is drawn**: loading (skeleton shaped like the content), empty (message + action), error (message + how to recover) are missing.
- **No active state in the navigation**: the current item must be visible (`bg-brand-muted text-brand-primary`).
- **No feedback on press** and no visible focus.
- **Dead actions**: buttons with no effect or leading nowhere; hide them or disable them with an explanation.

## Code

- **`div` soup**: semantic elements (`button`, `nav`, `header`, `main`, `section`, `table`).
- **Inline styles mixed with classes** (except data colours applied through `style=`).
- **Hallucinated imports**: check `apps/desktop/package.json` before importing; do not add an icon, animation or component library.
- **Commented-out dead code** left behind.

## Source licence

```
MIT License

Copyright (c) 2026 Leonxlnx

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
