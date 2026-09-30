---
name: owner-visual-preferences
description: Clément's visual decisions for Crate's UI — today's look is frozen (CRA-141), keep colour families, his colour vocabulary
metadata:
  type: feedback
---

**Freeze (CRA-141, 2026-09-30): no visible change in the dark theme.** Clément likes today's look (Player neon glass in cyan and amber, Pulse colour per metric, coloured toolbar badges, Beatport neon green) and reopened the charter because the first version *prescribed changes* to it. Until he answers CRA-141: no colour, radius, glow, font weight, text size, hover/press or layout change, no new colour token — even when DESIGN.md's charter or a "rule" (rounded-md buttons, no glow, 12 px data text) says otherwise.

**Why:** a D11 pass that unified buttons, badges and checkboxes onto the common components' look (rounded-md, font-medium, no glow, 12 px keys, Crate Checkbox) was sent back in full: "remove every visible change".

**How to apply:** when extracting a shared component, give it props/variants so each call site renders pixel-identical to before; never change the look to fit the component. Allowed meanwhile: accessible names, roles, keyboard handling, translations (same French words), a11y cursor fixes, bug fixes of wrong state (e.g. a highlight on the wrong segment), replacing `confirm()` with ConfirmModal, light-theme text that cannot be read (darker shade of the same hue), new screens reusing today's look. Compare dark 1400×900 screenshots before/after and list every difference.

Colour families stay (CRA-115): never propose "bring the view back to the accent" for Player, Pulse or Beatport.

His vocabulary: "neon glass" = Player glass hero with glowing cyan waveform and amber cues, and the Pulse glass cards; "sienne" was a mis-transcription of "cyan". Ask rather than guess when he names a colour. See [[charter-contrast-method]], [[tooling-pitfalls]].
