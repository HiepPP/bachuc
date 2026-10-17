# TASK-216 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/sidebar/src/sidebar.rs`: `SidebarNavEvent` and its `EventEmitter` impl; the New workspace and History nav rows through a shared `builtin_nav_row` (Search uses it too, same style and pitch); the per-project "+" button `new_workspace_button`, placed with the kebab in one trailing group.
- `packages/desktop-gpui/crates/sidebar/src/lib.rs`: re-export `SidebarNavEvent`. Required dep fix: `mod sidebar` is private, so the app could not name the type without it.

## Contract

- Produces: `SidebarNavEvent::OpenHistory`, emitted by the History row.
- Produces: `SidebarNavEvent::NewWorkspace { project_id: Option<String> }`. The New workspace row emits `None`; a project "+" emits `Some(project.id)`.
- `SidebarEvent` is unchanged, so the app match in `shell.rs` still builds. Until TASK-200 and TASK-203 subscribe, the new rows do nothing.
- Nav order: New workspace, History, Search, then plugin rows. This is Electron's default order (`BUILTIN_SIDEBAR_NAV_IDS` in `packages/app/src/sidebar-nav/model.ts`). The spec's Change line lists Search before History; the Boundaries line "Match Electron's order" wins.
- The "+" is 24 px, 15 px glyph, 2 px before the kebab, pulled 6 px onto the rail (`projectTrailingActions`). It fades in with `opacity` on row group hover, never `display`. The project title padding grows by 26 px so the title truncates before the "+". Row height is unchanged (`ROW_HEIGHT` 36, `NAV_ROW_MIN` unchanged).
- Each new pressable has `Role::Button`, an aria label, and the shared `on_press` (click plus `AccessibleAction::Click`).
- Safe default: no shortcut hint on New workspace. The GPUI sidebar has no shortcut-hint rendering and no keymap access; Search shows none either.
- Safe default: the "+" shows on every project. GPUI `ProjectRow` has no project kind, so Electron's `worktreeTarget` gate cannot apply.

## Verified

- `./cargo.sh build -p bachuc-sidebar` -> exit 0.
- `./cargo.sh test -p bachuc-sidebar` -> 2 + 1 + 22 + 24 + 0 passed.
- `./cargo.sh clippy -p bachuc-sidebar` -> no warnings in the sidebar crate.
- `./cargo.sh fmt --check -p bachuc-sidebar` -> exit 0.
- `./cargo.sh build -p bachuc-app` -> exit 0 (one pre-existing `next_theme` unused warning in `crates/app/src/state.rs`).
- `./cargo.sh metadata --format-version 1` -> exit 0.
- UI check, nav rows in order and hover "+" on a project: DEFERRED (UI checker).
- Anti-goal, 3 existing rows keep their height on a screenshot: DEFERRED (UI checker).

## Lessons

- The sidebar crate exports its event types from `lib.rs`; a new event type needs a `pub use` there, or the app cannot subscribe.
- TASK-200 and TASK-203: subscribe with `cx.subscribe(&sidebar, |shell, _, event: &SidebarNavEvent, cx| ...)`.
