# TASK-069 Outcome

## Outcome

Status: DONE

Changed:

- New [shared/timeline.ts](hiep-plugins/plugins/board/shared/timeline.ts): the `board.timeline` contract and the pure model: 24-hour window, gap folding copied from watchtower-board, x positions, hour ticks, hourly load buckets with the peak, rows per host and project with one subagent row, and the host notes.
- New [server/timeline.ts](hiep-plugins/plugins/board/server/timeline.ts): `timelineSnapshot` keeps runs that overlap the window, leaves out and counts runs without a start time, and sets `olderDropped` when the full store dropped runs inside the window.
- [index.server.ts](hiep-plugins/plugins/board/index.server.ts): the snapshot refresh moved into `refresh`, shared by `board.snapshot` and the new `board.timeline` handler. [server/store.ts](hiep-plugins/plugins/board/server/store.ts) names the limit `FINISHED_LIMIT`.
- New [client/timeline.tsx](hiep-plugins/plugins/board/client/timeline.tsx): legend, load strip, axis with break labels, host sections, thread and subagent rows, and the footer. Each host runs its own query; all hosts share one axis.
- [client/recaps.tsx](hiep-plugins/plugins/board/client/recaps.tsx): Runs, Timeline, and Recaps tabs. [client/page.tsx](hiep-plugins/plugins/board/client/page.tsx): renders `TimelineView` for the Timeline tab.
- New [tests/timeline.test.ts](hiep-plugins/plugins/board/tests/timeline.test.ts) with 9 cases. [README.md](hiep-plugins/plugins/board/README.md): new Timeline section.

Contract:

- Follows ADR-0012: Runs stays the triage view.
- Q-024 took the default: the store keeps 50 finished runs, and a host that lost runs in the window says "Older threads are not kept".
- The timeline uses the same visible runs as Runs, so a thread removed from Board or archived is not drawn.
- An empty window does not fold; a fold needs at least one thread.

Verified:

- `npx tsx --test tests/timeline.test.ts` -> 9 pass, including the timing case.
- `npm run typecheck && npm run-script lint && npm test` in the Board plugin -> exit 0, 0 warnings, 123 pass.
- Root `npm run typecheck` -> exit 0.
- Live check, 2026-10-12, Expo web on daemon 6768, light theme: with no threads the host said "Nothing ran in the last 24 hours". After a mock thread, the strip read "peak 1 · 14:00–14:21", a 48 px break read "23h 57m", and the running bar ended at the now line. Screenshot `/tmp/t069-timeline.png`.
- Pressing the bar opened the thread "Timeline check one". After a stop, the bar turned dashed gray as cancelled. Screenshot `/tmp/t069-cancelled.png`. The test agent is archived.
- The live check found one bug, now fixed: an empty window folded into one 24h break.
- Not checked on live: subagent rows, the TASK chip on a bar, and a second host. Tests cover the rows; live had no labelled or nested threads.

Anti-goal:

- After the model: 50 runs through `timelineSnapshot` and `timelineModel` took 0.19 ms; `tests/timeline.test.ts`, 2026-10-12.
- At completion: 0.17 ms; `tests/timeline.test.ts`, 2026-10-12.
- Result: PASS against the 30 ms limit. The case measures the RPC's own work; the host agent list read is not included.
