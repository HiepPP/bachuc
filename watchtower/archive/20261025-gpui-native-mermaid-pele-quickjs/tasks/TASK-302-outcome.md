# TASK-302 Outcome

## Status

DONE. TASK-301 reported GO, so the ADR was written.

## Changed

- `watchtower/decisions/ADR-0067-pele-on-quickjs-over-hidden-webview-mermaid.md` (new, accepted, 2026-10-25)
- `watchtower/decisions/ADR-0037-hidden-webview-png-snapshots-over-svg-and-inline-webviews.md` (`Status:` line only: `superseded by ADR-0067 (Mermaid only)`)
- `watchtower/DECISIONS.md` (new ADR-0067 row; ADR-0037 status; the formatter re-aligned the table padding, 5 changed lines with `git diff -w`)
- `watchtower/MEMORY.md` (`## Avoid` ADR-0037 line now bans SVG export of KaTeX only; new ADR-0067 line)
- `docs/gpui.md` (`diagram` row; the formatter re-aligned the table padding, 4 changed lines with `git diff -w`)

## Contract

- Produces: `watchtower/decisions/ADR-0067-pele-on-quickjs-over-hidden-webview-mermaid.md`.
- Consumes: TASK-301 GO, init 117 to 142 ms, overall p95 7.44 to 9.58 ms. The ADR cites them.
- The ADR holds the owner choices (vendored Pele with `include_str!`, gate limits, 2x PNG through resvg, `Failed` with no mermaid.js fallback), the ADR-0045 note, four options, and three Revisit If bullets. It also notes the `crates/diagram/assets/**` exclusion from oxlint, oxfmt, and the lint hook (commit `447a1b36a`).

## Verified

- `git diff --stat watchtower/decisions` -> only ADR-0037 changed, 1 insertion and 1 deletion; ADR-0067 is untracked (new).
- `rg -n "ADR-0067" watchtower/DECISIONS.md docs/gpui.md watchtower/MEMORY.md` -> one match in each file.
- `rg -c "## Revisit If" watchtower/decisions/ADR-0067-*.md` -> 1; 3 bullets under it.
- `npm run format:files -- docs/gpui.md` -> exit 0, no further change (the first run also formatted the other four changed files).

## Anti-goal

No ADR body other than ADR-0067 changed. Checkpoint (command `git diff watchtower/decisions`, source: git, 2026-10-25): ADR-0037 diff is the `Status:` line only.

## Lessons

- `oxfmt` formats `docs/gpui.md` here and re-aligns the whole table, so a one-row edit shows a 38-line diff. Use `git diff -w` to see the real change.
- Rewording an ADR-0037 `## Avoid` line (R4) still needs a new line for the superseding ADR.
