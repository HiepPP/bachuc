# TASK-278 GPUI sidebar saved order with Electron's rules

Group: A (shared files: `crates/sidebar/src/model.rs`, `crates/sidebar/src/prefs.rs`, `crates/sidebar/src/sidebar.rs`, with TASK-279 and TASK-280)
Class: code

## Brief

Goal: The GPUI sidebar lists projects, threads, and pinned rows in a saved order, with Electron's rules. New threads go first and new projects go last.

Change: name sort only -> name sort, then the saved order from `sidebar.json`, seeded with new keys on each refresh.

Design: [watchtower/CONTEXT.md](watchtower/CONTEXT.md)

Boundaries:

- Port Electron's rules exactly. Each rule and its source file are in [watchtower/CONTEXT.md](watchtower/CONTEXT.md). Use Electron's key formats, so TASK-279 can copy the import with no conversion.
- With an empty saved order, the rows stay the same as today. The existing tests in [crates/sidebar/tests/model.rs](packages/desktop-gpui/crates/sidebar/tests/model.rs) keep their expected values.
- Seed from the full client project list, not the list after the project or label filter. Electron seeds from all projects.
- Never prune saved keys. Electron keeps them, so a thread that comes back keeps its place. `retain_projects` in [prefs.rs](packages/desktop-gpui/crates/sidebar/src/prefs.rs) must not touch the order.
- Status grouping keeps `compare_status_rows`. Do not apply the saved order there.
- Trap: `prune_stale_prefs` in [sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) runs once and returns early while `projects()` is empty. Seeding must also wait for a non-empty project list. If it seeds an empty or partial list, existing threads later come in as "new" and jump to the top.
- Trap: without a server id, a thread key cannot be built. Then skip the thread order and seeding for that refresh, and keep the name order.
- Planner call: seed on every `refresh`. Save `sidebar.json` only when `seed_order` returns true, so a refresh with no new key writes nothing.
- Planner call: without a server id, still seed `project_order`, because project keys need no server id.
- Planner call: skip a project that has no entry in `Facts.project_keys`. It keeps its name slot and is not seeded.
- Planner call: seed new thread keys in name order. Seed pinned threads too, because Electron seeds every thread of the project.
- Trap: the `sample()` literal in the prefs tests in [prefs.rs](packages/desktop-gpui/crates/sidebar/src/prefs.rs) names every `ViewState` field, so it needs `order`. `to_json` must write `workspaceOrderByProject` with sorted keys, so `the_same_prefs_write_the_same_text` still passes.
- `Client::server_info()` returns the typed `ServerInfo`, whose `server_id` is a `String` (see `crates/protocol/tests/fixtures.rs:93`).
- Planner call: no "workspace list loaded" flag is added to the client. After the first seed, a reconnect only adds threads that are new.

How:

- In [model.rs](packages/desktop-gpui/crates/sidebar/src/model.rs), add `SidebarOrder`, `ViewState.order`, `Facts.server_id`, and `TreeSource::server_id`. Implement `server_id` for `Client` from the typed `ServerInfo.server_id`, and for `ClientCore` from `server_info()["serverId"]`. Fill `Facts.server_id` in `Facts::read`.
- Add `workspace_key`, `apply_stored_ordering` (a port of `applyStoredOrdering`), and `seed_order` (a port of `computeSidebarOrderUpdates`).
- Make `sorted` take `&Facts` and `&SidebarOrder`. After the name sort, apply `project_order` by view key, and `workspace_order_by_project[view_key]` by thread key.
- In `build_rows`, after the stable `pinnedAt` sort, apply `pinned_workspace_order` by thread key.
- In [prefs.rs](packages/desktop-gpui/crates/sidebar/src/prefs.rs), read and write `projectOrder`, `workspaceOrderByProject`, and `pinnedWorkspaceOrder`. A bad field gives an empty value, as other fields do.
- In `Sidebar::refresh` in [sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs), call `seed_order` with the full `client.projects()` and the facts. Call `save_prefs` when it returns true.
- Update the `sorted` doc comment, which says the drag order is not ported.

Files:

- [packages/desktop-gpui/crates/sidebar/src/model.rs](packages/desktop-gpui/crates/sidebar/src/model.rs) (order types, helpers, `sorted`, pinned order)
- [packages/desktop-gpui/crates/sidebar/src/prefs.rs](packages/desktop-gpui/crates/sidebar/src/prefs.rs) (the order fields in `sidebar.json`)
- [packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) (seed and save in `refresh`)
- [packages/desktop-gpui/crates/sidebar/tests/model.rs](packages/desktop-gpui/crates/sidebar/tests/model.rs) (red tests)

Interfaces:

- Produces: `bachuc_sidebar::model::SidebarOrder { pub project_order: Vec<String>, pub workspace_order_by_project: HashMap<String, Vec<String>>, pub pinned_workspace_order: Vec<String> }`, which derives `Clone, Debug, Default, PartialEq, Eq`, with `SidebarOrder::is_empty(&self) -> bool`, for TASK-279 and TASK-280
- Produces: `ViewState.order: SidebarOrder` for TASK-279 and TASK-280
- Produces: `Facts.server_id: Option<String>` and `bachuc_sidebar::model::workspace_key(server_id: &str, workspace_id: &str) -> String`, for TASK-280
- Produces: `bachuc_sidebar::model::seed_order(order: &mut SidebarOrder, projects: &[ProjectNode], facts: &Facts) -> bool` for TASK-279
- Produces: `sidebar.json` keys `projectOrder: string[]`, `workspaceOrderByProject: Record<string, string[]>`, and `pinnedWorkspaceOrder: string[]`, for TASK-279

Expected result:

- WHEN the saved order lists project and thread keys THE sidebar SHALL show those projects and threads in the saved order.
- WHEN a project or thread is not in the saved order THE sidebar SHALL keep its name-sort slot.
- WHEN a refresh sees a thread key that is not saved THE sidebar SHALL put it first in its project's saved order.
- WHEN a refresh sees a project key that is not saved THE sidebar SHALL put it last in the saved project order.
- WHEN the saved pinned order lists pinned threads THE pinned section SHALL show them in that order, after the `pinnedAt` sort.
- IF the saved order is empty THEN THE sidebar SHALL show the same rows as before this TASK.
- IF the server id is unknown THEN THE sidebar SHALL keep the name order for threads and SHALL NOT seed thread keys.

Red tests:

File: [packages/desktop-gpui/crates/sidebar/tests/model.rs](packages/desktop-gpui/crates/sidebar/tests/model.rs)

