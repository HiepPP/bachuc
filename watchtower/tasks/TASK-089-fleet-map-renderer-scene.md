# TASK-089 Shared renderer, scene, and scheduler

Group: D (writes the three.js files and the app `package.json`)
Class: risky

## Brief

Goal: Build the three.js side of the map: one shared WebGL renderer, a scene for nodes and edges, and a render scheduler that spends no frames when nothing changes.

Change: The app has no 3D code -> `three` is a lazy dependency, and three small modules render a fleet layout on demand.

Boundaries:

- Follows the Render Budget in [watchtower/CONTEXT.md](watchtower/CONTEXT.md): one WebGL context, lazy import, render on demand, pulse at most 30 frames per second, device pixel ratio at most 1.5.
- Add `three` and `@types/three` to `packages/app` with `npm install three @types/three --workspace=@getpaseo/app`. Pin nothing by hand; record the installed versions in the outcome.
- Only `shared-renderer.ts` and `scene.ts` may import `three` statically. TASK-090 loads them with `import()`.
- Shared renderer: `acquireSharedRenderer()` counts users and creates one `WebGLRenderer` on the first call. The last `release` calls `dispose()` and `forceContextLoss()`. A view blits the result to its own 2D canvas with `drawImage` right after `render`. Two Fleet map tabs then cost one context.
- Scheduler: pure, with injected `requestFrame`, `cancelFrame`, `now`, and `render`. `invalidate()` schedules one frame. A continuous pulse runs only when `setPulsing(true)`, `setVisible(true)`, `setFocused(true)`, and reduced motion is off. Otherwise zero frames.
- Scene: instanced mesh for nodes (one color per instance), one line set for edges, a fixed oblique perspective camera fitted to `boundingRadius`. No camera control in v1.
- Picking: ray cast against the instanced mesh by pointer position. Return a node id or `null`.
- Colors come in as plain hex numbers from the caller. The scene never reads the Unistyles theme.
- If Metro does not split `import("three")` into a lazy web chunk, follow the default in [Q-034](watchtower/QUESTIONS.md) and record the measured size change.
- Record an ADR with Record Decision: shared renderer with per-view blit over one canvas per view. Link it from the outcome.

How:

- Write `render-scheduler.ts` and its fake-timer tests first.
- Write `shared-renderer.ts` with an injectable renderer factory, so tests count creates and disposes without WebGL.
- Write `scene.ts` with `createFleetScene({ THREE })` returning `setGraph`, `setHovered`, `setPulsePhase`, `resize`, `pick`, `renderWith`, and `dispose`. Test graph building and picking in node, which needs no GL.

Files:

- [packages/app/package.json](packages/app/package.json) (add `three` and `@types/three`)
- [package-lock.json](package-lock.json) (lockfile update from npm)
- [packages/app/src/fleet-map/render-scheduler.ts](packages/app/src/fleet-map/render-scheduler.ts) (new: pure scheduler)
- [packages/app/src/fleet-map/render-scheduler.test.ts](packages/app/src/fleet-map/render-scheduler.test.ts) (new)
- [packages/app/src/fleet-map/shared-renderer.ts](packages/app/src/fleet-map/shared-renderer.ts) (new: reference-counted renderer)
- [packages/app/src/fleet-map/shared-renderer.test.ts](packages/app/src/fleet-map/shared-renderer.test.ts) (new)
- [packages/app/src/fleet-map/scene.ts](packages/app/src/fleet-map/scene.ts) (new: scene, camera, picking)
- [packages/app/src/fleet-map/scene.test.ts](packages/app/src/fleet-map/scene.test.ts) (new)

Expected result:

- With no `invalidate` call and no pulse, the scheduler renders 0 frames in 2 seconds of fake time.
- Many `invalidate` calls in one tick render 1 frame.
- With pulse, visible, focused, and motion allowed, the scheduler renders at most 30 frames in 1 second of fake time. If any of those flags is off, it renders 0 frames.
- Two `acquire` calls create 1 renderer. The first `release` disposes nothing. The second disposes it once.
- `pick` returns the id of the node under the pointer, and `null` over empty space.
- `setGraph` with N nodes and M edges builds one instanced mesh with N instances and one line set with M segments.

Anti-goal: Idle frame cost stays at zero. Tripwire: the scheduler test asserts 0 frames over 2 seconds of fake time with no invalidate and no pulse, and at most 30 frames per second with pulse; read at the end of the TASK from `render-scheduler.test.ts`. Drift gauge: entry web bundle size change from the lazy import, recorded in the outcome.

## Verify

- `cd packages/app && npx vitest run src/fleet-map/render-scheduler.test.ts --bail=1` -> all cases in Expected result pass.
- `cd packages/app && npx vitest run src/fleet-map/shared-renderer.test.ts --bail=1` -> pass.
- `cd packages/app && npx vitest run src/fleet-map/scene.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/fleet-map` -> exit 0.
- `rg -n "from \"three\"|require\(\"three\"\)" packages/app/src` -> matches only in `shared-renderer.ts` and `scene.ts`.
- `rg -n "useUnistyles" packages/app/src/fleet-map` -> no match.
- Needs a human or a browser session: after TASK-090, `cd packages/app && npx expo export --platform web --output-dir /tmp/fleet-export` shows `WebGLRenderer` in a separate chunk, or the outcome records the entry size change and cites Q-034.
- Anti-goal: the scheduler test asserts 0 idle frames and the 30 frames per second cap, and passes.
