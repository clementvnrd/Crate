# Project tracking

This folder tracks all the work of restoring the personal fork of Crate, since the audit of 25 September 2026. Since 30 September 2026 everything in it is written in English (see `CLAUDE.md` §0.4).

| File | Role | Updated |
| --- | --- | --- |
| [STATUS.md](STATUS.md) | Checkboxes per defect and per step, owner-only actions, session log | On every fix |
| [../CHANGELOG.md](../CHANGELOG.md) | Detailed log of every change (what changes for you, and why) | On every fix |
| [DEFECTS.md](DEFECTS.md) | The ~180 defects found during the audit, with their planned fix | Frozen (reference) |
| [AUDIT-REPORT.md](AUDIT-REPORT.md) | Situation, strengths, vision, phased plan, decisions | Frozen (reference) |
| [history/DISCUSSION-SUMMARY.md](history/DISCUSSION-SUMMARY.md) | Summary left by the previous assistant (builds 36 to 57) | Frozen (archive) |

## Way of working

1. We follow the register's **repair order** (12 steps): first what can destroy data, then what is wrong, then the polish.
2. **One defect (or a small related group) = one commit**, with the identifier in brackets in the message: `fix(stats): deduplicate Spotify listens [C10]`.
3. The **same commit** updates the code, its tests, the box in `STATUS.md` (then `yarn status` from the repository root) and an entry in `CHANGELOG.md`.
4. Every fix is **verified** before being ticked: Rust tests (`cargo test --features desktop`, from `src-tauri/`), Vitest (`yarn test`, from the root), and a manual check or a check in the browser harness when it is visual.
5. Tests **never** touch the real databases (`~/Library/...`): temporary databases only.
6. Pushed to GitHub after every step (or more often).
7. **Linear is the live board** (workspace "HomeMade", team Crate App, key `CRA`): every defect of the register is an issue titled `[B12] …`, every repair step is a milestone of the project *Consolidation 0.3.0*. The issue is moved to In Progress when work starts and to Done once the commit is pushed. The files in this folder stay the versioned record; the rules are in `CLAUDE.md` §1.

## Finding the history of a defect

From the repository root:

```bash
git log --oneline --grep "\[C11\]"
```
