# ADR-0026 Shared renderer with per-view blit over one canvas per view

Status: superseded by ADR-0028
Date: 2026-10-13
Supersedes: -
Scope: `packages/app/src/fleet-map/**`

## Context

- Chrome limits the number of live WebGL contexts per page, and the terminal already holds contexts through `@xterm/addon-webgl`.
- The Render Budget in [watchtower/CONTEXT.md](watchtower/CONTEXT.md) allows one WebGL context for the whole app, shared by every Fleet map tab.
- A Fleet map can be open twice at once: in the main pane and in the Explorer sidebar.
- The plan's Decisions section names the shared renderer with a per-view blit. TASK-089 recorded it on 2026-10-13 during an unattended run.

## Decision

- `acquireSharedRenderer()` in `packages/app/src/fleet-map/shared-renderer.ts` keeps one reference-counted `WebGLRenderer`. The first acquire creates it. The last release calls `dispose()` and `forceContextLoss()`.
- Each view keeps its own 2D canvas. For each frame, the view's scene sizes the shared renderer, renders, and the view copies the result to its canvas with `blitToCanvas` (`drawImage`) in the same task.

## Consequences

- Two Fleet map tabs cost one WebGL context.
- Each frame pays one `drawImage` copy and, when two views differ in size, a renderer resize.
- A view must blit right after `render`. The WebGL drawing buffer is not preserved after the browser composites.
- `scene.ts` must not cache renderer size: another view can resize the shared renderer between frames.

## Options Considered

- One WebGL canvas per view: rejected. Two tabs would use two contexts, against the one-context Render Budget and next to the terminal's contexts.
- One canvas moved between views: rejected. Only one view can show a frame at a time, so the main pane and the Explorer could not both show the map.

## Revisit If

- The Fleet map needs per-frame work where the `drawImage` copy costs more than 2 ms on the owner's machine.
- Fleet map views are limited to one at a time, so a shared context brings no saving.
