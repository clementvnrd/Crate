---
name: crate-visual-check
description: Visual check of Crate's interface with Playwright CLI — screenshots in light and dark themes, several accents, window sizes 1000×600 / 1400×900 / 1920×1080, French and English, reduced motion; in-page measured audit (WCAG contrast, unnamed buttons, overlaps, modals outside the window, crushed columns, text under 12 px); annotation session with the owner. Use before declaring interface work done, for an audit, or to compare with a mockup.
argument-hint: "[view] [light|dark|all]"
allowed-tools: Bash(playwright-cli:*) Bash(npx playwright:*) Bash(yarn dev:vite)
---

# Checking the interface with Playwright CLI

Tool: [microsoft/playwright-cli](https://github.com/microsoft/playwright-cli) (Apache 2.0). It drives a real browser through short commands and writes snapshots and screenshots to files, without filling the context. Full reference: `playwright-cli --help`.

## Prerequisites

1. **Playwright CLI installed.** Check: `playwright-cli --version`. If it is missing, **ask the owner** before installing anything (global install):
   ```bash
   npm install -g @playwright/cli@latest
   ```
2. **A frontend running in a browser.** Crate calls the Tauri backend (`invoke`) right at startup: opened as is in a browser (`yarn dev:vite`, port 1420), it lands on the crash screen. You need a **browser harness** that replaces the backend with mock data (fake IPC through `@tauri-apps/api/mocks`, about fifteen tracks, theme and language settings). The one used for the 25 September audit was not committed.
   - If the browser harness exists (look for a `harness` script in `package.json` or a `harness/` folder), start it and use its URL.
   - Otherwise, **say so clearly** in the report ("visual check impossible: no browser harness") and fall back on: `yarn design:scan`, code review, and a screenshot request to the owner (`yarn dev`, then a screenshot of the view concerned in light and dark). Never claim to have checked visually.

Screenshots and snapshots go into `.playwright-cli/` (ignored by git).

## Settings driven by localStorage

The app reads these keys on load (`app.html`); set them, then reload:

| Key | Values |
| --- | --- |
| `crate-theme` | `dark`, `light`, `system` |
| `crate-accent` | `blue`, `indigo`, `violet`, `purple`, `pink`, `rose`, `orange`, `amber`, `emerald`, `teal` |
| `crate-font` | `open-sans`, `jost`, `dm-sans`, `inter`, `nunito`, `fira-code`, `ibm-plex-mono`, `source-code-pro` |
| `crate-language` | `en`, `fr` (and the other locales) |

## Minimum matrix before "done"

| # | Theme | Accent | Window | Language |
| --- | --- | --- | --- | --- |
| 1 | dark | blue | 1400×900 | fr |
| 2 | light | blue | 1400×900 | fr |
| 3 | dark | orange | 1000×600 | en |
| 4 | light | amber | 1000×600 | en |
| 5 | dark | blue | 1920×1080 | fr |

Add `set-reduced-motion reduce` on one of the rows when animations are involved, and a monospace font (`fira-code`) when column widths are involved.

## Procedure

```bash
# 1. open the browser harness (adapt the URL)
playwright-cli open http://localhost:1420/
playwright-cli resize 1400 900

# 2. set a combination and reload
playwright-cli localstorage-set crate-theme light
playwright-cli localstorage-set crate-accent amber
playwright-cli localstorage-set crate-language fr
playwright-cli reload

# 3. go to the view (refs read from the snapshot)
playwright-cli snapshot
playwright-cli click e12

# 4. capture and measure
playwright-cli screenshot --filename=.playwright-cli/pulse-light-amber-1400.png
playwright-cli run-code --filename=.claude/skills/crate-visual-check/scripts/ui-audit.js

# 5. repeat for each row of the matrix, then close
playwright-cli close
```

Name screenshots `<view>-<theme>-<accent>-<width>[-<state>].png` so that before/after can be compared.

### What `ui-audit.js` measures

The script runs in the page and returns a JSON report:

| Field | Meaning | Register defect |
| --- | --- | --- |
| `lowContrast` | Text under 4.5:1 (3:1 for large text), actual background computed through the transparent layers | D3 |
| `unmeasuredContrast` | Text placed on an image or a gradient: to be looked at on the screenshot | D3 |
| `smallText` | Text under 12 px | rule 4 |
| `unnamedControls` | Button, link or field without an accessible name | D10 |
| `pointerOnly` | Element with a "hand" cursor that is neither a button nor a link (clickable `div`) | D10 |
| `overlaps` | Overlapping controls (toolbar at 1000 px) | D4 |
| `outOfWindow` | Modal, menu or tooltip that extends outside the window | D6 |
| `crushedColumns` | Text truncated in less than 48 px (crushed column) | D7 |
| `pageOverflowX` | Horizontal scrolling of the page | rule 10 |

A 0×0 window (hidden or unsized browser) returns an error: set a size with `resize` and run it again.

### Keyboard accessibility

```bash
playwright-cli press Tab        # repeat and check in the snapshot that focus moves in a logical order
playwright-cli snapshot         # the accessibility tree shows the button names
playwright-cli find --regex "button \\[ref="   # unnamed buttons (a named button reads button "Name" [ref=…])
playwright-cli press Escape     # a modal must close and return focus
```

## Feedback from the owner

For a design review, open the view and launch the annotation board: the owner draws boxes around areas and writes their remarks; you receive the annotated screenshot, the snapshot of the area and the notes.

```bash
playwright-cli show --annotate
```

## Fallback without Playwright CLI

If Playwright CLI is not available but the browser harness is running, the app's built-in browser (`mcp__Claude_Browser__*` tools) allows the same check: `resize_window` for the size, `javascript_tool` to set the `localStorage` keys and run the body of `ui-audit.js`, `computer` for screenshots. Mention it in the report.

## Report

For each row of the matrix: screenshots produced (paths), summarised `ui-audit.js` report (counts + findings), discrepancies visible on the screenshot. End with what could not be checked.
