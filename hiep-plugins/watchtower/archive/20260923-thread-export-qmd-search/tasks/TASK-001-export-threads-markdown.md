# TASK-001 Export threads to markdown

Group: A (index.server.ts is shared with TASK-002)
Class: code

## Brief

Goal: Keep one markdown file per Paseo thread, with one heading per turn. Update it when the thread finishes a turn.

Change: thread text lives only in the daemon -> each thread also has `<agentId>.md` in the export folder.

How:

- Add a pure function `renderThreadMarkdown(thread, entries, { cut })`. It groups entries into turns. A turn is one `user_message` and the `assistant_message` items after it. Assistant text before the first user message goes into `## Turn 0`. Other item types are skipped.
- Use this file layout:

  ```text
  # <title>

  Thread: <agentId> · <provider> · <cwd>
  Updated: <lastActivityAt>
  <optional line: Older messages were cut. Turn numbers start at the oldest kept message.>

  ## Turn 1

  ### Asked
  <user text>

  ### Answer
  <assistant replies, oldest first, separated by blank lines>
  ```

- Number turns from the first kept user message. When pages are cut, still count from the first kept turn, and add the cut line.
- Add `readTimeline(handle)`. It reads up to 10 pages of 200 entries, newest first, and returns entries oldest first plus a `cut` flag. Reuse the paging style of `readLastReply` in [plugins/thread-context-attach/server/threads.ts](plugins/thread-context-attach/server/threads.ts).
- Add `exportThread(paseo, agentId, dir)`. It writes to `<dir>/<agentId>.md.tmp`, then renames to `<agentId>.md`. Use file mode `0600`. Skip the write when the rendered text is unchanged.
- Add a small exporter with a per-thread debounce of 2 seconds and at most 2 exports at once. It exposes `onExported(listener)` so TASK-002 can react.
- In [plugins/thread-context-attach/index.server.ts](plugins/thread-context-attach/index.server.ts), call the exporter on `agent.turn_ended`. Backfill every non-archived thread once, one at a time, on the first hook or RPC call. The plugin gets a Paseo API only inside hooks and RPCs. Stop timers and pending work in the cleanup function.
- Resolve the folder from `PASEO_HOME` (default `~/.paseo`) as `plugin-data/thread-context-attach/threads/`. Create it with mode `0700`.
- Log one line per failed export. Never log thread text.

Files:

- [plugins/thread-context-attach/server/export.ts](plugins/thread-context-attach/server/export.ts) (new render, read, write, and exporter)
- [plugins/thread-context-attach/index.server.ts](plugins/thread-context-attach/index.server.ts) (hook, backfill, cleanup)
- [plugins/thread-context-attach/tests/export.test.ts](plugins/thread-context-attach/tests/export.test.ts) (new tests)

Expected result:

- Each non-archived thread has a file after plugin start.
- A finished turn updates that thread's file within about 3 seconds.
- The composer picker behaves exactly as before.

## Verify

- `cd plugins/thread-context-attach && npm test` -> all pass. New tests cover: turn grouping, Turn 0, skipped tool items, the cut line, an unchanged file not rewritten, the debounce, and the limit of 2 exports.
- `cd plugins/thread-context-attach && npm run typecheck` -> exit 0.
- `cd plugins/thread-context-attach && S=lint; npm run $S` -> exit 0.
- `paseo plugin reload thread-context-attach`, wait 30 seconds, then `ls ~/.paseo/plugin-data/thread-context-attach/threads | wc -l` -> about the number of non-archived threads, and `stat -f %Lp` on one file prints `600`.
