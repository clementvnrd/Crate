---
name: tooling-pitfalls
description: Blind spots of design:scan and the e2e audit (overlaps ignore scroll clipping), e2e flakiness, global shortcuts that block button activation
metadata:
  type: reference
---

- `yarn design:scan` reads only `class="…"` attributes in templates. Palette classes and hex values held in script objects (e.g. the `tone` maps of `Button` and `SegmentedControl`) are NOT counted, so moving copies into a shared component's script lowers the count by more than the real debt removed. Say so in any before/after report; grep the component's script for `cyan-|emerald-|#[0-9A-F]` to count what is hidden.
- `yarn test:e2e` (74 cases, 4 workers) sometimes fails the last cases of a full run with `waitForApp` timing out on `#wizard-view-switcher` (15 s) under load. Rerun the failing group with `npx playwright test --config e2e/playwright.config.ts -g "<theme> <size> <lang>"` before suspecting the code; it was green on rerun before any change.
- `e2e/app.ts` and `harness.e2e.ts` locate views by `#wizard-view-switcher button` + text "Player"/"Beatport": keep that id and those labels in both languages.
- The e2e ratchet counts text under 12 px per element: a shared badge at `text-[11px]` replacing a 12 px copy raises `smallText` (it happened with the harmonic wheel). DESIGN.md's key-badge spec is `text-xs`.
- `ui-audit.js` `overlaps` ignores scroll clipping and collapsed (0fr) containers: every focusable element (`tabindex="0"`, button) in rows scrolled out of view or collapsed is counted as overlapping whatever sits there (the player bar). Making list rows tab stops or laying "stretched" transparent buttons over rows raised Player 1000 overlaps 2 -> 16 and Discovery 5 -> 81 (D10, 2026-09-30). What passes: one tab stop on the clipped scroll container (grid + aria-activedescendant, rows `tabindex="-1"`), `inert` on collapsed parts.
- `ui-audit.js` `overlaps` also counts elements hidden BEHIND an open modal `<dialog>` (e.g. the library search input under the Upgrader/Duplicate Killer header buttons). Translating a label to a shorter/longer string can shift right-aligned header buttons by a few px and raise the count by 1 with no real problem (L1, 2026-09-30: English "Refresh" in the Upgrader -> "Scan again" kept the count). Measure boxes with a Playwright script before assuming a real overlap; a real fix is making the audit skip content outside `dialog[open]`.
- The global shortcuts (`useKeyboardShortcuts.ts`, window keydown) call `preventDefault` on Enter, Space and Shift+Tab whenever no input is focused: outside modals a focused button cannot be activated from the keyboard, and Shift+Tab switches views. New keyboard handlers must `stopPropagation`; test keyboard flows with Tab only (never Shift+Tab) in the harness.
- `yarn test:e2e` emulates reduced motion, so adding `motion-reduce:animate-none` changes screenshots (static ping/pulse dots). The Pulse screenshots sometimes catch a toolbar tooltip (Duplicate Killer) under the cursor: noise, not a regression. PIL (python3) is available for pixel diffs of `e2e/screenshots/` (copy the before set first; the run overwrites it).

Related: [[charter-contrast-method]].
