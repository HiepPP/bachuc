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

## Fix round 1

### Status

BLOCKED. This status replaces the first-build status above. The shell part of the latest-prompt reveal is fixed, and the background launch (lead request) is done. The reveal still cannot show on screen without a change in the timeline crate, which is outside this TASK's Files. The UI checks are DEFERRED (UI checker).

### Cause

The UI checker saw the toast "That message isn't loaded in this chat" and the tail view on every card. Two causes, found by reading:

1. Shell, fixed. The shell subscribes to the client before each `TimelineView`, and GPUI calls subscribers in subscription order (`gpui-pre-0.3.9/src/subscription.rs:20`, a `BTreeMap` by id). On the agent's first `TimelineChanged`, `on_client_event` called `reveal_passage` before the view synced that change. The new view was already `loaded` from the `sync` in `TimelineView::new` (`crates/timeline/src/lib.rs:412`, which sets `loaded = true` at client version 0), with no rows. So `reveal_now` found no row and `reveal_plan` returned `NotLoaded`. The message id was correct: both sides use `message_key`.
2. Timeline crate, the blocker. `reveal_now` (`crates/timeline/src/plugin_view.rs:121`) scrolls with `ListState::scroll_to_reveal_item`, which keeps tail follow on (`gpui-pre-0.3.9/src/elements/list.rs:677-702` does not touch `follow_state`). A new `TimelineView` follows the tail (`crates/timeline/src/lib.rs:304`), and the next layout moves the list back to the end (`list.rs:1043-1049`). `scroll_to_reveal_item` also puts a lower row at the bottom of the view, not at the top. The fix is one line in `reveal_now`: `self.list.scroll_to(ListOffset { item_ix: row, offset_in_item: px(0.) })`. `scroll_to` stops the tail follow (`list.rs:668-670`) and puts the prompt row at the top. The same gap affects every `RevealTimelinePassage` on a list that follows the tail.

### Changed

- `packages/desktop-gpui/crates/app/src/shell.rs`
  - `on_client_event`: a waiting latest-prompt reveal now runs through `cx.defer_in`. The deferred reveal runs after every subscriber of that `TimelineChanged`, so the view has synced the same version.
  - `latest_prompt_reveal`: a state with no entries returns `Wait`, not `Tail`. A permission change can raise the version before the first page arrives, and before this change, that ended the wait with no reveal.
- `packages/desktop-gpui/crates/app/src/main.rs` (lead request): `BACHUC_NO_ACTIVATE=1` skips `cx.activate(true)` and opens the main window with `focus: false`. With `focus: false`, GPUI calls `orderFront_` instead of `makeKeyAndOrderFront_` (`gpui-pre-macos-0.3.9/src/window.rs:1291-1295`). The pure function `activates_on_launch` holds the env rule. The normal launch is unchanged. The other `cx.activate(true)` call (`shell.rs:344`) runs only when the user clicks a system notification. GPUI also activates the app on a file drop (`gpui-pre-0.3.9/src/window.rs:5686`). Both need a user action.

### Contract

- `Shell::open_agent` keeps its signature (for TASK-200).
- Background launch: `BACHUC_NO_ACTIVATE=1` in the app's environment keeps the app in the background. Any other value or no value activates the app, as before. `open` does not pass the shell environment to the app, so the UI checker sets the variable with `open -g --env BACHUC_NO_ACTIVATE=1 "<bundle>"`.

### Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app`: pass, 62 tests. This includes the extended `the_latest_prompt_is_the_last_user_message` (failed first with `Tail` for a version-1 state with no entries, then passed) and the new `only_bachuc_no_activate_1_keeps_the_app_in_the_background`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app`: exit 0. It has 1 warning that was already there (`next_theme` is not used, `state.rs:272`).
- `./cargo.sh fmt --check -p bachuc-app`: exit 0.
- `./cargo.sh metadata --format-version 1 > /dev/null`: exit 0.
- UI check, the latest prompt shows at the top after a card click: BLOCKED. It needs the timeline crate fix above.
- UI check, the toast does not show on a card click: DEFERRED (UI checker).
- UI check, `BACHUC_NO_ACTIVATE=1` launch stays in the background through card clicks: DEFERRED (UI checker).
- Anti-goal, under 300 ms: DEFERRED (UI checker). The fix adds one deferred call per open.

### Lessons

- A shell handler of a `ClientEvent` runs before the handler of each view made later. Code in the shell that reads a view's state for the same event must use `cx.defer_in`.
- `TimelineView` is `loaded` from its first `sync` at client version 0, before any page arrives. So `reveal_passage` right after a view is created reports `NotLoaded` instead of waiting.
- `ListState::scroll_to_reveal_item` does not stop tail follow. On a timeline that follows the tail, a reveal must use `scroll_to`.

### Questions

- May the lead add `packages/desktop-gpui/crates/timeline/src/plugin_view.rs` to this TASK (or a follow-up TASK) for the one-line `scroll_to` change in `reveal_now`? Without it, no reveal shows on a timeline that follows the tail.

## Fix round 2

### Status

DONE. The lead widened this TASK (Q-120) to the timeline and workspace crates. This status replaces the fix round 1 status. The UI checks are DEFERRED (UI checker).

### Changed

