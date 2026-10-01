---
name: charter-contrast-method
description: How Crate's charter contrast ratios were computed without a browser harness, and the pitfalls found (glass composite, accent fills)
metadata:
  type: reference
---

The measured-contrast table in DESIGN.md was computed with a small Node WCAG script from the hex values in style.css and the Tailwind palette, before the browser harness existed (it now does: `yarn harness`, `yarn test:e2e`, which measure rendered contrast).

Pitfalls worth remembering:
- Glass surfaces must be composited before measuring: Pulse cards = surface-1 at 70% over surface-0 (#131316 dark, #fcfcfc light).
- White text on the accent fill fails 4.5:1 for all ten accents; black passes all ten, so `--brand-on` is black everywhere (CRA-141). On the darker `brand-hover` fill black drops to 3.34–4.47 for blue, indigo, violet, purple and rose: still an open point.
- Dark `text-tertiary` (#71717a) is only 3.67:1 on surface-1, 3.08:1 on surface-2.
- In the light theme every family hue at -400/-500 fails as text; the -700 shades pass (4.81 to 6.69).

Related: [[owner-visual-preferences]].
