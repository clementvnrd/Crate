---
name: crate-ui-audit
description: Design and accessibility audit of a view, a folder or Svelte components in Crate — tokens, common components, Web Interface Guidelines (Vercel), "AI" signatures (taste-skill), i18n, contrast. Produces file:line findings linked to the defect register. Use for "audit / review / check the design of…", before a redesign, or to review an interface diff.
argument-hint: "[file | folder | view name]"
---

# Crate interface audit

An audit **observes**, it does not fix: no file is modified unless explicitly requested. Target: `$ARGUMENTS` (if empty, ask which view or folder; for a diff, `git diff --name-only -- apps/desktop/src`).

## Procedure

1. **Scope.** List the `.svelte` files concerned and, for a view, the child components it renders. Note the view in the app (Player, Library, Pulse, Beatport, Settings…).
2. **Automatic measurement.** `yarn design:scan <paths> --details`. Each line is a candidate finding; check it in the code before reporting it (the `hardcoded-text` rule is heuristic, brand logos are exceptions).
3. **Reading.** Read each file in full. Check, in order:
   - the rules of [DESIGN.md](../../../DESIGN.md) and of the `crate-design-system` skill (tokens, radii, elevation, common components, typography, states);
   - the [adapted Web Interface Guidelines](references/web-interface-guidelines.md) (accessibility, focus, forms, animation, typography, content, performance, language, copywriting);
   - the ["AI" signatures in a dense app](references/anti-slop-product.md);
   - i18n: every visible string goes through `{$translate('key')}` (`translate` imported from `$shared/i18n`) and exists in `en.json` and `fr.json`;
   - layout robustness: `min-w-0`, `minmax(0, …)`, no fixed height, 1000×600.
4. **Visual check** if the browser harness is available (skill `crate-visual-check`): light and dark, two accents, 1000×600 and 1400×900, measured contrast. Otherwise, say so in the report ("not visually checked").
5. **Report** in the format below, then an ordered **fix plan**.

## Severity

- **Blocking**: invisible or unusable (contrast < 3:1, content cut off at 1000×600, action not reachable by keyboard, clickable element without a name, untranslated string visible in English).
- **Major**: outside the system (palette, hex, `dark:`, reinvented component, radius or elevation outside the system, view-specific colour, missing state).
- **Minor**: polish (`…`, `tabular-nums`, `transition-all`, weight, isolated arbitrary size, copywriting).

## Linking to the register

Link each finding to an identifier from `tracking/DEFECTS.md` when one exists:

| Identifier | Subject |
| --- | --- |
| D3 | Broken light theme, insufficient contrast |
| D7 | Beatport table too narrow at 1000 px |
| D10 | Accessibility (names, clickable `div`s, keyboard, focus, `motion-reduce`) |
| D11 | Reinvented components instead of the common ones |
| L1 | Untranslated strings |
| L3 | Numbers and units badly formatted in French |

A defect without an identifier is marked `[new]` and proposed for the register (category, file, fix, effort XS/S/M): it is up to the owner to add it, or to the main session to do so with their approval.

## Report format

Findings grouped by file, one line each, clickable path, no preamble:

```text
## apps/desktop/src/lib/components/stats/StatsKpiCards.svelte

StatsKpiCards.svelte:40 — major [D11] frosted-glass card (bg-surface-1/70 + backdrop-blur-xl + halo) → bg-surface-1, rounded-lg, no halo
StatsKpiCards.svelte:47 — blocking [L1] "Temps d'Écoute" hard-coded → key stats.kpi.listeningTime (en + fr)
StatsKpiCards.svelte:48 — minor font-black → font-semibold, tabular-nums

## apps/desktop/src/lib/components/common/Button.svelte

✓ compliant
```

Then:

1. **Summary**: number of findings per severity, `design:scan` counts before fixing.
2. **Fix plan**: ordered batches (blocking first, then tokens and common components, then polish), each with the identifiers concerned and an estimate.
3. **What could not be checked** (visual rendering, real keyboard interactions).

The report may be written in English (the main session speaks French to the owner). Do not restate the rules in the report: cite the expected fix.