```rust
/// Facts with a server id and a view key `key-<id>` for each project.
fn keyed_facts(projects: &[ProjectNode]) -> Facts {
    let mut facts = Facts {
        server_id: Some("srv".into()),
        ..Facts::default()
    };
    for project in projects {
        facts
            .project_keys
            .insert(project.id.clone(), format!("key-{}", project.id));
    }
    facts
}

fn strings(keys: &[&str]) -> Vec<String> {
    keys.iter().map(|key| key.to_string()).collect()
}

#[test]
fn a_saved_order_puts_projects_and_threads_in_its_order() {
    let projects = [
        project(
            "a",
            vec![workspace("a1", false, &[]), workspace("a2", false, &[])],
        ),
        project("b", vec![workspace("b1", false, &[])]),
    ];
    let facts = keyed_facts(&projects);
    let mut state = ViewState::default();
    state.order.project_order = strings(&["key-b", "key-a"]);
    state
        .order
        .workspace_order_by_project
        .insert("key-a".into(), strings(&["srv:a2", "srv:a1"]));
    let rows = build_rows_with(&projects, &[], &facts, &state);
    assert_eq!(ids(&rows), ["title", "p:b", "w:b1", "p:a", "w:a2", "w:a1"]);
}

#[test]
fn an_unsaved_thread_keeps_its_name_slot() {
    let projects = [project(
        "a",
        vec![
            workspace("a1", false, &[]),
            workspace("a2", false, &[]),
            workspace("a3", false, &[]),
        ],
    )];
    let facts = keyed_facts(&projects);
    let mut state = ViewState::default();
    // a2 is not saved, so it keeps slot 2; a3 and a1 fill slots 1 and 3.
    state
        .order
        .workspace_order_by_project
        .insert("key-a".into(), strings(&["srv:a3", "srv:a1"]));
    let rows = build_rows_with(&projects, &[], &facts, &state);
    assert_eq!(ids(&rows), ["title", "p:a", "w:a3", "w:a2", "w:a1"]);
}

#[test]
fn seeding_puts_new_threads_first_and_new_projects_last() {
    let projects = [
        project(
            "a",
            vec![workspace("a1", false, &[]), workspace("a2", false, &[])],
        ),
        project("b", vec![workspace("b1", false, &[])]),
    ];
    let facts = keyed_facts(&projects);
    let mut order = SidebarOrder {
        project_order: strings(&["key-b"]),
        ..SidebarOrder::default()
    };
    order
        .workspace_order_by_project
        .insert("key-a".into(), strings(&["srv:a1"]));
    assert!(seed_order(&mut order, &projects, &facts));
    assert_eq!(order.project_order, strings(&["key-b", "key-a"]));
    assert_eq!(
        order.workspace_order_by_project["key-a"],
        strings(&["srv:a2", "srv:a1"])
    );
    assert_eq!(order.workspace_order_by_project["key-b"], strings(&["srv:b1"]));
    assert!(
        !seed_order(&mut order, &projects, &facts),
        "a second seed with no new key changes nothing"
    );
}

#[test]
fn seeding_keeps_keys_that_are_gone() {
    let projects = [project("a", vec![workspace("a1", false, &[])])];
    let facts = keyed_facts(&projects);
    let mut order = SidebarOrder {
        project_order: strings(&["key-gone", "key-a"]),
        ..SidebarOrder::default()
    };
    order
        .workspace_order_by_project
        .insert("key-a".into(), strings(&["srv:gone", "srv:a1"]));
    assert!(!seed_order(&mut order, &projects, &facts));
    assert_eq!(order.project_order, strings(&["key-gone", "key-a"]));
}

#[test]
fn without_a_server_id_threads_keep_the_name_order_and_are_not_seeded() {
    let projects = [project(
        "a",
        vec![workspace("a1", false, &[]), workspace("a2", false, &[])],
    )];
    let mut facts = keyed_facts(&projects);
    facts.server_id = None;
    let mut state = ViewState::default();
    state
        .order
        .workspace_order_by_project
        .insert("key-a".into(), strings(&["srv:a2", "srv:a1"]));
    let rows = build_rows_with(&projects, &[], &facts, &state);
    assert_eq!(ids(&rows), ["title", "p:a", "w:a1", "w:a2"]);
    let mut order = SidebarOrder::default();
    seed_order(&mut order, &projects, &facts);
    assert!(order.workspace_order_by_project.is_empty());
    assert_eq!(order.project_order, strings(&["key-a"]), "projects still seed");
}

#[test]
fn a_project_without_a_view_key_keeps_its_name_slot_and_is_not_seeded() {
    let projects = [
        project("a", vec![workspace("a1", false, &[])]),
        project("b", vec![workspace("b1", false, &[])]),
        project("c", vec![workspace("c1", false, &[])]),
    ];
    let mut facts = keyed_facts(&projects);
    facts.project_keys.remove("b");
    let mut state = ViewState::default();
    state.order.project_order = strings(&["key-c", "key-a"]);
    let rows = build_rows_with(&projects, &[], &facts, &state);
    assert_eq!(ids(&rows), ["title", "p:c", "w:c1", "p:b", "w:b1", "p:a", "w:a1"]);
    let mut order = SidebarOrder::default();
    seed_order(&mut order, &projects, &facts);
    assert_eq!(order.project_order, strings(&["key-a", "key-c"]));
}

#[test]
fn status_grouping_ignores_the_saved_order() {
    let projects = [project(
        "a",
        vec![workspace("a1", false, &[]), workspace("a2", false, &[])],
    )];
    let facts = keyed_facts(&projects);
    let plain = ViewState {
        grouping: Grouping::Status,
        ..ViewState::default()
    };
    let mut ordered = plain.clone();
    ordered
        .order
        .workspace_order_by_project
        .insert("key-a".into(), strings(&["srv:a2", "srv:a1"]));
    assert_eq!(
        ids(&build_rows_with(&projects, &[], &facts, &ordered)),
        ids(&build_rows_with(&projects, &[], &facts, &plain))
    );
}

#[test]
fn a_saved_pinned_order_applies_after_the_pinned_at_sort() {
    let projects = [project(
        "a",
        vec![workspace("p1", true, &[]), workspace("p2", true, &[])],
    )];
    let mut facts = keyed_facts(&projects);
    facts
        .pinned_at
        .insert("p1".into(), "2026-10-09T00:00:00.000Z".into());
    facts
        .pinned_at
        .insert("p2".into(), "2026-10-01T00:00:00.000Z".into());
    let mut state = ViewState::default();
    state.order.pinned_workspace_order = strings(&["srv:p2", "srv:p1"]);
    let rows = build_rows_with(&projects, &[], &facts, &state);
    assert_eq!(ids(&rows)[..3], ["pinned", "w:p2", "w:p1"]);
}
```

Add a prefs round-trip test next to the existing prefs tests: a `SidebarPrefs` with a non-empty `view.order` goes through `to_json` and `parse` unchanged, and a file with `"projectOrder": 5` parses to an empty `project_order`.

Anti-goal: `build_rows` on the `large_tree()` fixture in [crates/sidebar/tests/model.rs](packages/desktop-gpui/crates/sidebar/tests/model.rs) stays within 1.5 times its time before this TASK, with a saved order that lists every key. Read it from a timing test or a `std::time::Instant` probe before and after the change. Tripwire: a lookup that scans the saved order once per item (O(n^2)).

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-sidebar --test model` -> the eight red tests pass, and every test that existed before passes with unchanged expected values.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-sidebar --lib` -> the prefs round-trip test passes.
- `git diff -U0 packages/desktop-gpui/crates/sidebar/tests/model.rs | rg '^-\s+\['` -> no removed expected row lists.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-sidebar -- -D warnings` -> no warnings.
- Anti-goal: `build_rows` time on `large_tree()` with a full saved order, before and after -> after is at most 1.5 times before.
- After the checks, run `./cargo.sh clean --profile dev` when no other `cargo` or `rustc` process runs.