- `packages/desktop-gpui/crates/timeline/src/plugin_view.rs:123`: `reveal_now` calls `self.list.scroll_to(ListOffset { item_ix: row, offset_in_item: px(0.) })` instead of `scroll_to_reveal_item(row)`. `scroll_to` stops tail follow (`gpui-pre-0.3.9/src/elements/list.rs:668-670`), so the next layout of a new view no longer moves back to the end. The message row now shows at the top.
- `packages/desktop-gpui/crates/workspace/src/workspace.rs:994`: the tab chip has an `AccessibleAction::Click` handler. It calls `focus_group(group)` and then `activate_tab(group, index)`, as a left mouse-down does through the group's capture mouse-down. This follows the sidebar and composer pattern (`crates/sidebar/src/sidebar.rs:112`). It lets the UI checker press a tab by AX for TASK-199.

### Callers of `reveal_passage`

- `shell.rs:1586`, Board latest prompt: top alignment is the wanted result. The prompt shows at the top and the reply follows below it.
- `shell.rs:1516`, `RevealTimelinePassage`: the message now shows at the top. If the passage text is far down a long message, the text view's `reveal_range` requests an autoscroll (`vendor/gpui-base/src/text/inline.rs:471`). The list honors child autoscroll in its normal prepaint (`list.rs:1289-1326`, called with `autoscroll: true` at `list.rs:1564`), so the passage still scrolls into view. This is from reading, not from a run.
- A reveal now always stops tail follow. New output does not pull the view down after a reveal. The user returns to the tail with the normal scroll-to-end control.

### Contract

- `TimelineView::reveal_passage` keeps its signature. A found message now shows at the top of the view, and tail follow stops.
- The workspace tab chip answers an AX press like a left mouse-down.

### Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-timeline`: pass, 80 tests.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-workspace`: pass, 34 tests.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app`: exit 0. It has 1 warning that was already there (`next_theme` is not used, `state.rs:272`).
- `./cargo.sh fmt --check -p bachuc-timeline` and `-p bachuc-workspace`: exit 0.
- `./cargo.sh metadata --format-version 1 > /dev/null`: exit 0.
- GitNexus impact (index stale, confirmed with `rg`): `reveal_now` LOW, `activate_tab` LOW, `focus_group` HIGH (6 upstream). `focus_group` was not changed, only called, so its callers keep their behavior.
- No unit test was added. The timeline and workspace crates have no GPUI test context (`rg "gpui::test|TestAppContext"` finds none). `reveal_now` needs a `TimelineView` entity with a client, and the AX handler needs a window. A test of `ListState` alone would test GPUI, not this code.
- UI check, a Board card click shows the latest prompt at the top, with no toast: DEFERRED (UI checker).
- UI check, a plugin `RevealTimelinePassage` with text highlights the passage in view: DEFERRED (UI checker).
- UI check, an AX press on a tab activates it: DEFERRED (UI checker).
- Anti-goal, under 300 ms: DEFERRED (UI checker). The change adds no work.

### Lessons

- For an AX press on a workspace tab, the handler must focus the group itself. The group's `capture_any_mouse_down` runs only for a mouse event.

## UI Checks Run 2

Run 2026-10-18 by the UI checker. Bundle `Bachuc GPUI Check` built from HEAD `da7231026` (includes `8819e6d89` and `7c066ec26`) plus uncommitted group C client files. PID 80366, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1`, connected only to live `127.0.0.1:6768` (`lsof`). The app stayed in the background for the whole run (`lsappinfo front` kept the owner's release app in front). A background GPUI window does not draw, so the checker forced each frame with the window's AX "zoom the window" action (a resize, no activation) and read the AX tree after a no-op F15 key. Disposable agents on live: "t198r2 running" (`558d897`, stopped after the run), "t199 tab A/B/C", "t209 ...", "t207 prose check", and "Reply with the single word ok" (`f23565a`).

- Running card click: PASS. Board closed, tab "t198r2 running" active, the timeline list starts at "User message: start bench" with the turn below it, no toast in the AX tree. A read 5 s later during the stream still started at the prompt, so tail follow did not pull the view down. Evidence: `/tmp/t198r2-02-running-open.png`.
- Finished card click ("t207 prose check", other workspace): PASS. Board closed, the prompt bubble is at the top of the view, no toast. Evidence: `/tmp/t198r2-03-finished-open.png`.
- Star: FAIL. The star is not drawn anywhere inside the card at 100% and 160% Board size; a pixel scan of the repo row's right end finds no glyph (`/tmp/t198r2-06-star-crop.png`, `/tmp/t198r2-08-star-click-pos.png`). It should show at 0.35 opacity (`card.rs:442-445`), so it is still laid out outside the card's clip. Its exact position cannot be read: cua gives no AX frames. A press by element index and a true AX "Press" both left the Board open with no tab, but the star did not toggle (the label stays "Star t199 tab A"), so neither press reached it. The Board `button()` helper (`plugins/board/src/card.rs:146-159`) has no `AccessibleAction::Click` handler, so AX presses on star, pencil, and Remove do nothing.
- Sidebar workspace press with the Board open ("t207-ws"): PASS. Board closed, workspace t207-ws shows with its tab and timeline. Evidence: `/tmp/t198r2-10-sidebar-workspace.png`.
- Anti-goal, click to first timeline paint under 300 ms: UNVERIFIED. The app logs no click or paint timestamps, and a background window paints only when forced. Press plus AX read took 807-848 ms, which includes cua round trips.
- Cleanup: PID 80366 quit and exited.
