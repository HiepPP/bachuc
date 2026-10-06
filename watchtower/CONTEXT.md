# Plan Context

## Shared Context

- The source is T3 Code at commit `9bd1d800` (2026-10-06). A scratch clone may sit at `/tmp/t3code`. If it is gone, run `git clone https://github.com/pingdotgg/t3code.git /tmp/t3code`.
- Each TASK names the T3 Code files it copies ideas from. Copy behavior, not code. T3 Code uses Effect; Paseo does not.
- Fork rule: build a feature in a plugin under `hiep-plugins/plugins/<id>`. Add only the smallest generic host API in `packages/*` when a plugin cannot reach. Read [hiep-plugins/AGENTS.md](hiep-plugins/AGENTS.md) before plugin work.
- New plugins copy the layout and scripts of [hiep-plugins/plugins/thread-branch/package.json](hiep-plugins/plugins/thread-branch/package.json): `npm test` runs `tsx --test tests/*.test.ts`. Set `requirements.paseo` to `>=0.10.2-beta.900`.
- Protocol rules come from [docs/protocol-compatibility.md](docs/protocol-compatibility.md) and [docs/rpc-namespacing.md](docs/rpc-namespacing.md). New fields are optional. Wire schemas have no `.transform()`, `.catch()`, or `.preprocess()`. New RPCs use dotted names with `.request` and `.response`. Gate a feature once on `server_info.features.*` and tag each shim with `COMPAT(name)`.
- Run GitNexus `impact` before you edit a symbol. Warn on HIGH or CRITICAL risk.
- Run only the test files you changed: `npx vitest run <file> --bail=1`. Never run a whole workspace suite.
- App browser tests run from `packages/app` with `npx vitest run --project browser <file> --bail=1`.
- If typecheck fails across packages, rebuild first: `npm run build:client` or `npm run build:server`.
- New UI strings go into every locale file under `packages/app/src/i18n/resources/`. See [docs/i18n.md](docs/i18n.md).
- Test on live (port 6768, `npm run cli -- ...`). Never restart or stop the release daemon on port 6767.
- Integrate docs changes into the doc that owns the subject, per the root `CLAUDE.md` rules.

## Decisions

- A3 ships as two TASKs: the daemon queue (TASK-003) and the app switch (TASK-004). The split keeps each review small.
- B3 and B4 each ship as a thin host API plus a plugin. Plugin client code cannot read a text selection, insert at the composer cursor, or load a WebView today.
- A7 ships as a plugin only. A server plugin can already register MCP tools, send prompts, run timers, and write `plugin-data` files.
- A2 changes only `cancelAgentRunCommand`. `AgentManager.cancelAgentRun` has about 8 callers, such as reload, replace, rewind, and archive.
- B3 uses a composer chip with an optional comment and a jump back to the source (Q-007). The jump back is its own host TASK, TASK-010, to keep reviews small.
- HTML reply frames may use the network, with a sandbox of `allow-scripts` only. See [ADR-0002](watchtower/decisions/ADR-0002-networked-html-replies-over-offline-only.md).
- The owner commits the open cmd+w work in this checkout before the autorun starts. The run needs a clean tree.

## Open Decisions

- None. Owner questions live in [watchtower/QUESTIONS.md](watchtower/QUESTIONS.md).

## References

- [docs/agent-lifecycle.md](docs/agent-lifecycle.md)
- [docs/plugins.md](docs/plugins.md)
- [public-docs/plugins/reference.md](public-docs/plugins/reference.md)
- [docs/data-model.md](docs/data-model.md)
- [docs/floating-panels.md](docs/floating-panels.md)
