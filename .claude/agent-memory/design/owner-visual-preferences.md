---
name: owner-visual-preferences
description: Clément's visual decisions for Crate's UI — CRA-141 (2026-10-01) allows exactly five changes, the dark look is otherwise kept; colour families stay; his vocabulary
metadata:
  type: feedback
---

**CRA-141 decided (2026-10-01): the dark look stays, except five approved points.** (1) black/white text on accent fills per accent (`--brand-on`, black for all ten); (2) family colour tokens instead of palette classes; (3) Beatport follows the light/dark theme; (4) four touches: Pulse cards rounded-xl, no hover lift/zoom on Pulse KPI cards, DK/Upgrader waveform previews in the Player cyan, toolbar shortcuts coloured only on a label or count badge. Anything else visible in the dark theme is still off-limits.

**Why:** a first D11 pass that unified buttons, badges and checkboxes onto the common look was sent back in full ("remove every visible change"); he likes today's look and only approved these points one by one.

**How to apply:** a token's dark value must equal the palette shade it replaces (write it as `var(--color-cyan-400)`); prove it with a before/after dark pixel diff of every harness scene and list each difference. When a change is not clearly inside the five points, leave it and report it as a decision (still open after CRA-141: white text on accent count badges and the Checkbox check mark, the darker accent hover under black text, the energy badge and toolbar count badges in light, the MIK logo's sky label).

**Accessibility choices (CRA-100, 2026-10-01: "1a / 2i oui / 2ii non").** Enter/Space go to a keyboard-focused control (1a) but Shift+Tab keeps switching views (1b not chosen); the hand cursor is removed where a click does nothing (2i); the focus ring icon buttons show after a mouse click STAYS (2ii refused): do not make it keyboard-only. Focus states, roles and keyboard handling are fine without asking.

Colour families stay (CRA-115): never propose "bring the view back to the accent" for Player, Pulse or Beatport.

His vocabulary: "neon glass" = Player glass hero with glowing cyan waveform and amber cues, and the Pulse glass cards; "sienne" was a mis-transcription of "cyan". Ask rather than guess when he names a colour. See [[charter-contrast-method]], [[tooling-pitfalls]].
