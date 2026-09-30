---
name: crate-ui-build
description: Protocol for creating a Crate component or view, or redesigning an existing view (Player, Pulse, Beatport, Upgrader…) — reading the need, preserve/redesign mode, modernisation levers in order, four states, complete execution without shortcuts, pre-delivery check. Use for any task that writes or rewrites interface code.
argument-hint: "[view or component] [goal]"
---

# Building or redesigning a Crate interface

Protocol adapted from taste-skill (reading the need, redesign protocol, final check), from its `redesign-existing-projects` skill and from `full-output-enforcement` ([Leonxlnx/taste-skill](https://github.com/Leonxlnx/taste-skill), MIT). The substantive rules are in `crate-design-system` and [DESIGN.md](../../../DESIGN.md).

## 1. Read the need before coding

Write one line at the top of the work:

> **Reading:** <view/component> for <concrete DJ use>, mode <creation | redesign-preserve | redesign-structural>, dials <V/M/D> (reference 2/2/8), common components used: <list>.

Example: *"Reading: Pulse KPI cards to read this week's listening time at a glance, mode redesign-preserve, dials 2/2/8, components: Text, Icon, Tooltip."*

If there is a genuine ambiguity that changes the result, ask **a single** question. Otherwise, state the reading and move on.

## 2. Choose the mode

- **Creation**: new component or view. Start from the closest common component and from existing patterns (track row, panel, modal).
- **Redesign-preserve** (default for any existing view): keep the structure, content, labels and shortcuts; bring the appearance back into the system.
- **Redesign-structural**: the layout itself is broken (overflows, fixed heights, nested reinvented components). Restructure, keeping the content and behaviour.

If the mode is not obvious: "Should the current layout of <view> be kept, or rethought?"

## 3. Audit before touching

For a redesign, first do a quick audit (skill `crate-ui-audit`, at minimum `yarn design:scan <files> --details`) and note:

- what works and must stay (signature interactions, order of information, shortcuts);
- what is outside the system (tokens, radii, glass, glow, arbitrary sizes, home-made components);
- the missing states (loading, empty, error);
- the current dial reading: the views from builds 40 to 57 are often at "variance 6 / motion 6 / density 4", far from the target.

## 4. Never change silently

Without the owner's explicit approval:

- navigation labels, view names, keyboard shortcuts;
- IPC commands, stores, settings keys, `localStorage` keys (`crate-theme`, `crate-accent`, `crate-font`, `crate-language`);
- existing i18n keys (add new ones; do not rename without migrating every locale);
- the order or direction of the library columns, the behaviour of the Player's transport;
- the logo and third-party brands.

## 5. Levers, in this order

Stop as soon as the goal is reached (roughly 70% of the value for 40% of the risk with the first three):

1. **Colours → tokens**: palette, hex and `dark:` replaced (conversion table in `crate-design-system`); view-specific colours brought back to the accent.
2. **Typography**: `Text` and the Tailwind scale, no more arbitrary sizes or extreme weights, `tabular-nums`.
3. **Surfaces and shapes**: glass, halos, glow and gradients removed; system radii and shadows; a single level of framing.
4. **Common components**: replace home-made controls (`Button`, `IconButton`, `Select`, `Checkbox`, `Tooltip`, `Spinner`, `Modal`); extract a shared component when a pattern is copied (D11: `SegmentedControl`, `KeyBadge`).
5. **Layout**: intrinsic heights, `flex-1 min-h-0`, `min-w-0`, `minmax(0, …)` grids, holding up at 1000×600.
6. **States**: loading, empty, error, focus, hover, press, disabled.
7. **Motion**: remove the unnecessary, keep 150 to 200 ms on listed properties, `motion-reduce`.
8. **Full replacement of a block**: only if it cannot be salvaged.

## 6. Write

- Svelte 5 (runes `$props`, `$state`, `$derived`, `$effect`), TypeScript, Tailwind 4: follow the style of neighbouring files (tabs, no semicolons, `$lib/…` and `$shared/…` imports).
- Check `apps/desktop/package.json` before any import; **no new dependency** (icons, animation, components) without approval.
- Every visible string: `{$translate('…')}` and the key in `en.json` **and** `fr.json` (the other 13 locales are handled in Step 12 — Translation of the tracker; do not invent translations there).
- New semantic colour: token in both themes of `style.css` (and `app.html` if the splash screen uses it).
- Comments in English in the code, like the rest of the repository; documents, CHANGELOG and commit messages in English too.

### Complete execution

A partial deliverable is a broken deliverable. Forbidden: `// ...`, `// rest unchanged`, `// TODO`, "same principle for the others", a skeleton instead of an implementation, one example followed by a description. Before handing back: recount the requested items (files, components, states, i18n keys) and check that they are all delivered. If the work has to be cut short, stop at a clean boundary (end of a file) and write down exactly what remains.

## 7. Pre-delivery check

Every box must be honestly tickable; otherwise it is not done.

- [ ] Reading line written, mode declared.
- [ ] `yarn design:scan <touched files>`: no new occurrence (compare before/after), remaining ones justified.
- [ ] No palette class, hex, `dark:`, `text-[Npx]`, `rounded-xl+`, `backdrop-blur`, glow, gradient or `transition-all` introduced.
- [ ] A single accent colour (`brand-*`), no view-specific colour; data palettes through `shared/utils`.
- [ ] Common components used; no reinvented control.
- [ ] Every clickable element is a button with a translated accessible name; visible focus; keyboard operable.
- [ ] Loading, empty and error handled.
- [ ] All strings translated in `en` and `fr` ("…", « », formal « vous », sentence case in French).
- [ ] Figures in `tabular-nums`, numbers and dates through `Intl`.
- [ ] Holds up at 1000×600 and 1920×1080; no fixed height on a content area.
- [ ] Animations justified, 150 to 200 ms, `motion-reduce` on every loop.
- [ ] Visual check done (skill `crate-visual-check`) or explicitly reported as not done.
- [ ] `yarn check:svelte` and `yarn test` green; `yarn format:check` and `yarn lint:check` on the touched files.
- [ ] Tracking up to date: CHANGELOG entry (section "Personal fork — change log", in English, identifier in brackets), box ticked in `tracking/STATUS.md` with a note if partial, `yarn status` rerun, DESIGN.md updated if a rule has changed.

Do not commit or push: hand back with the list of modified files, the identifiers addressed and the proposed commit message (`fix(ui): … [D11]`). The main session reviews and commits.
