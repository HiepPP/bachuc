# TASK-001 Recap log in Board

Group: A (writes only files in plugins/board)
Class: code

## Brief

Goal: Board keeps a log of the `## Recap` block that ends each agent reply. A new Recaps view groups the log by day, then by project, so the user gets a daily report without opening each thread.

Change: Board shows live runs only -> Board also has a Recaps view with each day's recaps per project.

How:

- Add a pure parser. It finds the last `## Recap` heading in a reply and reads the lines until the next `## ` heading or the end. It reads `Branch:`, `Did:`, and `Commit/push:`, with or without a leading `- `. It keeps the raw block, cut to 2,000 characters. A reply with no `## Recap` gives no entry.
- On `agent.turn_ended` with `outcome.kind === "completed"`, parse the last `assistant_message` in `timeline`. Save an entry with `agentId`, `turnId`, `title`, `cwd`, `workspaceId`, `endedAt`, local `day` (`YYYY-MM-DD`), `branch`, `did`, `commitPush`, and `raw`.
- Store entries in `$PASEO_HOME/plugin-data/board/recaps.jsonl` with mode `0600`. Skip a duplicate `agentId` plus `turnId`. Keep at most 90 days and at most 2,000 entries.
- Add RPC `board.recaps` with input `{ days }` (1 to 30, default 7). It returns days, newest first. Each day holds projects, and each project holds entries, newest first. Name the project the same way [plugins/board/server/store.ts](plugins/board/server/store.ts) does.
- Add a Runs and Recaps switch next to `BoardSizeControl`. The Recaps view shows each day, then a project header with the project color, then one row per recap: thread title, branch, did, and commit/push.
- A click on a row opens that thread, the same way a run card does.
- Each day has a Copy button. It copies the day as markdown (`## <day>`, `### <project>`, one bullet per recap) with `copyText`.

Files:

- [plugins/board/server/recaps.ts](plugins/board/server/recaps.ts) (new: parser and JSONL store)
- [plugins/board/shared/recaps.ts](plugins/board/shared/recaps.ts) (new: `board.recaps` RPC contract)
- [plugins/board/index.server.ts](plugins/board/index.server.ts) (record recaps on turn end, serve the RPC)
- [plugins/board/client/recaps.tsx](plugins/board/client/recaps.tsx) (new: Recaps view and day copy)
- [plugins/board/client/page.tsx](plugins/board/client/page.tsx) (Runs and Recaps switch in the header)
- [plugins/board/tests/recaps.test.ts](plugins/board/tests/recaps.test.ts) (new tests)
- [plugins/board/README.md](plugins/board/README.md) (new Recaps section)

Expected result:

- A completed turn that ends with `## Recap` adds one entry. A turn without it adds nothing.
- The Recaps view lists today's recaps under the right project, and a click opens the thread.
- Copy puts the day's markdown on the clipboard.
- The Runs view works as before.

## Verify

- `cd plugins/board && npm test` -> all pass. New tests cover: recap with bullets, recap without bullets, Vietnamese text, a missing field, no recap, a recap that is not the last section, duplicate `turnId`, the 90-day and 2,000-entry limits, and grouping by day then project.
- `cd plugins/board && npm run typecheck` -> exit 0.
- `cd plugins/board && S=lint; npm run $S` -> exit 0.
- After `paseo plugin reload board` and one completed turn that ends with `## Recap`: `stat -f %Lp ~/.paseo/plugin-data/board/recaps.jsonl` -> `600`, and the file has one new line for that turn.
- Manual, needs the Paseo app: the Recaps view shows that recap under today and its project. A click opens the thread. Copy pastes the day's markdown.
