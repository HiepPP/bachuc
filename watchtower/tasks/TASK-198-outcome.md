# TASK-198 outcome

## Status

DONE. The UI checks are DEFERRED (UI checker).

## Changed

- `packages/desktop-gpui/crates/app/src/shell.rs`
  - `open_workspace` calls the new `close_page_surface` before `select`. `open_agent` goes through `open_workspace`, so it also closes a page. An overlay stays open.
  - `open_agent` now finds the workspace with `agent_workspace`. This function uses the tree first. If the tree has no match, it uses the agent's own `AgentSummary.workspace_id`, but only if the tree lists that workspace. If both fail, nothing happens.
  - `OpenAgentAt { LatestPrompt }` opens the agent and then calls `reveal_latest_prompt`. That function finds the last loaded `user_message` with `latest_prompt_reveal` and calls `TimelineView::reveal_passage`. Two cases wait in the `pending_prompt` field for the agent's next `ClientEvent::TimelineChanged`: a tab that is still lazy (it is built on the activation event, after the current update), and a timeline that has not loaded (`version == 0`). With no user message, the tail view stays.
  - `pending_prompt` is cleared by each `open_agent` and by `reconnect`. It is set only when a tab for the agent exists.
  - Three pure functions with tests: `closes_surface_on_open`, `agent_workspace`, `latest_prompt_reveal`.

## Contract

- `Shell::open_agent(&mut self, agent_id: &str, window: &mut Window, cx: &mut Context<Self>)` keeps its signature. It now closes a `Page` surface and has the `workspace_id` fallback (for TASK-200).
- `OpenAgentAt { Latest }` behaves as `OpenAgent` did before.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app`: pass, 61 tests. This includes `only_a_page_surface_closes_when_an_agent_or_workspace_opens`, `an_agent_opens_in_the_tree_workspace_else_its_own`, and `the_latest_prompt_is_the_last_user_message`.
- `./cargo.sh fmt --check -p bachuc-app`: exit 0.
- `./cargo.sh metadata --format-version 1 > /dev/null`: exit 0.
- Read check: in `plugins/board/src/card.rs:91-96`, each card action calls `cx.stop_propagation()`. So star, pencil, and Remove do not reach `CardAction::Open`.
- `/solve` live reproduction: DEFERRED (UI checker). The cause came from reading the code instead: the page surface hid the workspace, and nothing closed it.
- UI check, 2 cards (running and finished): DEFERRED (UI checker).
- UI check, star and Remove keep the Board open: DEFERRED (UI checker).
- UI check, sidebar workspace click with the Board open: DEFERRED (UI checker).
- Owner hand test: DEFERRED (owner).
- Anti-goal, under 300 ms from click to first paint: DEFERRED (UI checker). The change adds no work to the open path, only one scan of the loaded entries.

## Lessons

- `state.rs` has the test `the_shell_refreshes_logos_only_from_the_theme_observer`. It searches `shell.rs` for the exact text `ClientEvent::TimelineChanged { .. }` in `on_client_event`. Keep that early-return line as it is, and put new `TimelineChanged` handling before it.
- `Shell::activate_key` builds a lazy agent tab only when the pane activation event arrives, after the current update. If code runs right after `open_agent`, `agent_timeline` returns `None` for a tab that was lazy. That code must wait, for example for the next `TimelineChanged`. The `RevealTimelinePassage` arm has the same gap and still has it.
- Every caller of `open_workspace` now closes a page surface. These callers include workspace navigate prev/next, `open_pending`, the plugin panel open, and the workspace changes open.

## UI Checks

Run 2026-10-18 by the UI checker. Bundle `Bachuc GPUI Check` built from HEAD `72fefd064` plus the uncommitted group C client files, PID 82578, connected only to live `127.0.0.1:6768`. Live had no running agent, so the checker made 2 disposable mock agents on live: "t198 running card" (`b39b61a9`, thirty-minute stream, stopped after the checks) and "t198 remove card" (`4508f134`, ten-second stream). Each made its own workspace in `/tmp/gpui-bench`. The app came to the front by itself after the second card click (`cx.activate(true)` at startup in `crates/app/src/main.rs:62`), not from a checker action.

- Card click, running card: PASS for the open, FAIL for the latest-prompt anchor. Board closed, tab "t198 running card" active, timeline list shows. The view is the tail (Step 17) with the toast "That message isn't loaded in this chat". Evidence: `/tmp/t198-02-running-open.png`.
- Card click, finished card "edit-diffs bench": PASS for the open, FAIL for the anchor. Board closed, tab "edit-diffs bench" active, timeline shows the tail (Step 2 and the recap), no reveal after a second read. Evidence: `/tmp/t198-03-finished-open.png`, `/tmp/t198-04-finished-later.png`.
- Card click, finished 10-second card: same result. The prompt is surely loaded in this short thread, but the toast "That message isn't loaded in this chat" shows and the view stays on the tail. Evidence: `/tmp/t198-05-short-open.png`. Cause not diagnosed: `reveal_latest_prompt` finds an id that `TimelineView::reveal_passage` does not have.
- Remove: PASS. Remove on the disposable card kept the Board open, the card left the Board ("1 running, 1 finished"), and no agent tab opened. Evidence: `/tmp/t198-09-after-remove.png`.
- Star: UNVERIFIED. The Star button is laid out past the card's right edge and clipped; the pencil glyph before it is half-cut (`/tmp/t198-07-crop.png`). An index press lands outside the card: on the running card it hit the next card and opened "t198 remove card" (`/tmp/t198-06-after-star.png`); on the remove card it hit empty space and the star did not toggle (`/tmp/t198-08-after-star-remove-card.png`). This clip is a separate Board layout bug, possibly from `72fefd064`.
- Sidebar workspace click with the Board open: UNVERIFIED (unattended): background frame. After the press on "turn-step-check", 3 reads over about 20 s showed the same frame and the same "6m 40s" duration while Zen was in front (`/tmp/t198-10-sidebar-workspace.png`, `/tmp/t198-11-sidebar-workspace-retry.png`). Code read: `SidebarEvent::OpenWorkspace` calls `open_workspace`, which calls `close_page_surface` (`shell.rs:664`).
- Anti-goal, click to first timeline paint under 300 ms: UNVERIFIED. The app logs no click or paint timestamps, and `open -g` drops its stderr. The AX press plus the next snapshot took 782 ms, 825 ms, and 1525 ms; these include cua round trips, so they are not a paint time.
- Cleanup: PID 82578 quit and exited. The owner's app (PID 37065) and live stayed up.
