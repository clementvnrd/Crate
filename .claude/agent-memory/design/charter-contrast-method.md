---
name: charter-contrast-method
description: How Crate's charter contrast ratios were computed without a browser harness, and the pitfalls found (glass composite, accent fills)
metadata:
  type: reference
---

The measured-contrast table in DESIGN.md was computed with a small Node WCAG script from the hex values in style.css and the Tailwind palette (no browser harness exists in the repo; `playwright-cli` is installed at /opt/homebrew/bin but there is nothing to point it at).

Pitfalls worth remembering:
- Glass surfaces must be composited before measuring: Pulse cards = surface-1 at 70% over surface-0 (#131316 dark, #fcfcfc light).
- White text on the accent fill (Button primary) fails 4.5:1 for all ten accents; black passes all ten. Any accent-fill work runs into this open decision.
- Dark `text-tertiary` (#71717a) is only 3.67:1 on surface-1, 3.08:1 on surface-2.
- In the light theme every family hue at -400/-500 fails as text; the -700 shades pass (4.81 to 6.69).

Related: [[owner-visual-preferences]].
