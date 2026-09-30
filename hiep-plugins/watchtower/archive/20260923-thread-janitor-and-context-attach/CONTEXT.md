# Plan Context

## Shared Context

- Both plugins are new, independent plugins under `plugins/<plugin-id>/`. Follow the layout and install steps in [AGENTS.md](AGENTS.md).
- Copy the package setup from [plugins/board/package.json](plugins/board/package.json). SDK packages are pinned to `0.9.0-beta.2`.
- Server code gets `PaseoApi` only inside hook handlers (`server.on`) and RPC handlers (`server.handle`). There is no `paseo` object at plugin start. See `PluginHookContext` and `PluginHandlerContext` in `node_modules/@getpaseo/plugin/dist/server/`.
- `PaseoApi.agents.list()` lists agents. `agents.ref(id).archive()` archives one. `agents.ref(id).timeline` fetches timeline pages.
- Settings follow the `server.registerSettings` pattern in [plugins/board/index.server.ts](plugins/board/index.server.ts).
- Attachment sources follow the `defineAttachmentSource` pattern in [plugins/watchtower-board/shared/board.ts](plugins/watchtower-board/shared/board.ts).
- Usage data from 2026-09-23: 299 agents, 166 not archived, 181 idle, 26 throwaway test threads.

## Decisions

- Janitor archives threads idle for 24 hours by default. It does not ask for confirmation.
- Archive is reversible in Paseo (unarchive). The janitor never deletes agents or files.
- Plugin IDs: `thread-janitor` and `thread-context-attach`.

## Open Decisions

- None.

## References

- [AGENTS.md](AGENTS.md)
- [watchtower/MEMORY.md](watchtower/MEMORY.md)
- `paseo plugin reload <plugin-id>`, `paseo plugin ls <plugin-id> --json`, `paseo plugin logs <plugin-id>`
