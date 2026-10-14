# Fleet map

The Fleet map is a workspace tab (`fleet_map`) that draws the agents of one host as a 3D map. Code lives in `packages/app/src/fleet-map/`. It runs on web and Electron; native shows a fallback message.

## Render budget

The map must cost nothing while you ignore it.

- **One WebGL context for the whole app.** Chrome limits contexts per page, and the terminal already holds some through `@xterm/addon-webgl`.
- **Render on demand.** A frame renders only after a data, size, hover, or theme change. Idle means zero frames.
- **The pulse is the only continuous animation.** It marks running agents. It runs at most 30 fps, and only while the view is visible, the window is focused, a node is running, and reduced motion is off.
- **Cap the pixel ratio at 1.5.** Nodes are instanced meshes and edges are one line set.

## Shared renderer

Every view keeps its own 2D canvas. One reference-counted `WebGLRenderer` draws each frame, and the view copies it with `blitToCanvas`. Two tabs, one in the main pane and one in the Explorer, then cost one context. Blit in the same task as the render, because the drawing buffer is not preserved after compositing. Never cache the renderer size, because another view can resize it. See [ADR-0026](../watchtower/decisions/ADR-0026-shared-renderer-blit-over-canvas-per-view.md).

## Lazy import

`view.web.tsx` loads `three`, `shared-renderer`, and `scene` with `import()` when nodes exist. An empty map mounts no canvas and loads nothing. Keep `three` out of static imports: only `shared-renderer.ts` and `scene.ts` may import it.

Metro puts `three` in a shared `__common` chunk that `index.html` loads at page start. It is split from the entry bundle but not lazy at page start. Electron loads it from disk, so the cost is accepted.

## Layout

`layout.ts` is a pure, deterministic function with no `three` and no random numbers. Output does not depend on input order. Workspaces form clusters on a golden-angle spiral, and subagents orbit their parent.

Do not replace it with a force simulation. A simulation needs continuous ticks, and that breaks on-demand rendering. A stable position also lets you find an agent again.

## Data scope

`useFleetNodes(serverId)` reads the session store of one host. A tab always has a `serverId`, so the "All machines" case with a null host never reaches the map ([ADR-0001](../watchtower/decisions/ADR-0001-active-host-over-host-filters.md)). Archived agents are skipped. A node's color comes from `getStatusDotColor`, so the map and the sidebar agree. The map adds no RPC and no protocol field.

## Gotchas

- **React Compiler.** The app runs it. A `useMemo` that reads an external store only through `void` is dropped, so select the data with a store hook.
- **Unistyles.** `useUnistyles()` is forbidden ([unistyles.md](unistyles.md)). Pass colors into the canvas through `withUnistyles` and `uniProps`. The vitest stub makes `withUnistyles` the identity, so test the palette builder, not the wiring.
- **New tab kinds.** The label functions in `workspace-screen.tsx` and `workspace-desktop-tabs-row.tsx` are not exhaustive. A new tab kind falls through to the Agent label with no type error. Add the kind to each function by hand.
