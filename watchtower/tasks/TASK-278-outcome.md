# TASK-278 Outcome

## Status

DONE (code). No UI line in this spec.

## Changed

- `packages/desktop-gpui/crates/sidebar/src/model.rs`: `SidebarOrder` (+ `is_empty`), `ViewState.order`, `Facts.server_id`, `TreeSource::server_id` (`Client` from typed `ServerInfo.server_id`, `ClientCore` from `server_info()["serverId"]`), `Facts::read` fills it, `workspace_key`, private `apply_stored_ordering` (port of `applyStoredOrdering`), `seed_order` (port of `computeSidebarOrderUpdates`), `name_sorted` (the old `sorted`) and a new `sorted(projects, facts, order)`, pinned order after the `pinnedAt` sort in `build_rows`.
- `packages/desktop-gpui/crates/sidebar/src/prefs.rs`: `projectOrder`, `workspaceOrderByProject` (written with sorted keys), `pinnedWorkspaceOrder`; a bad field gives an empty value; `sample()` has `order`; new test `the_saved_order_round_trips_and_a_bad_field_gives_empty`.
- `packages/desktop-gpui/crates/sidebar/src/sidebar.rs`: `refresh` calls `seed_order` with the full `client.projects()` right after `Facts::read`, and `save_prefs` runs only when it returns true.
- `packages/desktop-gpui/crates/sidebar/tests/model.rs`: the eight red tests, plus one timing probe test for the anti-goal.

## Contract

- Produces `bachuc_sidebar::model::SidebarOrder { project_order: Vec<String>, workspace_order_by_project: HashMap<String, Vec<String>>, pinned_workspace_order: Vec<String> }` with `Clone, Debug, Default, PartialEq, Eq` and `is_empty`.
- Produces `ViewState.order: SidebarOrder`, `Facts.server_id: Option<String>`, `workspace_key(server_id, workspace_id) -> String`, `seed_order(&mut SidebarOrder, &[ProjectNode], &Facts) -> bool`.
- Produces `sidebar.json` keys `projectOrder`, `workspaceOrderByProject`, `pinnedWorkspaceOrder`.
- `seed_order` reads projects in name order (`name_sorted`). It skips a project with no entry in `Facts.project_keys`. Without a server id it still seeds `project_order` and seeds no thread. It never prunes. A new thread key goes first, a new project key goes last.
- `sorted` applies thread order first, then project order, as Electron's `composeWorkspaceStructure` does. Status grouping is unchanged: `compare_status_rows` is a total order (it ends on the id), so the input order does not matter there. The pinned order applies in both groupings, because the pinned section exists in both.
- Seeding runs on every `refresh`, but only inside the branch that reads facts. An empty project list changes nothing.

## Verified

- Red: before any other edit the eight tests from the spec plus the probe failed to compile (`SidebarOrder`, `seed_order`, `Facts.server_id`, `ViewState.order` missing; E0422, E0560, E0609, E0433, E0425). Counts as red.
- `./cargo.sh test -p bachuc-sidebar --test model` -> 52 passed (43 older, 8 red tests, 1 probe), 0 failed. A run with the default thread count failed the three `building_500_*` timing tests under load (2.1, 2.2, 2.4 ms against 2 ms). They pass alone and with 2 threads (1.0 to 1.3 ms). This is the same load flake as the Known Failures timing tests, not a change effect.
- `./cargo.sh test -p bachuc-sidebar --lib` -> 15 passed, includes the prefs round trip and the bad-field test.
- `git diff -U0 packages/desktop-gpui/crates/sidebar/tests/model.rs | rg '^-\s+\['` -> no match: no expected row list removed (the diff has only added lines).
- `./cargo.sh clippy -p bachuc-sidebar --no-deps --lib --test model -- -D warnings -A clippy::too_many_arguments` -> clean. Without the allow, `kebab` in `sidebar.rs` fails `too_many_arguments` (8/7). It has the same signature in `HEAD`, so it is older than this TASK. `bachuc-protocol`'s `build.rs` also fails `-D warnings` (`unnecessary_unwrap`), and `tests/plugin.rs:228` fails `unnecessary_literal_unwrap`. Both are older and not touched. The spec line `clippy -p bachuc-sidebar -- -D warnings` therefore cannot pass as written (UNSATISFIABLE); the replacement is the command above.

