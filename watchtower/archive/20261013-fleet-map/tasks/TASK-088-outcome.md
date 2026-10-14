# TASK-088 Outcome

## Status

DONE

## Changed

- `packages/app/src/workspace-tabs/model.ts`: `{ kind: "fleet_map" }` in `WorkspaceTabTarget`.
- `packages/app/src/workspace-tabs/identity.ts`: normalize, equality, and tab id `"fleet_map"`.
- `packages/app/src/workspace-tabs/identity.test.ts`: `fleet_map` case in the utility panel identity test.
- `packages/app/src/stores/workspace-layout-storage.ts`: additive `z.literal("fleet_map")` strict object.
- `packages/app/src/stores/workspace-layout-storage.test.ts` (new): pre-change fixture, `fleet_map` tab, extra-field rejection.
- `packages/app/src/panels/panel-manifest.ts` and `panel-manifest.test.ts`: hosts `main` and `explorer`, resource key `fleet_map`.
- `packages/app/src/panels/fleet-map-panel.tsx` (new): `fleetMapPanelRegistration`, lucide `Network` icon through `withUnistyles`.
- `packages/app/src/panels/register-panels.ts`: registers the panel.
- `packages/app/src/fleet-map/view.tsx` (new): `FleetMapView({ serverId })` placeholder and native fallback.
- `packages/app/src/workspace-tabs/launcher/internal/catalog.ts` and `index.tsx`: `fleetMap` last in both launch orders, selection, and item `fleet-map`.
- `packages/app/src/workspace-tabs/launcher/internal/catalog.test.ts`: expected orders include `fleetMap`. Not in the spec Files list; the order change breaks it otherwise.
- `packages/app/src/screens/workspace/workspace-screen.tsx`, `workspace-desktop-tabs-row.tsx`: `fleetMap` fallback labels.
- `packages/app/src/screens/workspace/workspace-tab-menu.ts`: close button test id `workspace-fleet_map-close` (typecheck required it).
- `packages/app/src/i18n/resources/{en,ar,es,fr,ja,ko,pt-BR,ru,zh-CN}.ts`: `panels.fleetMap.label`, `subtitle`, `empty`, `webglUnavailable`.

## Contract

- Target: `{ kind: "fleet_map" }`. Tab id and resource key are `"fleet_map"`; `panelResourceKey` gives `"fleet_map:fleet_map"`.
- `FleetMapView` lives in `@/fleet-map/view` with `export interface FleetMapViewProps { serverId: string }`. TASK-090 adds `view.web.tsx` with the same export names and props. The panel reads `serverId` from `usePaneContext()`.
- The base view shows `panels.fleetMap.webglUnavailable`, because it is the native fallback.
- Launcher item id is `fleet-map`, `toggleTarget: null`, always visible where the host allows it.
- English strings: label "Fleet map", subtitle "Agents on this host as a map", empty "No agents on this host yet", webglUnavailable "The fleet map needs WebGL, which this view does not support."

## Verified

- `cd packages/app && npx vitest run src/workspace-tabs/identity.test.ts --bail=1` -> PASS 16 (new `fleet_map` case)
- `cd packages/app && npx vitest run src/stores/workspace-layout-storage.test.ts --bail=1` -> PASS 3 (pre-change fixture parses to an equal object; `fleet_map` case)
- `cd packages/app && npx vitest run src/stores/workspace-layout-store.test.ts --bail=1 > /tmp/layout-store-test.txt 2>&1` -> exit 0, PASS 134
- `cd packages/app && npx vitest run src/panels/panel-manifest.test.ts --bail=1` -> PASS 3 (new `fleet_map` case)
- `cd packages/app && npx vitest run src/i18n/resources.test.ts --bail=1` -> PASS 36
- `cd packages/app && npx vitest run src/workspace-tabs/launcher/internal/catalog.test.ts --bail=1` -> PASS 2
- `cd packages/app && npx vitest run src/screens/workspace/workspace-pane-content.test.tsx --bail=1` -> PASS 3
- `npm run typecheck` -> exit 0
- `npm run lint -- <all changed files> packages/app/src/workspace-tabs` -> exit 0, 0 warnings, 0 errors
- `npm run format:check:files -- <all changed files>` -> all files use the correct format
- Live (Electron from `npm run dev:desktop`, daemon 6768, 2026-10-14T02:50Z): New Tab lists Fleet map, and choosing it opens a `Fleet map` tab in the main pane -> PASS. The tab then showed the real map from TASK-090, not the placeholder. The Explorer host was not opened by hand -> UNVERIFIED.

## Anti-goal

Saved layouts stay compatible.

| Checkpoint                      | Before (10:58:40Z) | After (11:03:44Z) | Source                        |
| ------------------------------- | ------------------ | ----------------- | ----------------------------- |
| identity.test.ts pass count     | 15                 | 16                | vitest output                 |
| workspace-layout-store.test.ts  | 134                | 134               | `/tmp/layout-store-test.txt`  |
| panel-manifest.test.ts          | 2                  | 3                 | vitest output                 |
| Pre-change fixture parses equal | no test existed    | PASS              | workspace-layout-storage test |

No tripwire crossed. Final: PASS.

## Lessons

- In zsh, `$FILES` holding several paths is not word-split; use an array and `"${FILES[@]}"`. Passed as one argument, `npm run lint` showed an ESLint config error through the output wrapper, and oxfmt found no target file.
- Adding a launch order entry breaks `launcher/internal/catalog.test.ts`, which is outside the spec Files list; the spec should list it.
- The fallback label functions in `workspace-screen.tsx` and `workspace-desktop-tabs-row.tsx` are not exhaustive: a new kind silently falls through to the Agent label. Only `workspace-tab-menu.ts` failed typecheck.
- Sibling changes in `packages/app/package.json` and `package-lock.json` were present during the run; typecheck stayed green with them.
