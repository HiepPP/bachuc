# Plan Context

## Shared Context

- GPUI sorts sidebar projects and threads by name only, in `sorted` in [model.rs](packages/desktop-gpui/crates/sidebar/src/model.rs). Electron applies a saved order on top of that name sort. This gap is why the owner sees the GPUI order as random.
- Electron's rules, which this plan ports:
  - Name sort: [workspace-structure.ts](packages/app/src/projects/workspace-structure.ts) `buildWorkspaceStructureProjects`. GPUI already matches it.
  - Saved order on top: `composeWorkspaceStructure` and `applyStoredOrdering` in [selectors.ts](packages/app/src/stores/session-store-hooks/selectors.ts). An item that is not in the saved order keeps its slot. The saved items fill their own slots in saved order.
  - Seeding: `computeSidebarOrderUpdates` in [sidebar-workspaces-view-model.ts](packages/app/src/hooks/sidebar-workspaces-view-model.ts). Missing project keys go at the end. Missing thread keys go at the start, in name order. Electron never prunes saved keys.
  - Pinned: `splitPinnedSidebarGroups` in [use-sidebar-pins.ts](packages/app/src/hooks/use-sidebar-pins.ts). It sorts by `pinnedAt`, newest first, then applies `pinnedWorkspaceOrder`.
  - Drag: `handleProjectDragEnd` and `handleWorkspaceReorder` in [sidebar-workspace-list.tsx](packages/app/src/components/sidebar-workspace-list.tsx). They use `mergeWithRemainder` and `hasVisibleOrderChanged` from [sidebar-reorder.ts](packages/app/src/utils/sidebar-reorder.ts).
- Keys match Electron exactly, so the import is a copy:
  - A project key is its view key, which `Facts.project_keys` holds (`Client::project_view_key`).
  - A thread key is `<serverId>:<workspaceId>`. The server id comes from `server_info()`: `Client` returns the typed `ServerInfo` with `server_id`, and `ClientCore` returns JSON with `serverId`.
- The order lives in `sidebar.json` in `bachuc_command::data_dir()` (`~/Library/Application Support/BachucGpui`), per [ADR-0049](watchtower/decisions/ADR-0049-gpui-own-sidebar-json-over-shared-stores.md). It uses Electron's JSON field names `projectOrder`, `workspaceOrderByProject`, and `pinnedWorkspaceOrder`.
- Status grouping keeps its own sort (`compare_status_rows`). The saved order applies only to project grouping and to the pinned section.
- The other thread's client, composer, settings, timeline, vendor, and `Cargo.lock` changes were committed in `c17427731` on 2026-10-20. The tree was clean at run start, so `Cargo.lock` changes come only from TASK-279.

## Decisions

- [ADR-0057](watchtower/decisions/ADR-0057-gpui-saved-sidebar-order-with-electron-import-over-name-sort.md): a saved order with Electron's rules, a one-time Electron import through `rusty-leveldb`, and drag for projects, threads, and pinned rows.

## References

- [docs/gpui.md](docs/gpui.md): run cargo only through `./cargo.sh` in `packages/desktop-gpui`, and read the screen check rules.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-sidebar`
- After the tests, run `./cargo.sh clean --profile dev` when no other `cargo` or `rustc` process runs.
