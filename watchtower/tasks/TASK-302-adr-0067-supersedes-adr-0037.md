# TASK-302 ADR-0067 supersedes ADR-0037

Group: B (writes watchtower decision files, `watchtower/MEMORY.md`, and `docs/gpui.md`; start order only)
Class: docs

## Brief

Goal: Record the move from mermaid.js in a hidden WebView to Pele on QuickJS with resvg, using the TASK-301 numbers.

Change: ADR-0037 accepted -> ADR-0067 accepted and ADR-0037 superseded for Mermaid; KaTeX keeps the hidden WebView.

Boundaries:

- Write this TASK only when TASK-301 reports `GO`. If it reports `NO-GO`, this TASK is `BLOCKED`.
- Follow R1 to R5 in `~/.claude/skills/watchtower/references/decision-template.md`. Never rewrite the body of ADR-0037; change only its `Status:` line and its index row.
- Put the owner choices from `## Decisions` in [watchtower/CONTEXT.md](watchtower/CONTEXT.md) into the ADR: vendored Pele with `include_str!`, the gate limits, 2x PNG through resvg, `Failed` with no mermaid.js fallback.
- State that [ADR-0045](watchtower/decisions/ADR-0045-native-rust-plugin-clients-over-webview-panes-or-shared-logic.md) bans an embedded JS engine for plugin clients only. `bachuc-diagram` is a host crate, so it does not conflict.
- Options Considered must list: mermaid.js in the WebView (ADR-0037), Pele in the hidden WebView, Pele on the daemon over a new RPC, and a native Rust Mermaid layout.
- `## Revisit If` must include: Pele stops releasing or breaks Mermaid syntax that agents write; QuickJS render p95 grows past 16 ms; GPUI's image path gains a scale-aware SVG render.

How:

- Add `ADR-0067-pele-on-quickjs-over-hidden-webview-mermaid.md` and its row in `DECISIONS.md`.
- Mark ADR-0037 `superseded by ADR-0067 (Mermaid only)`.
- Update the ADR-0037 line under `## Avoid` in `MEMORY.md`, so it no longer bans SVG export of Mermaid.
- Update the `diagram` row in `docs/gpui.md`.

Files:

- [watchtower/decisions/ADR-0067-pele-on-quickjs-over-hidden-webview-mermaid.md](watchtower/decisions/ADR-0067-pele-on-quickjs-over-hidden-webview-mermaid.md) (new)
- [watchtower/decisions/ADR-0037-hidden-webview-png-snapshots-over-svg-and-inline-webviews.md](watchtower/decisions/ADR-0037-hidden-webview-png-snapshots-over-svg-and-inline-webviews.md) (Status line only)
- [watchtower/DECISIONS.md](watchtower/DECISIONS.md) (new row, ADR-0037 status)
- [watchtower/MEMORY.md](watchtower/MEMORY.md) (`## Avoid` line for ADR-0037)
- [docs/gpui.md](docs/gpui.md) (`diagram` crate row)

Interfaces:

- Consumes: the measured init ms, overall p95 ms, and the `GO` or `NO-GO` verdict in `watchtower/tasks/TASK-301-outcome.md` from TASK-301
- Produces: file `watchtower/decisions/ADR-0067-pele-on-quickjs-over-hidden-webview-mermaid.md` for TASK-303

Expected result:

- WHEN TASK-301 reports `GO` THE ADR index SHALL list ADR-0067 as accepted and ADR-0037 as superseded for Mermaid.
- IF TASK-301 reports `NO-GO` THEN THE plan SHALL leave every decision file unchanged.

Anti-goal: no ADR body other than ADR-0067 changes; tripwire; read at completion with `git diff --stat watchtower/decisions`.

## Verify

- `git diff --stat watchtower/decisions` -> only the new ADR-0067 and a one-line change in ADR-0037.
- `rg -n "ADR-0067" watchtower/DECISIONS.md docs/gpui.md watchtower/MEMORY.md` -> one match in each file.
- `rg -n "## Revisit If" watchtower/decisions/ADR-0067-*.md` -> one match, with at least three bullets under it.
- `npm run format:files -- docs/gpui.md` -> no change left.
