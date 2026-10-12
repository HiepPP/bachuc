# TASK-069 Board Timeline tab

Group: B4 (`hiep-plugins/plugins/board/server/timeline.ts`, `hiep-plugins/plugins/board/shared/timeline.ts`, `hiep-plugins/plugins/board/client/timeline.tsx`, the view switch in `client/recaps.tsx`, Board README and tests)
Class: code

## Brief

Goal: Board gets a third tab, Timeline, that shows every thread of every host over the last 24 hours, with a load strip on top and long idle gaps folded.

Change: Runs and Recaps -> Runs, Timeline, and Recaps.

Design: Claude Design project "Watchtower unified UI options", file `Board Timeline.dc.html`, option 1a with the load strip of option 1c.

Boundaries:

- Follows [ADR-0012](watchtower/decisions/ADR-0012-status-columns-with-task-chips-over-task-lanes.md): Runs stays the triage view; Timeline is for looking back.
- Server: a new RPC `board.timeline` returns the runs that overlap the last 24 hours from the run store (running and finished), with `startedAt`, `endedAt`, status, project, parent, `taskId`, and `needsInput`. A run without `startedAt` is left out and counted.
- The store keeps 50 finished runs. On a busy day the window can lose its oldest threads; the view then says "Older threads are not kept" (Q-024).
- Gap folding follows watchtower-board: an idle gap longer than max(2 h, 4 x the average thread) becomes a 48 px break labelled with its length. Copy the rule; plugins never import each other.
- Rows: one per top-level thread, grouped by host, then project; a thread's subagents share one thin row under it. Bars use the project tint when finished, dark while running, red when failed, dashed gray when cancelled, amber when it needs input.
- Load strip: agents running at once, per hour, top-level threads only; the peak is labelled.
- A bar press opens the thread; a TASK chip opens Watchtower at that TASK (TASK-066).
- Each host loads its own timeline; an offline host shows its lane as unavailable, as on Runs.

How:

- Add `server/timeline.ts` and the RPC; add `shared/timeline.ts` with the pure model: window, gap folding, x positions, hour ticks, and load buckets.
- Add `client/timeline.tsx` and the Timeline tab in the view switch.
- Test the model in a new `tests/timeline.test.ts`.

Files:

- [hiep-plugins/plugins/board/server/timeline.ts](hiep-plugins/plugins/board/server/timeline.ts) (new)
- [hiep-plugins/plugins/board/shared/timeline.ts](hiep-plugins/plugins/board/shared/timeline.ts) (new)
- [hiep-plugins/plugins/board/client/timeline.tsx](hiep-plugins/plugins/board/client/timeline.tsx) (new)
- [hiep-plugins/plugins/board/client/recaps.tsx](hiep-plugins/plugins/board/client/recaps.tsx) (third tab in `BoardViewSwitch`)
- [hiep-plugins/plugins/board/index.server.ts](hiep-plugins/plugins/board/index.server.ts) (register the RPC)
- [hiep-plugins/plugins/board/tests/timeline.test.ts](hiep-plugins/plugins/board/tests/timeline.test.ts) (new)
- [hiep-plugins/plugins/board/README.md](hiep-plugins/plugins/board/README.md)

Expected result:

- A 24-hour window with a 6 h 40 m idle gap shows one 48 px break labelled "6h 40m".
- A running thread ends at the now line; its subagents sit in one row under it.
- The load strip peaks at the busiest hour and labels it.
- Cases: a gap of 90 minutes stays at scale; a thread with no start time is left out and counted; no threads gives "Nothing ran in the last 24 hours".

Anti-goal: the timeline RPC for 50 runs answers in under 30 ms; drift gauge; read from a timing case in `tests/timeline.test.ts` after the model and at completion.

## Verify

- `cd hiep-plugins/plugins/board && npx tsx --test tests/timeline.test.ts` -> all pass, including the timing case.
- `cd hiep-plugins/plugins/board && npm run typecheck && npm run-script lint && npm test` -> all pass.
- Human check on live (Expo web on daemon 6768, light theme): the Timeline tab shows the live threads of the last 24 hours, the load strip, and a folded gap when one exists.
