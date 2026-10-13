# TASK-089 Outcome

## Status

DONE

## Changed

- `packages/app/package.json`: adds `three` `^0.186.1` and `@types/three` `^0.186.0` (installed by npm, not pinned by hand).
- `package-lock.json`: npm lockfile update.
- `packages/app/src/fleet-map/render-scheduler.ts` (new)
- `packages/app/src/fleet-map/render-scheduler.test.ts` (new)
- `packages/app/src/fleet-map/shared-renderer.ts` (new)
- `packages/app/src/fleet-map/shared-renderer.test.ts` (new)
- `packages/app/src/fleet-map/scene.ts` (new)
- `packages/app/src/fleet-map/scene.test.ts` (new)
- `watchtower/decisions/ADR-0026-shared-renderer-blit-over-canvas-per-view.md` (new, `proposed`)
- `watchtower/DECISIONS.md`: ADR-0026 row.

## Contract

For TASK-090. Load `./shared-renderer` and `./scene` with `import()`; also `import("three")` for the `THREE` argument.

- `createRenderScheduler({ requestFrame, cancelFrame, now, render })` returns `invalidate`, `setPulsing`, `setVisible`, `setFocused`, `setReducedMotion`, `dispose`.
  - Starts visible and focused, motion allowed, not pulsing. Set the real values on mount.
  - Every render, pulse or invalidate, keeps a 30 fps cap. Idle requests no frames.
  - An `invalidate` while hidden waits and renders once `setVisible(true)`. A blurred but visible view still renders on `invalidate`.
  - `render(time)` gets `now()`. Derive the pulse phase from it and call `scene.setPulsePhase`.
- `acquireSharedRenderer()` returns `{ renderer, release }`. It throws when WebGL is not available; catch it and show the unavailable message. A second `release` from one handle is a no-op.
- `blitToCanvas(renderer, canvas)` copies the frame into the view's 2D canvas. Call it right after `scene.renderWith(renderer)` in the same task. Set the 2D canvas `width`/`height` to CSS size times the capped pixel ratio.
- `createFleetScene({ THREE })` returns:
  - `setGraph({ nodes: [{ id, position, color, pulsing }], edges, boundingRadius, edgeColor })`. `position` is the layout's `FleetPosition`. `color` and `edgeColor` are hex numbers. Edges with an unknown end are skipped.
  - `setHovered(id | null)`, which scales the node by 1.4.
  - `setPulsePhase(phase)`, where phase 0..1 is one cycle and pulsing nodes scale by up to ±20%.
  - `resize(cssWidth, cssHeight, devicePixelRatio)`, which caps the ratio at `MAX_PIXEL_RATIO` (1.5).
  - `pick(cssX, cssY)` returns a node id or `null`.
  - `renderWith(renderer)`, which resizes the shared renderer only when its size or ratio differs from this view.
  - `dispose()`.
- Camera: fixed perspective, 40 degree field of view, looking at the origin from about 50 degrees above the plane, fitted to `boundingRadius` times 1.15. Nodes use `MeshBasicMaterial`, so instance colors match the hex passed in exactly.
- Decision: [ADR-0026](watchtower/decisions/ADR-0026-shared-renderer-blit-over-canvas-per-view.md) (proposed).

## Verified

- `cd packages/app && npx vitest run src/fleet-map/render-scheduler.test.ts --bail=1` -> RED first (module missing), then 12 passed.
- `cd packages/app && npx vitest run src/fleet-map/shared-renderer.test.ts --bail=1` -> RED first, then 4 passed.
- `cd packages/app && npx vitest run src/fleet-map/scene.test.ts --bail=1` -> RED first, then 8 passed.
- `npm run typecheck` -> exit 0 (all workspaces, including `@getpaseo/app`).
- `npm run lint -- packages/app/src/fleet-map` -> exit 0, 0 warnings and 0 errors on 12 files.
- `npm run format:files -- <the 6 fleet-map files>` -> formatted.
- `rg -n 'from "three"|require\("three"\)' packages/app/src` -> only `shared-renderer.ts:1` and `scene.ts:1`. The tests load three with `await import("three")`.
- `rg -n "useUnistyles" packages/app/src/fleet-map` -> no match.
- Lazy chunk check (`npx expo export --platform web`) -> UNVERIFIED (unattended). It needs TASK-090, because nothing imports these modules yet. Q-034 default applies: a main-bundle `three` is accepted. For reference, three 0.186.1 `build/three.core.js` plus `build/three.module.js` measure 2,120,885 bytes raw and 419,721 bytes gzip, unminified. Metro minifies and tree-shakes less than this. TASK-090 or a human must record the measured entry size change.

## Anti-goal

- Checkpoint "idle frames": 0 frames in 2 s of fake time with no invalidate and no pulse. Source: `render-scheduler.test.ts`, case "renders 0 frames in 2 seconds with no invalidate and no pulse". Read 2026-10-13T11:03:55Z. PASS.
- Checkpoint "pulse cap": at most 30 frames in 1 s with the pulse on, also while `invalidate` runs every frame. With any pulse flag off, 0 frames in 2 s. Source: the same file, 4 flag cases plus 2 cap cases. Read 2026-10-13T11:03:55Z. PASS.
- Drift gauge "entry web bundle size change": not measured, because the lazy import lands in TASK-090. See Verified.
- Tripwire not crossed. Final: PASS.

## Lessons

- The lockfile diff has npm churn beyond `three` and its type deps (`@dimforge/rapier3d-compat`, `@tweenjs/tween.js`, `@types/stats.js`, `@types/webxr`, `meshoptimizer`). `fflate` moved from 0.8.2 (dev) to 0.8.3, because `@types/three` needs `~0.8.3`. npm also dropped the `"peer": true` flag from 25 `@rollup/rollup-*` entries and `escodegen/node_modules/source-map`. All of this is npm output and was not edited by hand.
- A `import type ... from "three"` in a test file also matches the Verify `rg` for static imports. Use `await import("three")` and `InstanceType<typeof THREE.X>` in tests.
- `npm run lint -- <path>` ran oxlint correctly here. The TASK-086 note about an ESLint wrapper did not reproduce.
- GitNexus impact: not run, because the TASK edits no existing symbol. All code files are new, and `package.json` holds no symbol.
