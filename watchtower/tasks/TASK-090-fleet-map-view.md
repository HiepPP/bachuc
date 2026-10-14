# TASK-090 Fleet map web view

Group: E (writes only the web view and its test; it depends on the other groups by order, not by a shared file)
Class: code

## Brief

Goal: Show the map. The web view joins the node hook, the layout, and the lazy scene. A hover shows a label and a click opens the agent.

Change: The Fleet map tab shows a placeholder -> it shows nodes colored by status, links from parent to subagent, a hover label, and a click that opens the agent.

Boundaries:

- `view.web.tsx` serves Electron and the plain web. No `.electron.tsx` file is needed, because the view uses no Electron API. The base `view.tsx` from TASK-088 stays as the native fallback.
- Export `FleetMapView({ serverId })` with the same props as the base `view.tsx` from TASK-088. The panel from TASK-088 already passes `serverId` from the pane context. Do not edit the panel file or the locale files; TASK-088 owns them. Open an agent with `navigateToAgent({ serverId, agentId })` from [packages/app/src/utils/navigate-to-agent/index.ts](packages/app/src/utils/navigate-to-agent/index.ts).
- Load `./shared-renderer` and `./scene` with `import()` in an effect. Show the existing loading state until they resolve. If WebGL is not available, show a short text message. Do not throw.
- Colors: call `getStatusDotColor` with `showDoneAsInactive: true` and pass hex numbers into the canvas through `withUnistyles` and `uniProps`. Do not use `useUnistyles()`.
- Drive the scheduler: `invalidate()` on nodes change, resize, hover change, and theme change. `setPulsing(true)` only when at least one node has bucket `running`. Set visible and focused from `document.visibilityState` and window focus. Read reduced motion from `matchMedia("(prefers-reduced-motion: reduce)")`.
- Hover label is a DOM overlay (agent title and workspace name), not a 3D text mesh. Keep it out of the render loop.
- Empty state: no agents -> a short localized message and no canvas work.
- Release the shared renderer on unmount.
- Do not restructure the agent panel or the sidebar.

How:

- Write `view.web.tsx` with a canvas ref, a resize observer, pointer move and click handlers, and the effects above.
- Use the empty and unavailable strings that TASK-088 added to the locales.
- Add a small test for the pure parts that do not need GL: scheduler wiring helpers, if any are extracted, and the empty state. Prefer extracting a `useFleetMapDriver` hook that is testable with a fake scene.

Files:

- [packages/app/src/fleet-map/view.web.tsx](packages/app/src/fleet-map/view.web.tsx) (new: web and Electron view)
- [packages/app/src/fleet-map/view.test.tsx](packages/app/src/fleet-map/view.test.tsx) (new: driver and empty-state tests)

Expected result:

- Open Fleet map with agents: one node per agent, colored like the sidebar dots.
- A subagent node links to its parent.
- Hover over a node shows the agent title and workspace name. Moving away hides it.
- Click on a node opens that agent.
- A running node pulses. A blurred window or hidden tab stops the pulse and all frames.
- No agents shows the empty message.
- Two Fleet map tabs (main and Explorer) use one WebGL context.

Anti-goal: Nothing outside the map changes. Tripwire: `git diff --name-only` lists only files in this TASK's Files list plus `watchtower/` plan files; read at the end of the TASK.

## Verify

- `cd packages/app && npx vitest run src/fleet-map/view.test.tsx --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/fleet-map` -> exit 0.
- `rg -n "useUnistyles" packages/app/src/fleet-map` -> no match.
- `rg -n "import\(\"\./(scene|shared-renderer)\"\)" packages/app/src/fleet-map/view.web.tsx` -> both lazy imports present.
- Needs a human or a browser session: on live, with three or more agents and one subagent, the map shows colored nodes and a link, hover shows the label, click opens the agent, and a blurred window stops the pulse.
- Needs a human or a browser session: the Chrome performance panel shows no repeated frames after the pulse stops.
- Anti-goal: `git diff --name-only` shows only listed files and `watchtower/` plan files.
