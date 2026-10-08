# Crate — rules for coding assistants

Personal fork of `blackboxaudio/crate` (remote `upstream`), pushed to the repository `origin` (`clementvnrd/Crate`), which is **public** by Clément's decision (CRA-198, 2026-10-09): everything committed is world-readable. The owner is Clément: **replies to him are in French** (§0.2), **everything written in the repository is in English** (§0.4).

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

Every `.md` document (README, CHANGELOG, `tracking/`, `DESIGN.md`, `.claude/` agents and skills, handoffs), every script and its output, and every commit message is written in **English**. Only replies to Clément in chat are in French (§0.2). Why: the English technical vocabulary gets lost in translation, and a single language keeps file names, headings and the scripts that parse them consistent. Verbatim quotes keep their original wording. User-facing app strings are not documents: they go through i18n (`en.json` + `fr.json`).

## 1. Mandatory tracking

- Every fix references an identifier from the defect register (`tracking/DEFECTS.md`): `fix(zone): description [B12]`.
- In the **same commit**: tick the box in `tracking/STATUS.md` (italic note when partial or deferred), run `yarn status`, add the entry to `CHANGELOG.md` (section "Personal fork — change log"), and a line to the session log in `STATUS.md`.
- Update the README and other `.md` files whenever documented behaviour changes.
- **Delivery by pull request (Clément, 2026-10-01).** `gh` is installed, so the flow planned on 2026-09-30 is now active: small thematic commits on a branch named after the issue's `gitBranchName`, a pull request into `develop` (the real integration branch on `origin`; the assistant's own mirror branch `clementvnrd/develop` is no longer the delivery target), `Refs CRA-n` (or `Closes CRA-n`) in the PR body so Linear links it, then the issue moves to **In Review** with a comment giving the PR link. **Never merge a pull request and never enable auto-merge**: Clément merges from Linear's Reviews tab, and that merge is his validation. If he requests changes, the issue returns to **In Progress**. **Before opening each pull request** (Clément, 2026-10-08), a fresh subagent that receives only the diff, not the author's conclusions, reviews it; its findings are fixed or answered in the pull request.

### Linear — the live board (Clément, 2026-09-30)

Clément follows the project in **Linear** (workspace "HomeMade", team **Crate App**, key `CRA`, Linear MCP server `linear-server`). Work consistently with it, **during** the task and not only at the end, so he can see at any moment what is being built and what is left. Why: he wants a clear picture of progress without asking, and he is new to Linear, so the board is deliberately rich and every Linear concept is explained briefly the first time it is used.

- **One issue per piece of work, created before starting it.** Whenever Clément voices a new request, idea or bug, create the issue in the same reply (Backlog unless he decides to do it now). Never start work that has no issue.
- **Where it goes.** Five projects: *Consolidation 0.3.0* (the register defects, one **milestone per repair step**), *Polish & Unify 0.4* (scenario B ideas and open decisions, milestones = the pillars), *Release 1.0 & auto-update* (signed releases and the updater, milestones M1–M3), *Roadmap 0.5+* (work after 0.4, milestones = themes) and *Repository & workflow* (everything around the app). New work joins the project and milestone it belongs to. Roadmap themes are **project labels** (Reliable core, Fluid & light, DJ toolkit, Stats & integrations, Release & distribution) because the Linear connection cannot create initiatives; the plan itself is in the Linear documents "Roadmap 0.4 → 1.0", "Audit report — 2026-10-08" and "Idea bank".
- **Title.** Register defects start with their identifier, `[B12] Title`, so the title matches the commit message. Work outside the register has a plain title.
- **Labels.** One type (`Bug`, `Improvement` or `Feature`), one **Area** (`Rust backend`, `Frontend`, `IPC`, `Design`, `i18n`, `Tooling`), and `Owner action`, `Decision` or `Process` when they apply. Priority **Urgent** is reserved for critical defects (C*).
- **Status says who has the ball (Clément, 2026-09-30).** `Backlog` (not decided) → `Improve issue` (**Claude's turn to write the spec**: a raw, hand-written request that Clément parked there; a custom status in the Backlog category, see the `Improve issue` bullet below) → `On Hold` (**kept on purpose, not now**: important, but Clément chose not to do it for now; never touched by assistants and ignored by the hourly watch) → `Todo` (decided, ready to be taken) → `Needs input` (**waiting for Clément**: a question, a decision, or a confirmation for him; a custom status in the Started category, placed between Todo and In Progress) → `In Progress` (**Claude's turn**: started, or Clément answered and waits for me) → `In Review` (code pushed, **Clément must check it**) → `Done` (verified, or decided). When Clément has answered an issue, he moves it from Needs input to In Progress. **Every comment I post is followed, in the same step, by a status change, then a re-read to confirm it took effect** (a comment without a status change is a mistake):
  - I answered a question and he must decide or check again → **Needs input**.
  - He took a decision and I recorded it, nothing left to do on that issue → **Done** (I close `Decision` and `Owner action` issues once he has decided and it is recorded).
  - He took a decision but real work remains → the work becomes **its own issue** (Todo, or In Progress if I start now) and the decision issue is **Done**; only a small task I finish right away stays In Progress.
  - I need his explicit confirmation before something risky or irreversible → **Needs input**, saying which words confirm.
  - I open a new question or decision for him → **Needs input**.
  - I pushed code → **In Review**, never Done. Close with a comment giving the commit hash.
  - **An issue in `Improve issue` (Clément, 2026-10-03; meaning deduced from how he uses it).** It is not a request to code: rewrite it as a detailed English spec — root cause or context read from the code, expected behaviour, acceptance criteria, files involved, relations to existing issues (`relatedTo`, `blockedBy`, `parentId`), and a split into sub-issues when it is too big for one (each with its own type and Area label, project and milestone, and dependencies between them). Keep his original text and screenshots at the bottom of the description (edit with `patch`, never replace the whole description) and say what could not be verified (nothing was run, a screenshot could not be opened). Then post a comment and move the issue to **Needs input** so he can review the spec and move it to Todo. If the workspace limit blocks the sub-issues, list them in the parent's description instead and tell him.
