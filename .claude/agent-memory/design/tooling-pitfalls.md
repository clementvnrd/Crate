---
name: tooling-pitfalls
description: Blind spots of yarn design:scan and flakiness of yarn test:e2e found while fixing D11 (2026-09-30)
metadata:
  type: reference
---

- `yarn design:scan` reads only `class="…"` attributes in templates. Palette classes and hex values held in script objects (e.g. the `tone` maps of `Button` and `SegmentedControl`) are NOT counted, so moving copies into a shared component's script lowers the count by more than the real debt removed. Say so in any before/after report; grep the component's script for `cyan-|emerald-|#[0-9A-F]` to count what is hidden.
- `yarn test:e2e` (74 cases, 4 workers) sometimes fails the last cases of a full run with `waitForApp` timing out on `#wizard-view-switcher` (15 s) under load. Rerun the failing group with `npx playwright test --config e2e/playwright.config.ts -g "<theme> <size> <lang>"` before suspecting the code; it was green on rerun before any change.
- `e2e/app.ts` and `harness.e2e.ts` locate views by `#wizard-view-switcher button` + text "Player"/"Beatport": keep that id and those labels in both languages.
- The e2e ratchet counts text under 12 px per element: a shared badge at `text-[11px]` replacing a 12 px copy raises `smallText` (it happened with the harmonic wheel). DESIGN.md's key-badge spec is `text-xs`.

Related: [[charter-contrast-method]].
