# TASK-288 outcome

## Status

DONE

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
