# TASK-287 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/client/src/core.rs`: `ClientCore::remove_project`, `Pending::ProjectRemove`, and the `project.remove.response` arm.
- `packages/desktop-gpui/crates/client/src/lib.rs`: `Client::remove_project`.
- `packages/desktop-gpui/crates/client/src/model.rs`: `ClientEvent::ProjectRemoveFailed { project_id }`.
- `packages/desktop-gpui/crates/client/tests/rpcs.rs`: the 2 red tests, plus an accepted-removal test and an error-answer test.
- `packages/desktop-gpui/crates/sidebar/src/sidebar.rs`: three new project menu items, the confirm, the `removing` guard with a 10 s timeout, `OpenProjectSettings`, and 2 unit tests.
- `packages/desktop-gpui/crates/app/src/shell.rs`: `open_project_settings` opens Settings, or keeps the open one, and calls `Settings::open_project`.
- `packages/desktop-gpui/crates/timeline/src/lib.rs` and `packages/desktop-gpui/crates/client/examples/client_probe.rs`: one arm each for the new `ClientEvent` variant, because their matches are exhaustive.

## Contract

- Menu order: Open project settings, Copy path, Open in Finder, plugin items, Remove project.
- Remove shows the Electron confirm ("Remove project?", `Remove "<name>" from the sidebar?` + blank line + `Files on disk will not be changed.`, "Remove" danger, "Cancel"). Cancel sends nothing.
- Confirm sends one `project.remove.request` with the row's project id. A second confirm while the id is in `removing` returns before `remove_project`.
- `accepted: true` with no error drops the project from `project_list` and `empty_projects`, marks the tree dirty, and the row leaves. Nothing is dropped before the answer.
- `accepted: false`, an error, or a failed send pushes `ClientEvent::ProjectRemoveFailed`. The sidebar clears the guard and shows "Failed to remove some workspaces". The row stays.
- The guard also clears when the project leaves `client.projects()`, or after 10 s.
- Open in Finder runs `open <root>` on the background executor. On failure it shows "Couldn't open folder".

## Verified

- Red: `./cargo.sh test -p bachuc-client --test rpcs project_remove` -> compile error before any edit (`remove_project` and `ProjectRemoveFailed` missing). RED as expected.
- Green: same command -> 4 passed (2 red tests, accepted removal, error answer).
- `./cargo.sh build --workspace --all-targets` -> Finished, no error. It found the 2 exhaustive matches listed under Changed.
- `./cargo.sh test -p bachuc-sidebar` -> 0 failures (21 lib, 5 electron_order, 1 fake_daemon, 55 model, 32 plugin).
- `./cargo.sh test -p bachuc-client` -> 0 failures (rpcs 81, sync 22, others).
- `./cargo.sh fmt --check -p bachuc-sidebar -p bachuc-client -p bachuc-app -p bachuc-timeline` -> exit 0.
- Code read: the `ProjectRemoveFailed` handler calls `push_notification(REMOVE_FAILED)` and clears `removing`; `start_remove` returns before `remove_project` through `begin_remove`.
- Screen checks (menu list, Cancel and Remove, Open project settings, Open in Finder) -> PASS; see UI Checks.
- GitNexus impact on `ClientEvent` returned `risk: UNKNOWN` with a stale index (15 commits behind). It was confirmed with `rg` and the workspace build instead.
- Timing tests failed under load (load average 11-13) and passed when the load fell. Both `building_500_*` (Known Failures) and `sync one_thousand_workspace_updates_stay_cheap` (1,000 updates, bound 45 ms; 57 ms at load 11, 41-44 ms at load 6). The change adds no code to that path.

## Anti-goal

- Tripwire: no existing project menu item lost.
- Baseline before the first edit (`./cargo.sh test -p bachuc-sidebar`, 2026-10-21 16:38): 19 lib + 5 + 1 + 55 + 32 passed, 0 failed. The code listed Copy path and the plugin items (`sidebar.rs`, kebab builder).
- At completion (2026-10-21 16:52): 21 lib (+2 new) + 5 + 1 + 55 + 32 passed, 0 failed. The menu code keeps Copy path and `add_plugin_items`, with 3 added items. Items lost = 0.
- Lead read, UI check 2026-10-21 17:12: the menu showed Copy path and the plugin item Move to. Items lost = 0. Result: PASS.

## Questions

- The red tests as written call `connect`, which waits for an agent in the directory. Without `server.add_agent("a1", "idle")` all four tests failed with "connects". Agent added `server.add_agent("a1", "idle");` as the first line after `FakeServer::start()` in both red tests. No assertion changed.

## Lessons

- `Harness::connected` waits for at least one agent, so a client test needs `server.add_agent(..)` before `connect`. Put it in a Red tests block.
- A new `ClientEvent` variant broke exhaustive matches in `crates/timeline/src/lib.rs` and `crates/client/examples/client_probe.rs`, not in `crates/app/src/state.rs`.

## UI Checks

Run by the lead with `cua_repl` on the check bundle, connected to live (6768), 2026-10-21 17:10-17:25.

- Open the `wt-ui-proj` project menu -> PASS. Items in order: Open project settings, Copy path, Open in Finder, Move to: Workspace 1 (plugin), Remove project. Items lost = 0.
- Open project settings -> PASS. Settings opened at Projects with `wt-ui-proj` selected.
- Open in Finder -> PASS. A Finder window titled `wt-ui-proj` opened at `/private/tmp/wt-ui-proj`.
- Remove project, then Cancel -> PASS. The confirm showed the Electron title, message, and buttons, and the row stayed.
- Remove project, then Remove -> PASS. The row left the sidebar, `npm run cli -- project ls` no longer listed it, and `/tmp/wt-ui-proj` still had its commit.
- Note: the daemon kept the worktree folder `.dev/paseo-home/worktrees/01lg0gqg/wt-ui-x` after the remove. That is daemon behavior, not this TASK.
