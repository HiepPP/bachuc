# TASK-231 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/app/src/shell.rs`: explorer and PR notices go to toasts; the chat target rule and its sync to the Explorer; Add to chat attaches a workspace-file chip; file queries answer with `set_file_rows` and `set_file_error`; `view_script_terminal`; the `scripts_restart` field and the restart hook on `ScriptsChanged`; 2 new tests.
- `packages/desktop-gpui/crates/app/src/chrome.rs`: the Scripts menu row in Electron's order (View service, View terminal, Restart, Stop or Run), the URL row, `restart_script`, `restart_stopped_scripts`, `script_url`, and 1 new test.
- `packages/desktop-gpui/crates/app/src/state.rs`: the `WorkspaceScript` test literal has the 3 proxy URL fields.

## Contract

- Consumes, by exact name: `ExplorerNoticeEvent` and `PullRequestNoticeEvent` (each calls `Shell::show_toast(ok, message, cx)`), `Explorer::set_chat_target`, `FileResult { path, icon }`, `CommandCenter::set_file_rows`, `CommandCenter::set_file_error`, and the `WorkspaceScript` proxy URL fields. `bachuc_explorer::file_icons::file_icon` gives each file row its icon, from the last path segment.
- `PullRequestNoticeEvent` has its own subscription. The `PullRequestEvent` match stays exhaustive. `create_workspace` and `on_client_event` did not change.
- `chat_target(focused, group_actives, is_chat)` is a pure rule after Electron's `resolveFocusedChatTarget`. It returns the focused tab when it is a chat (agent, lazy agent, or draft). Else it returns the active tab of the first group, in reading order, that is a chat. `sync_chat_target` calls `Explorer::set_chat_target` from `sync_plugin_active` (select, tab activated, tab closed, host switch), from the end of `put_pane` (a tab replaced in place emits no `Activated`), and when the Explorer is made.
- Safe default: GPUI tabs record no parent tab, so Electron's middle step (the chat a file was opened beside) is skipped.
- Add to chat: a lazy agent tab is materialized first. The attachment is `workspace_file_attachment(path)`, with label = the file name and JSON `{"type":"text","mimeType":"text/plain","title":<name>,"text":"Workspace file: <path>"}`. The path is the Explorer's workspace-relative path, as in Electron. Then the tab is activated and focused.
- Safe default: `WorkspaceDraftPane` keeps its composer private, and `workspace_draft.rs` is outside this TASK's files. A draft target therefore gets the chip through `WorkspaceDraftPane::seed_fork(None, label, attachment, ..)`, which only calls `Composer::add_raw_attachment`.
- Files: a failed query calls `set_file_error(Some(..))` and shows no toast. A missing cwd answers `set_file_rows(vec![])`. Answers to older queries, or to a closed center, are still dropped.
- Scripts: `script_url` picks `proxy_url`, then `public_proxy_url`, then `local_proxy_url`, and only for a running service. A URL shows as a "View <name> service" eye button and as a URL row under the header (the label has no protocol). Both call `cx.open_url`. "View <name> terminal" shows only while the script runs and has a `terminal_id`. It activates the open terminal tab with that id, or attaches a new tab with `TerminalView::attach`. "Restart <name>" kills the terminal and records `workspace_id/name`. The next `ScriptsChanged` that lists the script as not running starts it again. Every button has `Role::Button`, an aria label, and an `AccessibleAction::Click` handler (through `pressable`).
- Not done (matches Electron, outside the spec): no Copy URL button, no route chooser, no Direct `localhost:<port>` URL (`WorkspaceScript` has no port).

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app` -> 79 passed. New: `the_chat_target_is_the_focused_chat_else_the_first_group_with_a_chat`, `a_workspace_file_attaches_as_electron_text_attachment`, `a_running_service_opens_its_proxy_url_first`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> builds. The only warning existed before (`next_theme`, `state.rs:424`).
- `./cargo.sh fmt --check -p bachuc-app` -> exit 0.
- `./cargo.sh clippy -p bachuc-app --tests` -> no warning in `shell.rs`, `chrome.rs`, or `state.rs`.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- GitNexus impact (the index is 71 commits stale): `sync_plugin_active` CRITICAL and `put_pane` CRITICAL. Each got one appended `sync_chat_target` call and kept its signature. `add_file_to_chat`, `query_files`, and `script_row` were not in the index. `rg` shows each of them has callers only in `shell.rs` and `chrome.rs`.
- UI check: a forced branch switch error shows a toast at the top center -> DEFERRED (UI checker). Top-center placement belongs to TASK-226.
- Owner hand test: Add to chat with an agent tab and with a draft tab focused -> DEFERRED (owner hand test).
- UI check: a service script shows its URL row and View terminal -> DEFERRED (UI checker).
- Anti-goal (no new request type after a workspace click) -> DEFERRED (UI checker). By code, `sync_chat_target` reads local state only and sends no request.

## Lessons

- A tab replaced in place keeps its key, so `Workspace::open` emits no `Activated`. Any shell state that depends on what the tabs show must also resync in `put_pane`.
- A private field of a sibling module view (`WorkspaceDraftPane::composer`) is not reachable from `shell.rs`. Reuse a public method that already does the job, or ask for a getter in a TASK that owns that file.
- Closing an attached script terminal tab kills its daemon terminal (`PaneContent::release`), and the kill stops the script. This is the same as for any other terminal tab.

## UI Checks

Run 2026-10-18 by the UI checker (run 5). Bundle `Bachuc GPUI Check` built from HEAD `08064a6f3` (includes `626143dc7` and TASK-225 `699395706`), PID 34022, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). Frames were forced with the AX "zoom the window" action; keys went to the check app only. Scratch repo `/tmp/t202-repo` (branches `feature-x` and `main`, new commit `1f8a4a9` with a `web` service script) was the live workspace `wks_9cd2d5d43fcc7b08` with the disposable mock agent "r5 fork src" (`d8ef867`). Request counts come from the live `daemon.log` `ws_runtime_metrics` lines (30 s windows, all clients), parsed by `/tmp/r5-metrics.py`. After the run, PID 34022 exited, the agent and the workspace were archived, and the scratch terminal was killed.

- A forced branch switch error shows a toast at the top center; the inline notice does not: PASS. See TASK-222 UI Checks (stash pop error toast and the Cancel path).
- The Scripts menu of a running service: PASS. After "Run web script" (`python3 -m http.server $PASEO_PORT`), the row "web script, running" had "View web service", "View web terminal", "Restart web", "Stop web", and the URL row "Open web--feature-x--t202-repo-6e946553.localhost:6768". Evidence: `/tmp/r5-231-01-scripts-running.png`. "View web service" and the URL row were not pressed, because they open the browser in front.
- View terminal: PASS. The press opened a "web" terminal tab with "Serving HTTP on 127.0.0.1 port 53498". Evidence: `/tmp/r5-231-02-view-terminal.png`.
- Restart stops then starts it: PASS. At 04:04:19 the window had `kill_terminal_request` 1 and `start_workspace_script_request` 1. `paseo script ls` at 04:04:23 showed web running, healthy, port 53601, terminal `1776fabf` (before: 53498, `6ddcb82d`). One `http.server` process ran. Note: the open "web" tab stayed on the old terminal and showed "[process exited]".
- Stop: "Stop web" stopped it ("web script, exit 1"; no `http.server` process).
- Anti-goal, no new request type after a workspace click: PASS. A click on t207-ws and back (04:05:43-04:05:46) gave the window 04:06:08: `plugin.rpc.invoke`, `fetch_workspaces`, `checkout_status`, `project.list`, `fetch_agents`, `checkout_pr_status`, `unsubscribe_checkout_diff`, `file_explorer`, `subscribe_checkout_diff`, `checkout.diff.get`, `agent.timeline.set_subscription`, `fetch_agent_timeline`, `agent.provider_subagents.list`, `subscription.release`. Each type is in an earlier workspace open or older log lines.
- Note: the first AX press on the project row "t207-ws" folded the project; a second press unfolded it.
