# TASK-006 Outcome

## Outcome

Status: DONE

Changed:

- Added [hiep-plugins/plugins/watchtower-board/client/overview-cells.tsx](hiep-plugins/plugins/watchtower-board/client/overview-cells.tsx) with `LifecycleStepper`, `NeedsYouCell`, `RunLogCell`, `ManualChecksCell`, and `HistoryCell`.
- The file also exports `OverviewCell`, the bordered frame with a title row that every cell uses, so TASK-007 can wrap the timeline in the same frame. It also exports `LIST_LIMIT`, `StatusCounts`, and `RunningTask`.
- Needs you copies `"<meta>: <title>"` with `copyText` and shows "Copied" on that row (Q-001, read-only). A cell with a blocking question gets a warning border and tint.
- The run log shows the newest log rows plus one `now` row per running task, 5 rows in total. Running tasks come from props, because `RUN.md` logs only finished iterations.
- Decisions and history shows at most 3 ADRs, proposed first, and at most 3 archived plans, then one "<n> more" line. This is below the 5-row limit, so the cell fits the bento row.

Contract:

- Props only. Cells call no data hook. Lists stop at 5 rows and add a "<n> more" line.
- `RunLogCell` takes `run` and `running: { id, minutes }[]`. `LifecycleStepper` takes `phases` and `counts: { done, running, blocked, todo }`.
- Icons are Lucide names through the host `Icon`: `FileText`, `Play`, `ShieldCheck`, `GitPullRequest`, `Archive`, `CircleCheck`, `CircleAlert`, `MessageCircleQuestion`, `Scale`, `ClipboardCheck`, and `Copy`.

Verified:

- `npm run typecheck` -> exit 0. `npm run lint` -> exit 0, 0 warnings.
- `rg -n "useQuery|useRpc" client/overview-cells.tsx` -> no match.
- `rg -n "—" client/overview-cells.tsx` -> no match.
- `rg -c "export function (LifecycleStepper|NeedsYouCell|RunLogCell|ManualChecksCell|HistoryCell)"` -> 5.
- `npm test` in the plugin folder -> 46 pass, 0 fail.
- Every icon name exists in the installed `lucide-react-native`. `MessageCircleQuestion` is an alias of `message-circle-question-mark.js`.
- The rendered cells: UNVERIFIED (autonomous run). Plan Verify checks them on live after TASK-008.

Anti-goal:

- Before the first edit: 0 data hooks; the file did not exist, 2026-10-07 12:58.
- After the first cell (`OverviewCell` and `LifecycleStepper`): 0; `rg -c "useQuery|useRpc"`, 2026-10-07 12:59.
- Final: 0; same command, 2026-10-07 12:59.
- Result: PASS against 0 data hooks.

Lessons:

- Lucide icons go through `Reflect.get(LucideIcons, name)` in the host, so PascalCase aliases such as `MessageCircleQuestion` work. Check names in `node_modules/lucide-react-native/dist/esm/lucide-react-native.js`.
