# TASK-003 Lifecycle and needs-you model

Group: B (standalone: `client/lifecycle.ts` and `tests/lifecycle.test.ts`)
Class: code

## Brief

Goal: Add pure functions that turn overview data into the lifecycle stepper and the Needs you list.

Change: no overview model -> `lifecyclePhases(overview)` and `needsYou(overview)` with tests.

Boundaries:

- Follow the lifecycle phase rules in [watchtower/CONTEXT.md](watchtower/CONTEXT.md) `## Decisions`.
- Pure functions in a `.ts` file. No React and no `react-native` import, so `tsx --test` can load them.

How:

- `lifecyclePhases(overview)` returns five phases in order: Plan, Implement, Verify, Review, Archive. Each has `state` (`done`, `current`, or `todo`) and a short `meta` line.
- Meta lines: Plan `"<n> tasks in <g> groups"`; Implement `"<done> of <n> done"`, plus `", autorun live"` when the run has a start and no finish; Verify `"<p> checks and <m> manual checks"`; Review `"Draft PR open"` when finished, else `"After the run"`; Archive `"Learnings, then archive"`.
- `needsYou(overview)` returns rows in this order: blocking questions (non-empty `blocks`), then other questions, then proposed ADRs, then one row for manual checks when any exist.
- Each row has `kind`, `id`, `title`, `meta`, and `tone` (`danger` for blocking, `warning` for a question with a default, `neutral` otherwise).
- Count distinct non-null task `group` values for the Plan meta. When every `group` is `null`, write only `"<n> tasks"`.

Files:

- [hiep-plugins/plugins/watchtower-board/client/lifecycle.ts](hiep-plugins/plugins/watchtower-board/client/lifecycle.ts) (new)
- [hiep-plugins/plugins/watchtower-board/tests/lifecycle.test.ts](hiep-plugins/plugins/watchtower-board/tests/lifecycle.test.ts) (new)

Expected result:

- A plan with open tasks gives Plan `done`, Implement `current`, and the other three `todo`.
- Every task `DONE` and no `Finished:` gives Verify `current`.
- A `Finished:` value gives Review `current`, and Implement and Verify `done`.
- Plan `Status: ARCHIVED` gives Archive `current`.
- Needs you puts a blocking question before a defaulted one, and an ADR after both.
- No tasks and no plan gives Plan `current` with meta `"No plan yet"`.

Anti-goal: the model stays cheap: `lifecyclePhases` plus `needsYou` on a 200-task overview finish within 5 ms on average over 100 runs; drift gauge; read in a test at completion.

## Verify

- In the plugin folder: `npx tsx --test tests/lifecycle.test.ts` -> every case above passes.
- In the plugin folder: `npm run typecheck` and `npm run lint` -> exit 0.
- `rg -n "react-native|from \"react\"" hiep-plugins/plugins/watchtower-board/client/lifecycle.ts` -> no match.
- The timing test reports the average -> 5 ms or less.
