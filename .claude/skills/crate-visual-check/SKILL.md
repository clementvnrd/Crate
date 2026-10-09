---
name: crate-visual-check
description: Visual check of Crate's interface with Playwright CLI — screenshots in light and dark themes, several accents, window sizes 1000×600 / 1400×900 / 1920×1080, French and English, reduced motion; in-page measured audit (WCAG contrast, unnamed buttons, overlaps, modals outside the window, crushed columns, text under 12 px); annotation session with the owner. Use before declaring interface work done, for an audit, or to compare with a mockup.
argument-hint: '[view] [light|dark|all]'
allowed-tools: Bash(playwright-cli:*) Bash(npx playwright:*) Bash(yarn harness) Bash(yarn test:e2e:*)
---

# Checking the interface with Playwright CLI

Tool: [microsoft/playwright-cli](https://github.com/microsoft/playwright-cli) (Apache 2.0). It drives a real browser through short commands and writes snapshots and screenshots to files, without filling the context. Full reference: `playwright-cli --help`.

## Prerequisites

1. **Playwright CLI installed.** Check: `playwright-cli --version`. If it is missing, **ask the owner** before installing anything (global install):
   ```bash
   npm install -g @playwright/cli@latest
   ```
   If it reports that Google Chrome is not found, do not download anything: point it at a Chromium already cached by Playwright with a config file (`playwright-cli --config <file> open <url>`, the option is only read by `open`), for example `{ "browser": { "browserName": "chromium", "launchOptions": { "executablePath": "<~/Library/Caches/ms-playwright/chromium-XXXX/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing>", "headless": true } } }`.
2. **The browser harness running.** Crate calls the Tauri backend (`invoke`) right at startup, so `yarn dev:vite` alone lands on the crash screen. The committed **browser harness** runs the real app with a fake backend (16 tracks, playlists, tags, Beatport, Pulse statistics, albums, duplicates, upgrader matches…). From the repository root:
   ```bash
   yarn harness        # http://localhost:1430/ (port fixed; `yarn dev` uses 1420 and 1421)
   ```
   Everything is in `apps/desktop/harness/` and its `README.md` (URL parameters such as `?theme=light&lang=fr&accent=amber&beatport=out&library=empty&playing=trk-03`, how to reach each screen, how to add a fixture or a handler). The console lists commands nobody mocked in `window.__harness.unmocked`: if a view is blank or wrong, look there first, and add the handler rather than concluding the app is broken.
   - Without a running harness, **say so clearly** in the report ("visual check impossible: no browser harness") and fall back on `yarn design:scan`, code review, and a screenshot request to the owner (`yarn dev`, then a screenshot of the view concerned in light and dark). Never claim to have checked visually.
3. **The automated matrix** (`yarn test:e2e`, from the repository root) already opens every view in both themes, at 1000×600 and 1400×900, in English and French, writes screenshots to `e2e/screenshots/<view>-<theme>-<width>-<lang>.png`, runs `ui-audit.js` and compares its counts with `e2e/baseline.json` (a ratchet: a count that rises fails, a count that falls is reported; lower the baseline with `yarn test:e2e:update-baseline` and commit it with the fix). Run it after any interface change, then use the procedure below for what it does not cover (accents, fonts, 1920×1080, states, a view as it reads on screen).

Screenshots and snapshots go into `.playwright-cli/` (ignored by git).

## Settings driven by localStorage

The app reads these keys on load (`app.html`); set them, then reload:

| Key              | Values                                                                                             |
| ---------------- | -------------------------------------------------------------------------------------------------- |
| `crate-theme`    | `dark`, `light`, `system`                                                                          |
| `crate-accent`   | `blue`, `indigo`, `violet`, `purple`, `pink`, `rose`, `orange`, `amber`, `emerald`, `teal`         |
| `crate-font`     | `open-sans`, `jost`, `dm-sans`, `inter`, `nunito`, `fira-code`, `ibm-plex-mono`, `source-code-pro` |
| `crate-language` | `en`, `fr` (and the other locales)                                                                 |

## Minimum matrix before "done"

| #   | Theme | Accent | Window    | Language |
| --- | ----- | ------ | --------- | -------- |
| 1   | dark  | blue   | 1400×900  | fr       |
| 2   | light | blue   | 1400×900  | fr       |
| 3   | dark  | orange | 1000×600  | en       |
| 4   | light | amber  | 1000×600  | en       |
| 5   | dark  | blue   | 1920×1080 | fr       |

Add `set-reduced-motion reduce` on one of the rows when animations are involved, and a monospace font (`fira-code`) when column widths are involved.

## Procedure

```bash
# 1. open the browser harness (started with `yarn harness`); appearance can also come from the URL
playwright-cli open "http://localhost:1430/?theme=light&accent=amber&lang=fr"
playwright-cli resize 1400 900

# 2. or set a combination through localStorage and reload
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

| Field                | Meaning                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Register defect |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| `lowContrast`        | Text under 4.5:1 (3:1 for large text), actual background computed through the transparent layers                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | D3              |
| `unmeasuredContrast` | Text placed on an image or a gradient: to be looked at on the screenshot                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | D3              |
| `smallText`          | Text under 12 px                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | rule 4          |
| `unnamedControls`    | Button, link or field without an accessible name                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | D10             |
| `pointerOnly`        | Element with a "hand" cursor that is neither a button nor a link (clickable `div`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | D10             |
| `overlaps`           | Overlapping controls (toolbar at 1000 px), measured on the part of each control that can be seen: boxes are cut by every ancestor that clips its overflow, following the containing blocks (an `absolute` box escapes ancestors below its positioned one, a `fixed` box all of them unless one has a transform, filter or containment, a modal dialog sits in the top layer), and by the arrow strip of a `.scroll-affordance` edge that shows a fade. While a modal `<dialog>` is open, only the controls inside it are compared: the rest of the page is inert and covered | D4              |
| `outOfWindow`        | Modal, menu or tooltip that extends outside the window                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | D6              |
| `crushedColumns`     | Text truncated in less than 48 px (crushed column)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | D7              |
| `pageOverflowX`      | Horizontal scrolling of the page                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | rule 10         |

The lists stop at 40 entries each (`const MAX = 40`); `yarn test:e2e` lifts that cap in memory so it counts every finding, which is what makes its baseline a real ratchet.

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
