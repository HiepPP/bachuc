# TASK-001 Overview plan data reader

Group: A (shared file: `server/overview.ts`, `shared/overview.ts`, and `tests/overview.test.ts`, also written by TASK-002)
Class: code

## Brief

Goal: Add the `watchtower.overview.read` RPC. It returns the plan data that the overview needs and that the board RPC does not have.

Change: only `watchtower.read` exists -> `watchtower.overview.read` also returns plan meta, task deps as IDs, Plan Verify lines, and manual checks.

Design: option 3f in the mock named in [watchtower/CONTEXT.md](watchtower/CONTEXT.md).

Boundaries:

- Keep the `watchtower.read` RPC and `boardSchema` unchanged. The Explorer panel and attachment search use them.
- Read files only through the guarded `readFile` in [hiep-plugins/plugins/watchtower-board/server/board.ts](hiep-plugins/plugins/watchtower-board/server/board.ts). Export the helpers you reuse; do not copy them.
- Read-only. The reader never writes, renames, or deletes a file.

How:

- Create `shared/overview.ts` with a zod `overviewSchema` and `readOverviewRpc` (`name: "watchtower.overview.read"`, input `{ workspaceId }`).
- The schema holds: `plan` (`title`, `slug`, `status`, `updated`, each nullable except `title`), `tasks` (the board task fields plus `depIds: string[]` and `group`, the Tracker Group cell or `null` when the column is missing), `planVerify: string[]`, `manualChecks: { text: string; tasks: string[] }[]`, `message`, and `warnings`.
- Create `server/overview.ts` with `readOverview(directory)`. Reuse `readBoard` for tasks, then add the new fields from `NEXT.md`.
- `depIds`: every `TASK-NNN` token in the Deps cell. `-` gives an empty list.
- `planVerify`: the bullet lines of `## Plan Verify`, as plain text.
- `manualChecks`: each `## Handoff` bullet that starts with `Manual check pending:`. Keep the text after the prefix. Take the `TASK-NNN` tokens of the line as `tasks`.
- Add `loadWorkspaceOverview(workspaceId, paseo)` to `server/handlers.ts`, like `loadWorkspaceBoard`.
- Register the RPC in `index.server.ts`.
- Add `tests/overview.test.ts`. Use a temp fixture like `tests/board.test.ts`.

Files:

- [hiep-plugins/plugins/watchtower-board/shared/overview.ts](hiep-plugins/plugins/watchtower-board/shared/overview.ts) (new schema and RPC)
- [hiep-plugins/plugins/watchtower-board/server/overview.ts](hiep-plugins/plugins/watchtower-board/server/overview.ts) (new reader)
- [hiep-plugins/plugins/watchtower-board/server/board.ts](hiep-plugins/plugins/watchtower-board/server/board.ts) (export `readFile`, `tableRows`, and `plain`; no behavior change)
- [hiep-plugins/plugins/watchtower-board/server/handlers.ts](hiep-plugins/plugins/watchtower-board/server/handlers.ts) (add `loadWorkspaceOverview`)
- [hiep-plugins/plugins/watchtower-board/index.server.ts](hiep-plugins/plugins/watchtower-board/index.server.ts) (register the RPC)
- [hiep-plugins/plugins/watchtower-board/tests/overview.test.ts](hiep-plugins/plugins/watchtower-board/tests/overview.test.ts) (new tests)

Expected result:

- `readOverview` on a fixture returns the slug, status, and updated date from `## Current Active Plan`.
- `depIds` for `TASK-006, TASK-010` is `["TASK-006", "TASK-010"]`. For `-` it is `[]`.
- `planVerify` lists each Plan Verify bullet. A plan with no `## Plan Verify` gives `[]`.
- `manualChecks` keeps only `Manual check pending:` bullets, with their task IDs.
- A missing `watchtower/NEXT.md` gives an empty plan with a `message`, not a thrown error.
- `overviewSchema.parse` accepts every reader result in the tests.

Anti-goal: the board RPC behavior stays the same: `tests/board.test.ts` keeps 10 of 10 passing; tripwire; read from `npx tsx --test tests/board.test.ts` before the first edit, after the `board.ts` export change, and at completion.

## Verify

- In the plugin folder: `npx tsx --test tests/overview.test.ts` -> every test passes.
- In the plugin folder: `npx tsx --test tests/board.test.ts` -> 10 pass, 0 fail.
- In the plugin folder: `npm run typecheck` and `npm run lint` -> exit 0.
- `rg -n "watchtower.overview.read" hiep-plugins/plugins/watchtower-board/shared/overview.ts` -> one match.
