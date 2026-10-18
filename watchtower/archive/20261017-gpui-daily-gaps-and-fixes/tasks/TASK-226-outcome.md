# TASK-226 Outcome

## Status

DONE. The UI check is DEFERRED (UI checker).

## Changed

- `packages/desktop-gpui/crates/app/src/toast.rs`:
  - `toast_duration(ok)` is a pure rule: 2.2 s for a success note (Electron's `DEFAULT_DURATION_MS`) and 3.2 s for an error note (Electron's `toast.error`). `TOAST_DURATION` is gone.
  - `Toast` has a `duration` field. `ToastStack::push` returns `Option<&Toast>` instead of the id.
  - `MAX_TOASTS` is 1. Electron's host holds one toast, so a new note replaces the shown one.
  - The host sits at the top center: top `HEADER_HEIGHT + SPACE_2` (36 + 8, as Electron's app-shell `topOffset + spacing[2]`), left and right `SPACE_4`, a 92 % width boundary with a 480 px maximum width, and notes centered.
  - Tests: `a_new_toast_replaces_the_shown_one_and_leaves_by_id` (replaces the old 3-note test) and `a_success_toast_lasts_2_2_s_and_an_error_toast_3_2_s`.
- `packages/desktop-gpui/crates/app/src/shell.rs`: `show_toast` reads the pushed toast's id and duration and waits for that duration. Its doc line names the top center. The import drops `TOAST_DURATION`.

## Contract

- `Shell::show_toast(&mut self, ok: bool, message: String, cx: &mut Context<Self>)` keeps its signature.
- Produces: `toast::toast_duration(ok: bool) -> Duration`, `Toast::duration`, and `ToastStack::push(..) -> Option<&Toast>`. `rg` finds the only `push` caller in `shell.rs`.
- Anti-goal: a timer starts only in `show_toast` after a non-blank push. `render_toasts` returns `None` for an empty stack, so an empty list adds no element, timer, or repaint. A replaced note's timer still fires once, finds no id, and calls no `cx.notify()`. This is the same as before the change.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app toast` -> 2 passed, including the duration rule.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app` -> 80 passed.
- `./cargo.sh fmt --check -p bachuc-app` -> exit 0.
- `CARGO_BUILD_JOBS=4 ./cargo.sh clippy -p bachuc-app --tests` -> no warning in `toast.rs` or `show_toast`.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- GitNexus impact: `show_toast` and `render_toasts` are not in the stale index, and `push` is ambiguous (21 symbols). `rg` shows that every toast symbol is used only in `shell.rs`.
- UI check on live: a commit in the scratch repo shows the toast at the top center -> DEFERRED (UI checker).

## Lessons

- The first test run failed in `bachuc-explorer` (`has_branch` missing) while the other builder edited that crate. A later run compiled. A shared-checkout build failure in a crate outside your Files is not yours to fix; wait and retry.
- Electron's toast pauses its timer on hover (`pauseDismiss`/`resumeDismiss`). GPUI notes do not pause; that is outside this TASK.

## UI Checks

Run 2026-10-18 by the UI checker (run 5). Bundle `Bachuc GPUI Check` built from HEAD `08064a6f3` (includes `626143dc7` and TASK-225 `699395706`), PID 34022, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). Frames were forced with the AX "zoom the window" action; keys went to the check app only. Scratch repo `/tmp/t202-repo` (branches `feature-x` and `main`, new commit `1f8a4a9` with a `web` service script) was the live workspace `wks_9cd2d5d43fcc7b08` with the disposable mock agent "r5 fork src" (`d8ef867`). Request counts come from the live `daemon.log` `ws_runtime_metrics` lines (30 s windows, all clients), parsed by `/tmp/r5-metrics.py`. After the run, PID 34022 exited, the agent and the workspace were archived, and the scratch terminal was killed.

- A commit in the scratch repo shows the success toast at the top center: PASS. Commit (header button) committed `a.txt` as `86a7ccc` after about 19 s (message generation). The toast "Committed" box spans x 1586-1833 device px in a 3420 px window: center 1709.5 against the window center 1710. Its top is 88 device px = 44 pt = header 36 + 8. Evidence: `/tmp/r5-226-01-commit-toast.png`.
- Success toast leaves after about 2.2 s: PASS. Commit toast, zoom-forced polls: visible at 19.6 s and 21.4 s, gone at 22.8 s (coarse). "Stashed changes restored", F15 polls at about 0.5 s: visible for at least 1.69 s and at most 2.69 s.
- Error toast stays about 3.2 s: PASS. "Git command failed: git stash pop ...": visible for at least 2.57 s and at most 3.60 s. The error toast also shows at the top center (`/tmp/r5-226-02-error-toast.png`).
- Note: a second Commit press at about 04:00 sent `checkout_commit_request`, but after 8 min the daemon had sent no reply and the header showed a spinner. The daemon logged "Agent MCP transport error ... Unsupported protocol version: 2026-07-28" at 04:00:06. This looks like the daemon commit message generator, not the toast.