- **Issue limit (Clément, 2026-10-03).** Linear's free plan caps the **whole workspace** (every team, retired ones included) at 250 issues, and `Canceled` or `Done` issues still count until they are archived. When creating an issue fails with "free issue limit", do not work around it. Clément authorised removing closed issues, on condition that a **log document** listing them (id, title, status) is first saved in the team's documents on Linear, so the history survives outside the limit. The assistant's Linear tools can neither delete nor archive an issue, so Clément does the removal himself in the Linear interface; deleting a whole retired team keeps a 30-day grace period. **Planning within the budget (Clément, 2026-10-08):** before creating issues in bulk, measure the headroom (active issues of every team against 250; archived ones do not count; whether recently deleted ones count is unknown, so plan for the worst case), show Clément the counts per project first, never go below a margin of 10, and prefer few rich issues with checklists; the rest of a plan goes into Linear documents, which cost no issue.
- **Commits** keep `[B12]` and add the Linear key in the body when an issue exists: `Refs CRA-42`.
- **Linear complements the repository files, it does not replace them.** `tracking/STATUS.md`, `tracking/DEFECTS.md` and `CHANGELOG.md` stay the versioned record (they survive without Linear); update both in the same step.
- **Clément answers in Linear, not in chat (Clément, 2026-09-30).** He answers decisions and questions by **commenting on the issue** and/or **moving it to Done** (closing a `Decision` or `Owner action` issue means "decided" or "done"). **At the start of every reply**, before anything else, check Linear for his feedback: list the open `Decision` and `Owner action` issues and read their comments (`list_issues`, `list_comments`), plus any issue he moved. Act on what he wrote, then confirm in the reply which answers were taken into account. Why: he wants to answer where the question is, without copying it into chat.
- **Who wrote a comment.** The Linear MCP server is signed in as Clément, so **my comments show him as the author**. Every comment I write starts with `🤖 Claude:`; a comment without that prefix is Clément's. Never treat a prefixed comment as his decision.
- **Hourly watch (Clément, 2026-09-30).** The scheduled task `crate-linear-hourly-check` (stored in `~/.claude/scheduled-tasks/`, runs while the desktop app is open) checks Linear every hour with the same rules as above and does nothing when nothing is new. Clément chose the highest autonomy: it may answer, record decisions, and **implement a register defect** when he gives a go (moving the issue to In Progress, or a decision that unblocks it), at most one per run, only on a clean `develop`, ending at In Review, never Done. It scans every open issue assigned to Clément or labelled `Decision` / `Owner action` (no time window) and follows the status rules above. Its limits are written in the task's prompt; change them there and here together.
- **Report.** Close every substantial answer to Clément with the Linear changes made (issues created, moved, closed, with their `CRA-n` keys).

## 2. Technical rules

