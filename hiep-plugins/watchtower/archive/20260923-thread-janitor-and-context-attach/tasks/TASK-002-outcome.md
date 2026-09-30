# TASK-002 Outcome

## Outcome

Status: DONE

## Changed

New untracked plugin `plugins/thread-context-attach/` (scaffolded with `paseo plugin init`, greeting example files removed):

- `paseo-plugin.json`, `package.json`, `package-lock.json`, `tsconfig.json` (board-style scripts, SDK 0.9.0-beta.2)
- `shared/threads.ts`: RPCs `thread-context.list-threads`, `thread-context.get-thread-snapshot`, `thread-context.search`; attachment source `thread` titled "Thread"
- `server/snapshot.ts`: pure snapshot builder (last assistant message, 20,000-char cap with `[Truncated: ...]` marker, header, empty message), ordering, query match
- `server/threads.ts`: `agents.list()` + timeline paging (tail, then up to 4 older pages of 200), snapshot cache, attachment search
- `index.server.ts`, `index.client.tsx` (registers the source, returns cleanup)
- `tests/snapshot.test.ts`, `README.md`

## Contract

- Composer + -> Thread lists up to 12 non-archived threads, newest activity first. Selecting one adds one attachment: `# Thread: <title>`, `Provider:`, `Cwd:`, `## Last reply`, reply text. No reply -> `This thread has no assistant reply yet.` Nothing is sent automatically.
- `lastActivityAt` is derived as the newest of `updatedAt`, `lastUserMessageAt`, `attentionTimestamp` (the daemon agent snapshot has no such field).
- Deviation: the host passes only `{ query }` to attachment search (verified in SDK `searchPluginAttachments` and the installed 0.9.1 app bundle), so the composer picker cannot know the current agent/workspace. "Current workspace first" and "exclude current agent" are implemented and tested in `list-threads` (inputs `currentAgentId`/`currentWorkspaceId`) but UNSATISFIABLE in the picker until Paseo passes composer context. Replacement check: picker lists newest first; the current thread may appear in the list.
- RPC names use lowercase-hyphen form; the daemon rejected `threadContext.listThreads` as an invalid RPC method.

## Verified

Group-level run (group B has only this task), from `plugins/thread-context-attach`:

- `npm run format`, `npm run typecheck`, `oxlint` (0 warnings, 0 errors), `npm test`: 4/4 pass (last assistant message, 20,000-char truncation, no-assistant thread incl. older-page read, ordering/exclusion).
- `paseo plugin install "$PWD" --id thread-context-attach` first failed on the RPC name; after renaming, `paseo plugin reload thread-context-attach` -> `paseo plugin ls thread-context-attach --json` status `running`, no error; logs show `Plugin ready`.
- Read-only live smoke against the local daemon (counts only, no content printed): 50 threads listed newest first; current-agent exclusion and same-workspace-first held; 5 snapshots had the header; search returned 12 schema-valid items, 0 errors.
- Pending human verification: Paseo app composer + -> Thread -> pick a finished thread -> attachment shows its last reply; nothing sends until Send.
