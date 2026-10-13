# TASK-090 Outcome

## Status

DONE (2026-10-13T11:15Z). Live checks are UNVERIFIED (unattended).

## Changed

- `packages/app/src/fleet-map/view.web.tsx` (new: web and Electron view)
- `packages/app/src/fleet-map/view.test.tsx` (new: 14 cases)

## Contract

- `FleetMapView({ serverId })` has the same props as the base `view.tsx`. It reads `useFleetNodes(serverId)`.
  - No nodes: shows `panels.fleetMap.empty` and mounts no canvas, so no import and no WebGL work.
  - Nodes: mounts a raw `<canvas>` inside the view. A spinner shows while the lazy chunk loads. `panels.fleetMap.webglUnavailable` shows when loading or `acquireSharedRenderer()` rejects. Nothing throws.
- Theme: `buildFleetPalette(theme)` calls `getStatusDotColor({ showDoneAsInactive: true })` for each bucket and parses the hex to a number. The edge color is `foregroundMuted`. `withUnistyles` plus `uniProps` passes the palette. The mapping caches one palette per theme object, so an unchanged theme never invalidates the scene.
- `useFleetMapDriver({ nodes, palette, onOpenNode, loadRuntime?, frames? })` holds all wiring and takes a fake runtime and fake frame source in tests. `loadRuntime` and `frames` are read once at mount.
  - `loadFleetMapRuntime()` runs `import("three")`, `import("./shared-renderer")`, and `import("./scene")`. `draw(canvas)` calls `scene.renderWith(renderer)` then `blitToCanvas` in the same task (ADR-0026). The view never caches the renderer size. `dispose()` disposes the scene and releases the shared renderer.
  - The scheduler gets `invalidate()` on graph change (nodes or theme), on resize, and on hover change. `setPulsing(true)` only when a node has bucket `running`. Visible comes from `document.visibilityState`, focus from `document.hasFocus()` and window `focus`/`blur`, reduced motion from `matchMedia("(prefers-reduced-motion: reduce)")`.
  - Pulse phase is `(time % 1600) / 1600`.
  - Resize: the canvas bitmap is CSS size times the pixel ratio capped by `runtime.maxPixelRatio` (1.5), and `scene.resize` gets the CSS size.
  - Hover: `onPointerMove` and `onPointerLeave` on the canvas call `scene.pick`, set the hover on the scene, and show a DOM overlay (title, workspace name) at the pointer. The overlay has `pointerEvents="none"`, so it never traps hover. The overlay moves only when the hovered node changes. Click opens `navigateToAgent({ serverId, agentId })`.
  - Unmount: the scheduler is disposed, listeners and the observer are removed, and the runtime is released. A runtime that resolves after unmount is released at once.
- The panel and locale files are untouched.

## Verified

- Checkpoint `view-test`: `cd packages/app && npx vitest run src/fleet-map/view.test.tsx --bail=1` -> PASS (14) FAIL (0), 2026-10-13T11:14Z. RED first: the first run failed on missing React global and a missing `ResizeObserver.unobserve`, both test setup.
- Checkpoint `typecheck`: `npm run typecheck` -> exit 0, 2026-10-13T11:13Z.
- Checkpoint `lint`: `npm run lint -- packages/app/src/fleet-map` -> 0 warnings, 0 errors.
- `npm run format:check:files -- <both files>` -> all matched files use the correct format.
- `rg -n "useUnistyles" packages/app/src/fleet-map` -> no match.
- `rg -n 'import\("\./(scene|shared-renderer)"\)' packages/app/src/fleet-map/view.web.tsx` -> lines 135 and 136, both present.
- Static `three` imports in `packages/app/src` (non-test): only `shared-renderer.ts:1` (value) and `scene.ts:1` (type only). `view.web.tsx` has none.
- Browser, live, three or more agents plus one subagent: colored nodes, link, hover label, click opens the agent, blurred window stops the pulse -> UNVERIFIED (unattended).
- Chrome performance panel shows no repeated frames after the pulse stops -> UNVERIFIED (unattended). The scheduler test and the blur/hidden/reduced-motion driver tests show 0 frames, but not in a real browser.
- Two tabs share one WebGL context -> UNVERIFIED (unattended). The pool test (TASK-089) shows one renderer for two acquires. Each view acquires once.

## Lazy chunk (Q-034 answer)

Command: `cd packages/app && npx expo export --platform web --output-dir /tmp/fleet-export` -> exit 0 in 11 s.

- `scene-*.js` (2,524 B) and `shared-renderer-*.js` (675 B) are separate async chunks.
- `WebGLRenderer` is not in the entry bundle `index-*.js` (21,917,063 B raw). It sits in `__common-*.js` (834,044 B raw), which is the Metro shared chunk of the two async chunks.
- `index.html` loads `__common-*.js` with a `<script defer>` tag. So `three` is split from the entry but not lazy: it loads at page start. In Electron it loads from disk, so Q-034 default applies.
- No baseline was measured. A baseline needs a checkout or stash of the base commit, which this run may not do. The entry bundle size above is the drift gauge starting value.

## Anti-goal

Tripwire: `git diff --name-only` lists only the TASK Files plus `watchtower/` plan files.

- Builder read: `git status --short` shows `watchtower/NEXT.md` (modified before this run) and the two new files. Source: git, time 2026-10-13T11:14:46Z. The export wrote only to `/tmp/fleet-export`.
- Lead read: `git status --short` before the commit lists only `view.web.tsx`, `view.test.tsx`, this outcome, and `watchtower/NEXT.md`. Source: git, time 2026-10-13T11:15:38Z.
- Lead re-check: `npx vitest run src/fleet-map --bail=1` -> 6 files, 58 tests pass. `npm run typecheck` -> exit 0. `npm run lint -- packages/app/src/fleet-map` -> 0 warnings, 0 errors.
- Lead read of the export in `/tmp/fleet-export`: `WebGLRenderer` is in `__common-*.js` (814.5K) and `shared-renderer-*.js`, not in `index-*.js`. `index.html` loads `__common-*.js` with a script tag. Confirms the builder's finding.
- Result: PASS.

## Lessons

- The unistyles test stub makes `withUnistyles` the identity, so `uniProps` is ignored in vitest. Test the palette builder and the driver hook directly. Test `FleetMapView` only for the empty state.
- jsdom has no `PointerEvent`: `fireEvent.pointerMove` drops `clientX`. Dispatch `new MouseEvent("pointermove", { clientX, clientY })` instead.
- `document.hasFocus()` is false in jsdom. Stub it to true, or the pulse never starts.
- The vitest setup has no automatic cleanup. Call `cleanup()` in `afterEach`.
- The classic JSX transform needs `vi.stubGlobal("React", React)` in this repo's jsdom tests.
- A `satisfies` fake keeps its `Mock` types only if the helper returns the inferred type. A parameter typed `FleetMapRuntime` loses `.mock`.
- Lint rule `react-perf` flags inline functions and objects in JSX props, and `promise/always-return` flags `.then` callbacks with a bare `return`. Hoist style objects and use an async function with try/catch.
- The theme `fontSize` scale has no `xs`.
