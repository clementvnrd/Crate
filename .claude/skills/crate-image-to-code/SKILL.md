---
name: crate-image-to-code
description: Turn an image into a faithful Crate interface — screenshot of a visual bug, mockup, generated image, screenshot of another app (Rekordbox, Serato, Spotify…) or an inspiration DESIGN.md file (awesome-design-md). Systematic analysis of the image, translation into Crate's tokens and components, implementation without drift, side-by-side comparison. Use as soon as an image or a visual reference comes with the request.
argument-hint: "[image path or reference] [target view]"
---

# From image to code, within Crate's system

Adapted from `image-to-code` ([Leonxlnx/taste-skill](https://github.com/Leonxlnx/taste-skill), MIT): its central idea — the image is a **specification** that is analysed before coding, then implemented without drifting towards a generic template — holds for Crate. Its "generate landing page mockups first" part does not apply: Crate already has its system, and the image is used to **decide on a layout**, not a palette.

## Possible image sources

| Source | What we take from it | What we never take from it |
| --- | --- | --- |
| Screenshot of Crate (bug, before/after) | The exact state to fix or to reach | — |
| Mockup or sketch by the owner | Layout, hierarchy, content, spacing | Colours and fonts if they contradict the system (flag it) |
| Screenshot of another DJ or music app | Idea for organising information, density, interaction pattern | Palette, font, logo, brand style |
| Inspiration DESIGN.md | Principles (hierarchy, density, restraint) | Raw tokens: they are translated into Crate tokens |

## 1. Analyse the image as a specification

Look at the image (`Read` tool on the file) and fill in this table **before** coding:

| Axis | What to record |
| --- | --- |
| Role | Which view, which area, which DJ task |
| Visual priority | What the eye sees first, second, third |
| Structure | Grid, columns, alignments, fixed and scrolling areas |
| Text | All readable text, word for word (it becomes i18n keys) |
| Typography | Size and weight ratios, lines, truncation, aligned figures |
| Spacing | Between heading and content, between rows, inner margins, gutters; logic, not pixels |
| Components | Buttons (solid, ghost, icon), badges, fields, separators, menus |
| Colour | Where colour carries information and where it is decorative |
| States | Hover, selection, now playing, empty, loading, error — visible or implied |
| Density | Number of elements per screen, compared with Crate's library |
| Blind spots | What the image does not say |

For a screenshot of another app: also note **why** it works (e.g. "BPM and key sit in a fixed column on the right, readable without searching").

## 2. Translate into Crate

Write the mapping table:

| In the image | In Crate |
| --- | --- |
| Background, panels, controls | `surface-0` / `surface-1` / `surface-2` depending on elevation |
| Accent colour of the reference | `brand-primary` (the accent chosen by the user) |
| Information colours (key, energy, status) | data palettes (`getCamelotColor`, `getEnergyInfo`) or `danger/warning/success` |
| Font of the reference | the user's font (`var(--font-family)`), `Text` scale |
| Button, field, menu, modal | `Button`, `Input`, `Select`, `ContextMenu`, `Modal`… |
| Radii and shadows | DESIGN.md system |
| Icons | `Icon.svelte` (add the missing ones) |

**Order of priority in case of conflict**: Crate's rules > fidelity to the image > ease of implementation. Every discrepancy imposed by the system is listed for the owner ("the mockup uses a purple background; I kept the user accent").

## 3. Implement without drifting

- Keep the image's layout, order of information, size ratios and spacing rhythm.
- Do not "simplify" into a generic template, do not tighten generous spacing or loosen an intended density, do not reintroduce nested cards that the image does not have.
- Blind spot: 1) keep the visible language, 2) keep the spacing logic, 3) keep the component family, 4) choose the simplest and most faithful version — do not fill the gap with a generic default.
- Everything else follows `crate-ui-build` (i18n, states, complete execution, final check).

## 4. Compare

If the browser harness is available (skill `crate-visual-check`): capture the result at the same window size as the image, look at them side by side, list the discrepancies (structure, hierarchy, spacing, text) and iterate at most three times. Otherwise, describe the expected discrepancies and ask the owner for a screenshot.

## Inspiration library

[awesome-design-md](https://github.com/VoltAgent/awesome-design-md) (MIT, © 2026 VoltAgent) collects 73 DESIGN.md files extracted from real websites. They describe **brands**, not tools: read them for principles, never to copy values. The ones relevant to an app like Crate:

| File | Interest for Crate |
| --- | --- |
| `spotify` | Music, dark, artwork, track lists |
| `linear.app` | Dense, dark product, surface scale, a single rare accent |
| `raycast` | macOS desktop app, keyboard first, compact lists |
| `superhuman` | Density and speed, visible shortcuts |
| `warp` | Dark desktop tool, data readability |
| `elevenlabs` | Audio, waveforms, dark |
| `sentry`, `posthog` | Data dashboards, charts, statistics |

Read on demand (do not copy them into the repository):

```bash
curl -s https://raw.githubusercontent.com/VoltAgent/awesome-design-md/main/design-md/spotify/DESIGN.md
```

For a reference format of the file, see the structure of Crate's [DESIGN.md](../../../DESIGN.md), modelled on these files (token frontmatter, then overview, colours, typography, layout, elevation, shapes, components, do's and don'ts, responsive, iteration guide, gaps).
