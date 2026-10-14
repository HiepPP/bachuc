# Learn 20261013-fleet-map

## Summary

Discrepancy: 5 found. All six TASKs shipped and the live check passed. The gaps were small: one wrong assumption about the sidebar, one missing file in a Files list, one Metro fact the plan guessed wrong, and two checks the run could not do by hand.

## Per TASK

- TASK-086: plan a pure layout function with a 25 ms limit for 500 nodes -> match. Shipped [packages/app/src/fleet-map/layout.ts](packages/app/src/fleet-map/layout.ts). Mistake: the builder could not print the exact time, because the vitest reporter hides console output. Fix: ask the builder to write each measured value into the outcome, and let the lead measure it again from a script (2.15 ms).
- TASK-087: plan to fall back to the sidebar's `cwd` rule when `workspaceId` is missing -> shipped `workspaceId ?? cwd` (Q-035). Mistake: the spec assumed the sidebar has such a rule. It skips those agents. Fix: open the code that a Boundaries line says to follow before you write the line.
- TASK-088: plan a `fleet_map` tab kind with 24 listed files -> shipped 25 files. Mistake: [packages/app/src/workspace-tabs/launcher/internal/catalog.test.ts](packages/app/src/workspace-tabs/launcher/internal/catalog.test.ts) was not in Files, and a launch order change breaks it. The tab label fallbacks in the two workspace files are not exhaustive, so a missing label gives no type error. Fix: when a spec adds a tab kind or a launch entry, list the tests that pin the order, and list both label functions.
- TASK-089: plan `three` as a lazy dependency behind `import()` -> shipped as planned, and the lazy chunks split. Mistake: the plan guessed that Metro would keep `three` out of the page start. Metro put it in the shared `__common` chunk, which `index.html` loads at start (Q-034 default). The size guess of about 150 KB gzip was not measured; the chunk is 814.5K raw. Fix: check how the bundler splits a chunk before the plan promises lazy loading. The lockfile also changed beyond `three` (npm output only).
- TASK-090: plan a web view with hover, click, and pulse -> match. Manual checks stayed `UNVERIFIED (unattended)` in the run, then passed by hand on 2026-10-14. Mistake: the plan counted on a live check that the run was not allowed to start. Fix: split manual checks from the run, and tell the owner when the run ends which ones wait.
- TASK-091: plan a doc of at most 80 lines -> match (38 lines).

## Plan-Level

- Wave shape: four groups in wave 1 and a limit of three builders. Group B waited for a free slot. No harm, and the waiting was short.
- ID migration at CREATE renamed six archive files of an older plan (TASK-076..078 to 083..085). It needed an owner answer and touched old archive text.
- The live check needed a running Electron window. `cua_repl` cannot move the pointer without a click, so hover was checked in a headless Chromium connected to live. A real window blur and the Explorer host stayed unchecked.
- ADR-0026 was written as `proposed` during the run and accepted by the owner before the merge.

## Lessons

- Count `CanvasRenderingContext2D.drawImage` calls over a few seconds to probe frame work of a canvas view. The run measured about 23 blits per second while pulsing, and 0 while blurred or idle.
- A headless browser connected to the live daemon (`127.0.0.1:6768`) is a good way to check hover and real mouse paths when `cua_repl` cannot. Never accept the default port 6767 in the connect form: it is release.
- Write measured values into the outcome from a script, not from a test reporter that hides output.
- When a plan promises "lazy" or "split", make the Verify line read the built output (`expo export`), and write what the bundler does, not what you expect.
