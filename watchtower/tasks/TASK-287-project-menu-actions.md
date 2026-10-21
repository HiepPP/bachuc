# TASK-287 Project menu settings, Finder, and Remove

Group: B (writes the `bachuc-sidebar`, `bachuc-client`, and `bachuc-app` shell files below; no file shared with Group A)
Class: code

## Brief

Goal: The GPUI sidebar project menu gets "Open project settings", "Open in Finder", and "Remove project", with the same logic as the Electron project menu.

Change: menu has Copy path and plugin items -> Open project settings, Copy path, Open in Finder, plugin items, Remove project.

Design: `ProjectMenuItems` and `handleRemoveProject` in [packages/app/src/components/sidebar-workspace-list.tsx](packages/app/src/components/sidebar-workspace-list.tsx), and [packages/app/src/projects/project-remove.ts](packages/app/src/projects/project-remove.ts).

Boundaries:

- Remove project shows a destructive confirm first. Title "Remove project?". Message `Remove "<project name>" from the sidebar?` then a blank line, then `Files on disk will not be changed.`. Ok "Remove" with the danger variant, cancel "Cancel". These are the Electron strings in [packages/app/src/i18n/resources/en.ts](packages/app/src/i18n/resources/en.ts).
- On confirm, send `{"type": "project.remove.request", "projectId": <id>, "requestId": <id>}` as `ProjectRemoveRequestSchema` in [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) defines it. No protocol change.
- The daemon removes the project and its workspaces. It emits workspace `remove` updates with `removedProjectId`, but no project update (`project.remove.request` handler in [packages/server/src/server/session.ts](packages/server/src/server/session.ts)).
- Trap: the `workspace_update` `remove` branch in `ClientCore::handle` in [core.rs](packages/desktop-gpui/crates/client/src/core.rs) ignores `removedProjectId`, and the project tree also lists `project_list` and `empty_projects`. So the row stays as an empty project unless the client drops it. Planner call: on a `project.remove.response` with `accepted: true`, drop that project from `project_list` and `empty_projects`, rebuild the tree, and mark the directory changed. Do not drop the row before the answer.
- IF `project.remove.response` has `accepted: false` or an `error`, show the notification "Failed to remove some workspaces", the Electron `removeFailed` text.
- Ignore a second Remove for the same project while one is pending, as the `archiving` set guards archive in [packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs).
- Open project settings opens Settings at the Projects section for this project root, through `Settings::open_project` in [packages/desktop-gpui/crates/settings/src/settings.rs](packages/desktop-gpui/crates/settings/src/settings.rs).
- Open in Finder runs `open <root>` off the UI thread, as `Explorer::reveal` in [packages/desktop-gpui/crates/explorer/src/explorer.rs](packages/desktop-gpui/crates/explorer/src/explorer.rs) does. On failure, show "Couldn't open folder", the Electron `openFolderFailed` text.
- Planner call: GPUI talks to one host at a time, so Remove targets only the row's project id on that host. The Electron multi-host loop and its "Update the host" gate are left out, because release and live both run this fork with `projectRemove`.
- Non-goal: "Open in new window". GPUI is one window.
- Trap: `kebab` menu builder for the project row in `sidebar.rs` runs each time the menu opens. Clone the row data into the closure; do not borrow `self`.
- Trap: `Pending::Action` in [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) only logs an error and never reads `accepted`. Add a `Pending` variant for project remove that reads `accepted` and `error`.
- Planner call: on a failed answer, `ClientCore` pushes `ClientEvent::ProjectRemoveFailed { project_id }` into its `events`, as it does for `PermissionRequested`. The sidebar reacts to that event.
- Trap: other code matches on `ClientEvent`, for example `crates/app/src/state.rs` and `crates/client/examples/client_probe.rs`. A new variant can break an exhaustive match; `./cargo.sh build --workspace --all-targets` finds each one. The pending guard clears on that event, on the project leaving the tree, or after a timeout, as `ARCHIVE_TIMEOUT` does for archive.
- Trap: `Shell::open_settings` in [shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) returns early when Settings is already open. Call `open_project` on the existing `self.settings` entity in that case too.
- Trap: a new `SidebarEvent` variant breaks the exhaustive match on it in `shell.rs`, so both files change in one commit.
- Open in Finder uses `open <root>`, not the `open -R` that `Explorer::reveal` uses, so the folder itself opens.

How:

- In `core.rs`, add `ClientCore::remove_project(&mut self, project_id: &str)`, and handle `project.remove.response`.
- In [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) and [packages/desktop-gpui/crates/client/src/model.rs](packages/desktop-gpui/crates/client/src/model.rs), add `Client::remove_project(&mut self, project_id: &str, cx: &mut Context<Client>)` and `ClientEvent::ProjectRemoveFailed { project_id: String }`.
- In `sidebar.rs`, add the three items to the project kebab menu, the confirm dialog, the pending guard, and a `SidebarEvent::OpenProjectSettings { root, name }`.
- In [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs), handle `OpenProjectSettings` by opening Settings and calling `open_project`.

Files:

- [packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) (menu items, confirm, pending guard, new event)
- [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) (send `project.remove.request`, read `project.remove.response`)
- [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) (public `remove_project`)
- [packages/desktop-gpui/crates/client/src/model.rs](packages/desktop-gpui/crates/client/src/model.rs) (new `ClientEvent::ProjectRemoveFailed` variant)
- [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs) (red tests)
- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (open Settings at the project)

Expected result:

- WHEN Hiep opens a project menu THE menu SHALL list Open project settings, Copy path, Open in Finder, the plugin items, then Remove project.
- WHEN Hiep picks Remove project THE sidebar SHALL show the confirm with the Electron title, message, and buttons.
- WHEN Hiep confirms THE client SHALL send one `project.remove.request` with the row's project id.
- WHEN the daemon accepts THE project row and its workspaces SHALL leave the sidebar, and no file on disk SHALL change.
- IF Hiep cancels THEN no request SHALL go out.
- IF the response has `accepted: false` or an error THEN the sidebar SHALL show "Failed to remove some workspaces" and keep the row.
- IF Hiep confirms Remove again while a request for that project is pending THEN no second request SHALL go out.
- WHEN Hiep picks Open project settings THE app SHALL show Settings at the Projects section for that project.
- WHEN Hiep picks Open in Finder THE project folder SHALL open in Finder.

Red tests:

File: [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs)

```rust
// Uses the file's existing `connect`, `sent`, `WAIT`, `FakeServer`, and `json!`.
// Add `ClientEvent` to the file's `use bachuc_client::{...}` list.
#[test]
fn remove_project_sends_the_project_remove_request() {
    let server = FakeServer::start();
    let mut harness = connect(&server);
    harness.core.remove_project("p1");
    let request = sent(&server, &mut harness, "project.remove.request");
    assert_eq!(request["projectId"], "p1");
    assert!(request["requestId"].as_str().is_some_and(|id| !id.is_empty()));
}

#[test]
fn a_refused_project_remove_is_reported() {
    let server = FakeServer::start();
    server.reply(
        "project.remove.request",
        "project.remove.response",
        json!({"projectId": "p1", "accepted": false, "removedWorkspaceIds": [], "error": "busy"}),
    );
    let mut harness = connect(&server);
    harness.core.remove_project("p1");
    assert!(harness.pump_until(WAIT, |harness| {
        harness.events.contains(&ClientEvent::ProjectRemoveFailed {
            project_id: "p1".to_string(),
        })
    }));
}
```

Anti-goal: the project menu keeps every item it has today; items lost = 0 (Copy path and every plugin item still show); tripwire; read from the `bachuc-sidebar` tests and a screen check before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client --test rpcs project_remove` -> both red tests pass.
- A client test in [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs) seeds project `p1` with one workspace, answers `accepted: true` with the workspace `remove` update, and asserts `p1` is gone from `projects()` -> passes.
- `cd packages/desktop-gpui && ./cargo.sh build --workspace --all-targets` -> no error from the new `ClientEvent` or `SidebarEvent` variant.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-sidebar` -> 0 failures.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client` -> 0 failures.
- Screen check on live (needs the GPUI app and a screen, per Screen checks in [docs/gpui.md](docs/gpui.md)): open a project kebab -> Open project settings, Copy path, Open in Finder, the plugin items such as "Move to", then Remove project.
- Screen check on live: add a throwaway project for a temp folder, pick Remove project, then Cancel -> the row stays; pick Remove project, then Remove -> the row leaves the sidebar and the temp folder still exists on disk.
- Screen check on live: pick Open project settings -> Settings shows the Projects section for that project.
- Screen check on live: pick Open in Finder -> a Finder window shows the project folder.
- Code read: the `ProjectRemoveFailed` handler in `sidebar.rs` calls `push_notification` with "Failed to remove some workspaces" and clears the pending guard -> present. A live failure needs a daemon error and is not forced.
- Code read: a second confirm while the project id is in the pending set returns before `remove_project` -> present; the pending set clears after the timeout.
