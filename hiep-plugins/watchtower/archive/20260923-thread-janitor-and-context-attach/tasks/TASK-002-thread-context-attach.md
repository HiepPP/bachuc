# TASK-002 Thread Context Attach plugin

Group: B (writes only `plugins/thread-context-attach/`)
Class: code

## Brief

Goal: Let the user attach the last reply of another thread to the next message. This replaces handoff files in `/tmp` and copy-paste between Codex and Claude threads.

Change: copy the other thread's reply by hand -> composer + -> Thread -> pick a thread -> reply is attached as a snapshot.

How:

- Scaffold `plugins/thread-context-attach/` with `paseo plugin init`, as in [AGENTS.md](AGENTS.md).
- Define an attachment source with `defineAttachmentSource` in `shared/`. Follow [plugins/watchtower-board/shared/board.ts](plugins/watchtower-board/shared/board.ts).
- Add a server RPC `listThreads`. It returns recent agents from `agents.list()`: id, title, provider, cwd, `lastActivityAt`. Sort by `lastActivityAt`, newest first. Put the current workspace first. Exclude the current agent.
- Add a server RPC `getThreadSnapshot(agentId)`. It reads the timeline and returns the last assistant message text. Cap the text at 20,000 characters and mark the cut when it truncates.
- The attachment text starts with a header: thread title, provider, and cwd. Then the reply text follows.
- Nothing sends automatically. The user reviews the composer and sends.
- Register the source in `index.client.tsx` and return cleanup.
- Write tests for the snapshot builder: picks the last assistant message, truncates long text, handles a thread with no assistant message.
- Write a README.

Files:

- [plugins/thread-context-attach/](plugins/thread-context-attach/) (new plugin: manifest, package files, `index.server.ts`, `index.client.tsx`, `server/`, `shared/`, `tests/`, `README.md`)

Expected result:

- Composer + shows a Thread source. It lists recent threads, newest first. Paseo sends only the search query to the source, so the picker cannot put the current workspace first or hide the current thread. The `listThreads` RPC still supports both when a caller passes them.
- Picking a thread adds one attachment with the header and that thread's last reply.
- A thread with no reply gives a clear empty message, not an error.

## Verify

- `cd plugins/thread-context-attach && npm run typecheck && npm run lint && npm test` -> all pass, including the three snapshot cases.
- `paseo plugin install "$PWD/plugins/thread-context-attach" --id thread-context-attach`, then `paseo plugin ls thread-context-attach --json` -> `running`, no load error.
- Manual check in the Paseo app (needs a human): composer + -> Thread -> pick a finished thread -> the attachment shows its last reply. Nothing sends until the user presses send.