## Anti-goal

`build_rows` on `large_tree()` (50 projects x 10 workspaces, project mode, debug, median of 21):

| Checkpoint                                                      | Value             | Source                                                                     | Time             |
| --------------------------------------------------------------- | ----------------- | -------------------------------------------------------------------------- | ---------------- |
| before, no saved order                                          | 1.148 ms          | `building_500_workspaces_in_project_mode_stays_under_2_ms` before any edit | 2026-10-20 14:48 |
| after, no saved order                                           | 1.087 to 1.119 ms | probe test, 4 runs                                                         | 2026-10-20       |
| after, full reversed saved order (every project and thread key) | 1.267 to 1.269 ms | probe test, 2 runs                                                         | 2026-10-20       |

Full order is 1.10 times the before value and 1.16 times the same-run no-order value; the limit is 1.5. The first implementation used a `HashMap` per list and measured 1.62 ms (1.46 times); the tripwire (a per-item scan of the saved order) did not apply, but a sorted vector with a binary search per saved key removed the hashing cost. PASS.

## Lessons

- A small `HashMap` or `HashSet` per list costs more than a sorted `Vec` in a debug build (the 2026-10-17 memory again): 0.5 ms of the 1.6 ms for 50 lists of 10.
- `cargo clippy -p <crate> -- -D warnings` also lints workspace path dependencies and fails on `bachuc-protocol/build.rs`; use `--no-deps`.
- `sorted` took `&Facts` with a named lifetime because the project key borrows from the facts.
- `refresh` seeds before the rows build, so a new thread shows first in the same frame.

## Group-level run

One run for TASK-278, TASK-279, and TASK-280 together (2026-10-20 14:59 to 15:00, after the last TASK, `CARGO_BUILD_JOBS=4`):

- `./cargo.sh test -p bachuc-sidebar` -> lib 19, `electron_order` 5 (1 ignored), `fake_daemon` 1, `model` 55, `plugin` 32 passed. One run of the `model` suite failed `building_500_workspaces_in_status_mode_stays_under_2_ms` under load (the suite runs on several threads). Alone it passes at 1.32 and 1.35 ms, and a rerun of the whole suite passes. Status grouping code is unchanged by the group (`sorted` returns at once with an empty order).
- `./cargo.sh check -p bachuc-app -p bachuc-composer` -> finished, no error. These crates use `bachuc-sidebar`.
- `./cargo.sh fmt --check -p bachuc-sidebar` -> exit 0.
- No `cargo clean` was run; the lead does it.

## Lead Check

2026-10-20 15:02, lead re-check after the builder report:

- `./cargo.sh test -p bachuc-sidebar --test model --test electron_order --lib` -> 55, 5 (1 ignored), and 19 passed.
- `git diff -U0 crates/sidebar/tests/model.rs | rg '^-\s+\['` -> no match.
- `./cargo.sh fmt --check -p bachuc-sidebar` -> exit 0.
- `./cargo.sh clippy -p bachuc-sidebar -- -D warnings` fails only in `crates/protocol/build.rs` and `crates/theme/src/motion.rs`, and `--no-deps` fails only on `kebab()`, which has 8 arguments at HEAD. The builder's `--no-deps ... -A clippy::too_many_arguments` command is clean. UNSATISFIABLE as written; the replacement passes.
- Anti-goal: lead read of `apply_stored_ordering` in `model.rs`. It sorts the keyed items once and finds each saved key by binary search, so no per-item scan of the saved order. Builder read 1.10x, under 1.5x. PASS.
- Committed in `9cc6e3d50`.
