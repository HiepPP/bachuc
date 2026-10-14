# Plan Context

## Shared Context

- Fleet map is a workspace tab of kind `fleet_map`. It shows the agents of one host as a 3D map. The owner chose this tab kind, not a route and not a plugin.
- The host is the `serverId` of the tab. A tab always has a `serverId`, so the "All machines" case with a null active host never reaches the map. This also follows [ADR-0001](watchtower/decisions/ADR-0001-active-host-over-host-filters.md).
- The map shows every managed agent of that host, grouped by workspace. Subagents link to their parent through `getParentAgentIdFromLabels`. Provider subagents are not in v1.
- v1 is read only. A node has a status color, a pulse while running, a hover label, and a click that opens the agent. There is no camera control, no filter, and no moving particles.
- The host feature lives in `packages/app` because it needs the session store and workspace tabs. See [ADR-0017](watchtower/decisions/ADR-0017-native-host-features-over-thin-host-changes.md).
- Electron and web only. `view.web.tsx` serves both, because Electron is the Metro web platform and the map uses no Electron API. The base `view.tsx` is the native placeholder. See the platform rules in [CLAUDE.md](CLAUDE.md).
- Pure logic stays out of three.js: [packages/app/src/fleet-map/layout.ts](packages/app/src/fleet-map/layout.ts) and [packages/app/src/fleet-map/nodes.ts](packages/app/src/fleet-map/nodes.ts) run in plain vitest.

## Render Budget

- One WebGL context for the whole app, shared by every Fleet map tab. Chrome limits contexts, and the terminal already uses `@xterm/addon-webgl`.
- Lazy load. `import("three")` runs only when a Fleet map view mounts. Nothing else imports `three` statically.
- Render on demand. A frame renders only after data, size, hover, or theme changes.
- The pulse is the only continuous animation. It runs at most 30 frames per second, only while the view is visible, the window is focused, a node is running, and reduced motion is off.
- Cap the device pixel ratio at 1.5. Use instanced meshes for nodes and one line set for edges.

## Rules For Every TASK

- Do not use `useUnistyles()`. It is banned by [docs/unistyles.md](docs/unistyles.md). Pass theme colors into the canvas with `withUnistyles` and `uniProps`.
- The app runs React Compiler. A `useMemo` that reads an external store only through `void` is dropped. See the 2026-10-01 learning in [watchtower/MEMORY.md](watchtower/MEMORY.md).
- Add no RPC and no protocol field. See [docs/protocol-compatibility.md](docs/protocol-compatibility.md).
- Run `npm run typecheck` and `npm run lint -- <files>` after each change. Run only the test files the TASK adds or changes, one at a time.
- Test on live. Do not build a release.

## Decisions

- Deterministic layout over a force simulation. A force simulation needs continuous ticks and breaks the on-demand render budget.
- One shared renderer that each view blits to its own 2D canvas, over one canvas per view. Two Fleet map tabs, main and Explorer, then cost one WebGL context.
- Node color comes from `getStatusDotColor` in [packages/app/src/utils/status-dot-color.ts](packages/app/src/utils/status-dot-color.ts), so the map and the sidebar agree.

## Open Decisions

- [Q-034](watchtower/QUESTIONS.md): accept `three` in the main web bundle if Metro cannot split it into a lazy chunk.

## References

- [docs/explorer-sidebar.md](docs/explorer-sidebar.md)
- [docs/agent-lifecycle.md](docs/agent-lifecycle.md)
- [docs/coding-standards.md](docs/coding-standards.md)
- [packages/app/src/panels/pull-request-panel.tsx](packages/app/src/panels/pull-request-panel.tsx) is the nearest simple panel to copy.
