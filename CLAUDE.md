# Crate — rules for coding assistants

Personal fork of `blackboxaudio/crate` (remote `upstream`), pushed to the private repository `origin` (`clementvnrd/Crate`). The owner is Clément: **replies to him are in French** (§0.2), **everything written in the repository is in English** (§0.4).

## 0. Standing rules (highest priority)

### 0.1 Capture instructions as they are given

In **every chat**, whenever Clément says something that reads like an instruction about how the project should be built, run, reviewed or communicated — a convention, a constraint, a preference, a "never do X", a "from now on" — record it in this file **in the same reply**, before or alongside doing the work. **Place it in the section it belongs to, replacing what it supersedes** — never append it below an older rule it contradicts. If it is ambiguous whether it is a durable rule or a one-off, add it under §9 "Open questions" rather than dropping it. Rules that are clearly one-off ("just for this test", "for now") are **not** added here.

### 0.2 How to talk to Clément — French, objective-driven, English technical terms

Replies to Clément are **in French**, written to answer "what does this mean for me and what do I do now", never to display technical detail.

**Every technical term stays in English** (Clément, 2026-09-25) — *commit, push, branch, merge, build, feature flag, migration, schema, IPC, store, hook, lint, snapshot, backup, parsing, fixture…* Write "le build desktop passe", not "la compilation de bureau réussit". Section titles and table headers follow the same rule. Why: Clément should never have to translate French back into English in his head.

**Structure every substantial answer like this:**

1. **Open with the headline** — what the situation actually is, in one or two sentences. Unblocked? Blocked? What changed? Never open with a tool output or a file listing.
2. **Number the sections** and give each one a title that states its point.
3. **Explain the *why*, not just the *what*.** After any technical fact, add the consequence in plain words ("Ce que cela signifie : …").
4. **Use before/after comparison tables** when something has changed, with a column *"Pourquoi c'est mieux"*.
5. **Close on the concrete deliverable** — what is produced, how to run it, what result proves it works.

**Every command given to run states the directory it must be run from** (Clément, 2026-09-30): `yarn …` from the repository root (`~/Coding Projects/crate`), `cargo …` from `src-tauri/`. Why: a command without its place is ambiguous and gets run in the wrong place.

**Do not:** make raw command output, paths or line numbers the substance of an answer (use them as evidence *inside* a sentence that already states the point); write dense tables of technical parameters with no interpretation; lead with caveats before the main message; be terse to the point of being cryptic.

**The register, from Clément's own model answer (2026-09-22), abridged:**

> Excellente nouvelle ! Tout s'éclaire et la situation est désormais parfaitement débloquée. […] Voici le résumé clair et structuré de ce qui s'est passé, de ce qui a changé, et de ce qu'il te reste à faire.
>
> **1. La grande nouveauté : le problème est résolu !** […]
> *Ce que cela signifie : …*
>
> **2. Ce qui a changé entre hier et aujourd'hui**
> | Sujet | Avant | Maintenant | Pourquoi c'est mieux |
>
> **3. Ta mission : ce qu'il faut livrer** […]

### 0.3 Keep this file lean — hygiene check after every major task (Clément, 2026-09-29)

After every **major task** — a batch of defect fixes, a push, a decision taken, a view redesign — and before the closing report, re-read this file end to end and check four things:

1. **Nothing is stale.** Every version, date, status and "still to do" matches the current documents and the real state. Settled items are removed, not struck through.
2. **Nothing volatile is hard-coded.** Commit hashes, test counts, progress and "state as of" snapshots live in `tracking/STATUS.md`, `tracking/DEFECTS.md` and `CHANGELOG.md`, or are read from a command (`yarn status`, `git log`). This file holds durable rules and binding facts, and points to where the moving parts live.
3. **Nothing is duplicated or contradictory** between sections, or between this file and `DESIGN.md` / `tracking/` — "one fact, one place".
4. **Every addition sits in the section it belongs to**, replacing what it supersedes rather than being appended below it.

Close the report with one line: "CLAUDE.md: checked — N changes" or "checked — no change". Removing or rewording a rule Clément gave is proposed to him, never done silently.

### 0.4 Everything written in the repository is in English (Clément, 2026-09-30)

Every `.md` document (README, CHANGELOG, `tracking/`, `DESIGN.md`, `.claude/` agents and skills, handoffs), every script and its output, and every commit message is written in **English**. Only replies to Clément in chat are in French (§0.2). Why: the English technical vocabulary gets lost in translation, and a single language keeps file names, headings and the scripts that parse them consistent. On 2026-09-30 the whole tracking system was migrated (`suivi/` → `tracking/`, `AVANCEMENT.md` → `STATUS.md`, `yarn suivi` → `yarn status`) and all earlier documents translated — a clean break with the previous way of working. Verbatim quotes keep their original wording. User-facing app strings are not documents: they go through i18n (`en.json` + `fr.json`).

## 1. Mandatory tracking

- Every fix references an identifier from the defect register (`tracking/DEFECTS.md`): `fix(zone): description [B12]`.
- In the **same commit**: tick the box in `tracking/STATUS.md` (italic note when partial or deferred), run `yarn status`, add the entry to `CHANGELOG.md` (section "Personal fork — change log"), and a line to the session log in `STATUS.md`.
- Update the README and other `.md` files whenever documented behaviour changes.
- Small thematic commits, pushed to `origin`.

## 2. Technical rules

- Rust desktop (from `src-tauri/`): the `desktop` feature is not a default. `cargo test --features desktop`, `cargo clippy --features desktop -- -D warnings`.
- Every desktop-only module is guarded by `#[cfg(feature = "desktop")]` (the mobile build uses `--no-default-features --features mobile`).
- Tauri IPC: Rust parameters in snake_case, JS keys in camelCase; an IPC rejection is a **string**, never an `Error`.
- Migrations: only appended at the end of `schema::get_migrations()`, never renumbered once released.
- **Never** a test that opens `~/Library/...` or the real Crate / Mixed In Key databases: `tempfile` / `:memory:` only.
- **Mixed In Key is read-only**: no write to `Collection11.mikdb`.
- **No secrets** in the code (usernames, tokens, passwords).
- Frontend: theme through `[data-theme]` (no `dark:`), colours through tokens, shared components (`Button`, `Modal`, `Tooltip`…), no hard-coded strings (i18n key in `en.json` + `fr.json`).
- Design: `DESIGN.md` is authoritative. All interface work (audit, D* fix, redesign, new view, mock-up) is delegated to the `design` agent (`.claude/agents/design.md`); `yarn design:scan <files>` must show no new occurrence.
- Tests (from the repository root): `yarn test` (Vitest), `yarn check:svelte`.

## 9. Open questions

_None._
