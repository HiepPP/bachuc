# ADR-0057 GPUI saved sidebar order with a one-time Electron import over name sort

Status: accepted
Date: 2026-10-20
Supersedes: -
Scope: `packages/desktop-gpui/crates/sidebar/**`

## Context

- GPUI sorts projects and threads by name only ([model.rs](packages/desktop-gpui/crates/sidebar/src/model.rs) `sorted`); its doc comment says the drag order is not ported.
- Electron applies a saved order on top of the name sort ([selectors.ts](packages/app/src/stores/session-store-hooks/selectors.ts) `composeWorkspaceStructure`). New threads go first and new projects go last ([sidebar-workspaces-view-model.ts](packages/app/src/hooks/sidebar-workspaces-view-model.ts) `computeSidebarOrderUpdates`). Drag changes the order.
- Electron keeps that order in Chromium LevelDB at `~/Library/Application Support/Bachuc/Local Storage/leveldb`, key `_paseo://app\x00\x01sidebar-project-workspace-order`, value `\x01` plus Latin-1 JSON `{"state":{"projectOrder","workspaceOrderByProject","pinnedWorkspaceOrder"},"version":N}`. Checked on 2026-10-20.
- The value crosses a LevelDB log block boundary, so a plain byte scan cannot read it. A real LevelDB reader is needed.
- The Electron keys already match GPUI: project view keys come from `Client::project_view_key`, and thread keys are `<serverId>:<workspaceId>`.
- [ADR-0049](watchtower/decisions/ADR-0049-gpui-own-sidebar-json-over-shared-stores.md) keeps GPUI sidebar choices in `sidebar.json` and rejected reading Electron's stores on an ongoing basis.

## Decision

- GPUI saves the project, thread, and pinned order in `sidebar.json` with Electron's keys and Electron's rules: name sort, then the saved order, new threads first, new projects last, and drag to reorder.
- GPUI imports Electron's saved order once, with the `rusty-leveldb` crate on a temp copy of the LevelDB folder. After that, `sidebar.json` owns the order.
- Hiep chose this on 2026-10-20, in the `/watchtower new` interview.

## Consequences

- On the first launch after the change, GPUI shows the order that Electron showed.
- The sidebar crate gets one new dependency, `rusty-leveldb`.
- The two apps do not stay in step after the import. A drag in one app does not show in the other, as ADR-0049 says.

## Options Considered

- Name sort only: rejected. Hiep sees it as random, because Electron shows the saved order.
- Sort by the latest activity: rejected. It does not match Electron, and the order jumps while agents run.
- Saved order without an import: rejected. The first launch seeds old threads by name, so it differs from Electron until Hiep drags each row.
- One manual import by an agent at implement time: rejected. It covers only one machine and one data folder.
- Read Electron's store on every launch: rejected in ADR-0049. The stores are client-local, and the apps must not overwrite each other.

## Revisit If

- Electron is removed from the machine, or its saved order is older than the GPUI order. Then the import code can go.
- Hiep wants the order to follow him between GPUI and Electron, or between machines.
