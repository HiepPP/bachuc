# TASK-088 Fleet map tab wiring

Group: C (writes the tab model, layout storage, launcher, panel registry, and locale files)
Class: risky

## Brief

Goal: Add a `fleet_map` workspace tab kind that opens from the New Tab launcher and shows a placeholder panel. Later TASKs fill the panel.

Change: Workspace tabs have no map kind -> `fleet_map` is a singleton tab for hosts `main` and `explorer`, saved and restored with the layout, with a launcher entry and a localized label.

Design: [docs/explorer-sidebar.md](docs/explorer-sidebar.md)

Boundaries:

- Copy the `pull_request` wiring. It is the nearest singleton kind. Search for `pull_request` in `packages/app/src` and mirror each site.
- The kind is a singleton: `resourceKey` is `"fleet_map"` and the tab id is `"fleet_map"`, like `files`.
- Hosts are `["main", "explorer"]`. Panels stay independent of the shell. See the panel host contract in [docs/explorer-sidebar.md](docs/explorer-sidebar.md).
- The saved layout schema in `workspace-layout-storage.ts` is a strict object union. The new literal must be additive. A layout saved before this change must still load unchanged. Release runs on the owner's real saved layouts. See Debugging in [CLAUDE.md](CLAUDE.md).
- Create the base `view.tsx` as the placeholder and native fallback. It exports `FleetMapView({ serverId })`. The panel reads `serverId` from the pane context and passes it in. TASK-090 adds `view.web.tsx` next to it with the same props. Do not add `view.web.tsx` here.
- All nine locales need the new keys, because the parity test in [packages/app/src/i18n/resources.test.ts](packages/app/src/i18n/resources.test.ts) checks them. Add every string the map needs now, so TASK-090 edits no locale file: `panels.fleetMap.label` ("Fleet map"), `.subtitle`, `.empty`, and `.webglUnavailable`. See [docs/i18n.md](docs/i18n.md).
- Use an existing lucide icon through `withUnistyles`, as the pull request panel does.

How:

- Add `{ kind: "fleet_map" }` to `WorkspaceTabTarget` in the model. Add it to `normalizeSimpleWorkspaceTabTarget` and to the tab id function in `identity.ts`.
- Add the literal to the storage schema.
- Add the manifest entry and register `fleetMapPanelRegistration` in `register-panels.ts`.
- Add `fleetMap` to both launch order arrays and to `BUILT_IN_SELECTIONS` and `builtIns` in the launcher.
- Follow typecheck errors for every other exhaustive `target.kind` switch, such as tab labels in `workspace-screen.tsx` and `workspace-desktop-tabs-row.tsx`, and the tab menu.
- Add `panels.fleetMap.*` keys to all nine locale files.
- Add or extend tests for identity and the manifest. Add a new `workspace-layout-storage.test.ts` for storage parsing, because no storage test file exists yet.

Files:

- [packages/app/src/workspace-tabs/model.ts](packages/app/src/workspace-tabs/model.ts) (new target kind)
- [packages/app/src/workspace-tabs/identity.ts](packages/app/src/workspace-tabs/identity.ts) (normalize and tab id)
- [packages/app/src/stores/workspace-layout-storage.ts](packages/app/src/stores/workspace-layout-storage.ts) (schema literal)
- [packages/app/src/stores/workspace-layout-storage.test.ts](packages/app/src/stores/workspace-layout-storage.test.ts) (new: parse tests)
- [packages/app/src/panels/panel-manifest.ts](packages/app/src/panels/panel-manifest.ts) (manifest entry)
- [packages/app/src/panels/register-panels.ts](packages/app/src/panels/register-panels.ts) (register panel)
- [packages/app/src/panels/fleet-map-panel.tsx](packages/app/src/panels/fleet-map-panel.tsx) (new: panel registration)
- [packages/app/src/fleet-map/view.tsx](packages/app/src/fleet-map/view.tsx) (new: placeholder and native fallback)
- [packages/app/src/workspace-tabs/launcher/internal/catalog.ts](packages/app/src/workspace-tabs/launcher/internal/catalog.ts) (launch order)
- [packages/app/src/workspace-tabs/launcher/index.tsx](packages/app/src/workspace-tabs/launcher/index.tsx) (selection and item)
- [packages/app/src/screens/workspace/workspace-screen.tsx](packages/app/src/screens/workspace/workspace-screen.tsx) (tab labels)
- [packages/app/src/screens/workspace/workspace-desktop-tabs-row.tsx](packages/app/src/screens/workspace/workspace-desktop-tabs-row.tsx) (tab labels)
- [packages/app/src/screens/workspace/workspace-tab-menu.ts](packages/app/src/screens/workspace/workspace-tab-menu.ts) (menu rules, only if typecheck requires)
- [packages/app/src/i18n/resources/en.ts](packages/app/src/i18n/resources/en.ts) and the eight other files in `packages/app/src/i18n/resources/` (new keys)

Expected result:

- New Tab lists Fleet map. Choosing it opens a tab with the placeholder.
- The tab survives a reload and is a singleton: choosing it twice focuses the same tab.
- The kind is accepted by `main` and `explorer` hosts.
- A layout saved before this change loads with no change.
- The i18n parity test passes.

Anti-goal: Saved layouts stay compatible. Tripwire: the existing `workspace-layout-store` and `identity` tests keep passing with the same case count or more, and a fixture saved before this change parses to an equal object; read before the first edit and at the end.

## Verify

- `cd packages/app && npx vitest run src/workspace-tabs/identity.test.ts --bail=1` -> pass, with a new fleet_map case.
- `cd packages/app && npx vitest run src/stores/workspace-layout-storage.test.ts --bail=1` -> pass, with a fixture saved before this change and a fleet_map case.
- `cd packages/app && npx vitest run src/stores/workspace-layout-store.test.ts --bail=1 > /tmp/layout-store-test.txt 2>&1` -> exit 0. The file is large, so read the output file.
- `cd packages/app && npx vitest run src/panels/panel-manifest.test.ts --bail=1` -> pass, with a fleet_map case.
- `cd packages/app && npx vitest run src/i18n/resources.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0. Every exhaustive switch handles `fleet_map`.
- `npm run lint -- packages/app/src/panels/fleet-map-panel.tsx packages/app/src/fleet-map/view.tsx packages/app/src/workspace-tabs` -> exit 0.
- Needs a human or a browser session: on live, New Tab shows Fleet map and the placeholder opens in the main pane and in Explorer.
- Anti-goal: record the pass count of the identity and layout-store test files before the first edit and after the last edit; the after count is not lower.