- **Scope: macOS only (Clément, 2026-09-30, CRA-122).** Crate is used on this Mac only; work for other machines or platforms (Windows, Linux, mobile) has no value and is not prioritised. The existing platform guards below stay untouched until Clément decides to remove them.
- **Interface languages (Clément, 2026-09-30, CRA-114).** French and English are kept complete; the other 13 locales fall back to English, and the fallback is documented.
- **Overall plan (Clément, 2026-09-30, CRA-112; confirmed 2026-10-01).** Scenario "A then B": *Consolidation 0.3.0* and *Polish & Unify 0.4* are both in scope, in that priority order, but not strictly sequential — Clément confirmed on 2026-10-01 that both projects run in parallel rather than waiting for 0.3.0 to be 100% done first. Scenario C (pluggable sources, mobile companion, fingerprinting) stays out of scope.
- Rust desktop (from `src-tauri/`): the `desktop` feature is not a default. `cargo test --features desktop`, `cargo clippy --features desktop -- -D warnings`.
- Every desktop-only module is guarded by `#[cfg(feature = "desktop")]` (the mobile build uses `--no-default-features --features mobile`).
- Tauri IPC: Rust parameters in snake_case, JS keys in camelCase; an IPC rejection is a **string**, never an `Error`.
- Migrations: only appended at the end of `schema::get_migrations()`, never renumbered once released.
- **Never** a test that opens `~/Library/...` or the real Crate / Mixed In Key databases: `tempfile` / `:memory:` only.
- **Mixed In Key is read-only**: no write to `Collection11.mikdb`.
- **No secrets** in the code (usernames, tokens, passwords), and no absolute home paths (write `~/…`): the repository is public.
- Frontend: theme through `[data-theme]` (no `dark:`), colours through tokens, shared components (`Button`, `Modal`, `Tooltip`…), no hard-coded strings (i18n key in `en.json` + `fr.json`).
- Design: `DESIGN.md` is authoritative. All interface work (audit, D* fix, redesign, new view, mock-up) is delegated to the `design` agent (`.claude/agents/design.md`); `yarn design:scan <files>` must show no new occurrence.
- **Visual language (Clément, 2026-09-30, CRA-115; five open points decided 2026-10-01, CRA-141).** Clément likes today's look: the Player's neon glass in cyan and amber, Pulse's colour per metric, the coloured badges of the toolbar, Beatport's neon green (CRA-115, unchanged). `DESIGN.md`'s charter describes that look and, per his CRA-141 answers, now also **implements**: automatic black-or-white text on an accent-coloured button, chosen per accent for contrast; named colour tokens for every family (`--deck-live`, `--beatport`…) instead of palette classes in components; the Beatport view following the light/dark theme instead of staying always dark; and four homogenisation touches — Pulse cards `rounded-xl` instead of `rounded-2xl`, no hover lift/zoom on Pulse KPI cards, Duplicate Killer's and the Upgrader's waveform previews take the Player's cyan, toolbar shortcuts carry a family colour only on a label or a count badge. Nothing else about the dark theme changes outside these five points. This rollout is tracked as [D3] (`CRA-96`); `CRA-141` itself is closed once this rule and `DESIGN.md` reflect the decision.
- **Performance is a feature (Clément, 2026-10-08).** Every proposal — issue, pull request, roadmap item — states its performance cost (start-up, 50k-track library, idle CPU, memory, bundle size). The budgets and how each is measured are in the Linear document "Roadmap 0.4 → 1.0".
- **Gig safety (Clément, 2026-10-08).** Nothing may interrupt or slow a set: while audio plays or a long job runs, no automatic relaunch or install prompt, no modal opening by itself, no full re-sync or rescan.
- **Releases and auto-update (Clément, 2026-10-08, CRA-199).** The app trusts only the fork's own updater key and release manifests; `docs/RELEASING.md` is the binding procedure. The private signing key never enters the repository, an issue or a log, and is never printed; a published version is never replaced (bump instead); a rollback re-points the manifest and fixes forward. **Decided on CRA-198 (Clément, 2026-10-08 and 2026-10-09):** releases are GitHub releases of the public `clementvnrd/Crate`, and the app reads its channel manifests on the `update-channels` branch, which only `scripts/release/publish.mjs` changes (Q1); the fork's first version is **1.0.0** (pre-releases `1.0.0-staging.N`, Q2); releases are built by the GitHub Actions release workflow when the release's own `v*` tag is pushed, with the one-command build on the Mac as the backup (Q4); push that one tag, never `git push --tags`; upstream's tags are not to be kept or fetched locally (Q5).
- **Beatport download stays (Clément, 2026-09-30, CRA-113; confirmed for a public repository 2026-10-09, CRA-198: "Beatport stays").** The FLAC download through the third-party `beatportdl` tool is kept as it is, even though the repository is public. Do not isolate, rewire or remove it.
- Tests (from the repository root): `yarn test` (Vitest), `yarn check:svelte`, and `yarn test:e2e` (Playwright on the browser harness `yarn harness`, port 1430). Interface work ends with `yarn test:e2e`: its audit counts (contrast, unnamed controls, overlaps…) are a ratchet in `e2e/baseline.json`, they may only go down, and a lower count is committed with `yarn test:e2e:update-baseline`.

## 9. Open questions

- **Linear versus `tracking/STATUS.md`** (2026-09-30): both currently track the same defects. Which one is authoritative in the long run, or should a script keep them in sync? Until Clément decides, both are updated in the same step (§1, "Linear").
- **Hourly watch and `Improve issue`** (2026-10-03): the scheduled task `crate-linear-hourly-check` does not know this status. Issues in it are assigned to Clément, so the watch may read them. Should it also rewrite them as specs (§1, "Improve issue"), or leave them to the interactive session? Until Clément decides, only the interactive session handles them.
- **Language of Linear content** (2026-09-30): issues and comments are written in English, like the repository, because they mirror commit messages and tracking files. Only the beginner's guide document is in French. Confirm or change.
