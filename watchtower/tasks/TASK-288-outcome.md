# TASK-288 outcome

## Status

BLOCKED: BUILT, UNVERIFIED (Q-186). Code committed; see UI Checks.

## Changed

- `packages/desktop-gpui/crates/explorer/src/diff_view.rs` (`render_file_toolbar`; `render` picks the toolbar per source)

## Contract

- `fn render_file_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<>` in `DiffView`, element id `diff-file-toolbar`, wrap button id `diff-file-wrap`. `render_toolbar` is untouched.
- `Source::All` shows `render_toolbar`; `Source::File` and `Source::Commit` show `render_file_toolbar`, in every state (Loading, Failed, Empty, Ready).

## Verified

- `./cargo.sh test -p bachuc-explorer` -> 56 + 38 + 31 passed, 0 failed (group-level run, after TASK-291).
- `./cargo.sh clippy -p bachuc-explorer --no-deps -- -D warnings` -> only the known `large_enum_variant` at `file_view.rs:94` (the `State` enum; the line moved from 86 because another builder edits that file). With `-A clippy::large_enum_variant` it passes. Used `--no-deps` per AUTORUN.
- `./cargo.sh fmt --check -p bachuc-explorer` -> exit 0.
- Screen checks (toolbar and button on one-file and commit diff, press toggles label, 300 px width) -> DEFERRED (UI checker).

## Anti-goal

All-files toolbar keeps its 5 controls in order. Code read: `git diff` shows no line changed inside `render_toolbar`; the AX read of `diff-view-toolbar` before and at completion -> DEFERRED (UI checker). Code-read PASS.

## Lessons

- `file_view.rs` is edited by another builder at the same time, so the `large_enum_variant` line number drifts.

## UI Checks

UI checker, 2026-10-22, check bundle at `74d73ff80` on live (6768), zoom 1.25. Screenshots in `/tmp/wt-ui/` (`288-*.png`).

- Screen check, one-file diff from Changes has `diff-file-toolbar` and "Wrap long lines": UNVERIFIED. A press on a Changes row opens the all-files view (`open_all_diffs`), not the one-file diff. `ExplorerEvent::OpenDiff` has no emitter: `rg "OpenDiff" packages/desktop-gpui/crates packages/desktop-gpui/plugins` finds only the enum, the shell handler, and an example. `Source::File` is unreachable in the app. Pre-existing: the emitter went away with commit `57d2d6b47` (all-files view). The toolbar code itself was not observed for `Source::File`.
- Screen check, commit file diff from the commits list shows the same toolbar: PASS. AX: `toolbar Diff` holds only `button Wrap long lines`; the icon sits at the right end (`288-commit-diff.png`). Pressing it gives `button Scroll long lines`. The open all-files view followed to "Scroll long lines" and the LONGLINE row wrapped to 4 rows (`288-all-wrapped.png`). Wrap restored to "Wrap long lines" after.
- Screen check, 300 px pane keeps the wrap button inside the pane: UNVERIFIED at 300 px. With the Explorer open the pane never drops under 400 px (`WORKSPACE_BODY_MIN_WIDTH`); a 300 px pane has no way to open a commit diff. At 400 px (640 px window, 240 px Explorer) the wrap button sits at the right end inside the pane (`288-pane400.png`).
- Anti-goal, all-files `diff-view-toolbar` keeps 5 controls in order: PASS. After: AX `Diff mode`, `Collapse all files`, `Hide whitespace`, `Wrap long lines`, `Refresh`. Before: `git diff d82c3b9a2 HEAD -- diff_view.rs` has no hunk inside `render_toolbar` (line 625); only its call site in `render` changed.

Blocked: BUILT, UNVERIFIED (Q-186)
