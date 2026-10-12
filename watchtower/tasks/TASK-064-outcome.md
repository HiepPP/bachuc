# TASK-064 Outcome

## Outcome

Status: DONE

Changed:

- New `shared/lanes.ts`: `laneModel` places run clusters into project grids (TASK rows plus Ad-hoc, columns Todo, Running, Finished, Verified) with hints; `treeTask` lets a rootless cluster follow its newest labelled subagent; `needsYouItems` builds the Needs you list.
- New `client/lanes.tsx` (`TaskLanes`) and `client/needs-you.tsx` (`NeedsYouColumn`).
- `client/page.tsx`: each host lane shows Needs you beside the task lanes; the old `RunColumn` and its two status columns are gone. Cards render through the existing `RunCluster` via a render prop.
- `README.md`: new Task lanes section; column wording updated.
- New `tests/lanes.test.ts`; `tests/host-lanes.test.ts` reads card actions through the lane grid's `renderTree` and asserts the new layout.

Contract:

- Stars, Remove, unread fading, project colors, subagent clusters, the size control, and host lanes keep working; the host-specific action test still checks every call per host.
- "N new" moved from the Just finished header to the Finished and Verified headers of each grid.
- Deviation from the spec's How: cells hold the full existing cards, not one-line cards with a selection step. It reuses every card action with no new UI state; one-line cards can follow as their own TASK.
- Needs you is per host lane, not one list across hosts.

Verified:

- `npx tsx --test tests/lanes.test.ts` -> 6 pass.
- `npm test` -> 124 pass, 0 fail.
- `npm run typecheck` -> exit 0; `npm run-script lint` -> 0 warnings, 0 errors.
- `npm run cli -- plugin reload board` on live -> `running`, log "Plugin ready".
- Human check on live, 2026-10-12, light theme: PASS after one fix. Seen in the Expo web app (`npm run dev:app`, headless agent-browser) on the live daemon 6768, with mock agents labelled `watchtower.task`. At 1440 px, bachuc showed rows TASK-059 to TASK-066 plus Ad-hoc, "8/8", Needs you on the left with the 3 manual checks, a TASK-064 run in Verified, an unlabelled running run in Ad-hoc Running, and edit-diffs-demo as only its Ad-hoc row. Screenshot `/tmp/lanes-1.png`.
- Compact (420 px) first showed the grid about 280 px wide. Fix: the lane container uses `alignItems: "stretch"` on compact layouts (`client/page.tsx`). After reload the grid and Needs you fill the width. Screenshot `/tmp/lanes-compact2.png`.
- The `npm run dev:desktop` Electron window could not be checked: its screenshots stayed on an old frame and its Board did not poll, as if the window were occluded. The same app code was checked in the web build on the same daemon.

Anti-goal:

- Before changes: `client/page.tsx` 1764 lines; `wc -l`, 2026-10-12.
- After mounting lanes: 1659 lines (the `RunColumn` removal offsets the new mount).
- Final: 1659 lines; `wc -l`, 2026-10-12.
- Result: PASS against the 1764-line tripwire.

Lessons:

- `tests/host-lanes.test.ts` transpiles `page.tsx` and stubs unknown imports as `{}`; new client modules render as elements with an undefined type, so tests read their props.
