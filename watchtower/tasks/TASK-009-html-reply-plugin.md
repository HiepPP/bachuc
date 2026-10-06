# TASK-009 HTML reply plugin

Group: E (standalone: new folder `hiep-plugins/plugins/html-reply/`)
Class: code

## Brief

Goal: An agent calls `publish_html({ title, html })`. The page renders inline in that agent's thread as a plugin timeline row.

Change: an agent can only answer with Markdown -> an agent can also show a themed, sandboxed HTML page in the thread.

Design: T3 Code `apps/server/src/htmlRender/HtmlRender.ts:94` and `apps/web/src/components/chat/HtmlRenderFrame.tsx:27`.

Boundaries:

- Needs `HtmlFrame` from TASK-008.
- Register `publish_html` with `server.registerTool`. Append the row with `paseo.agents.ref(id).timeline.append({ type: "plugin", id, kind, version, data })`. Render it with `client.addTimelineRenderer`.
- Row data holds only `{ id, title }`, because row data is capped at 64 KiB ([packages/server/src/server/agent/agent-timeline-content.ts](packages/server/src/server/agent/agent-timeline-content.ts), lines 6-11).
- Save the HTML in `$PASEO_HOME/plugin-data/html-reply/<id>.html`. The renderer loads it through a plugin RPC with `useRpc`.
- Reject HTML larger than 1 MiB with a clear tool error.
- Follows [ADR-0002](watchtower/decisions/ADR-0002-networked-html-replies-over-offline-only.md): render with `HtmlFrame` and `network: true`, while HTML replies need CDN libraries and API calls. Never ask for more sandbox rights.
- Q-008 answered: network on, fixed height with inner scroll. Rows are not restored after a daemon restart, but the file stays.
- Native OpenCode and OMP agents cannot see plugin tools ([docs/plugins.md](docs/plugins.md), lines 342-344). Accept this.

How:

- Scaffold the plugin from the thread-branch layout, with server and client entries.
- Write pure modules: `server/store.ts` (save and read by id, size check) and `shared/row.ts` (row schema).
- In `index.server.ts`, register `publish_html` and the RPC that returns HTML by id.
- In `index.client.tsx`, register the timeline renderer that shows the title and an `HtmlFrame` with `network: true`.
- Write tests for the store, the size limit, and the row data size.
- Write a README. Install and reload on live only.

Files:

- `hiep-plugins/plugins/html-reply/paseo-plugin.json`, `package.json`, `package-lock.json`, `tsconfig.json` (new plugin)
- `hiep-plugins/plugins/html-reply/index.server.ts` (tool and RPC)
- `hiep-plugins/plugins/html-reply/index.client.tsx` (timeline renderer)
- `hiep-plugins/plugins/html-reply/server/store.ts`, `shared/row.ts` (pure modules)
- `hiep-plugins/plugins/html-reply/tests/*.test.ts` (tests)
- `hiep-plugins/plugins/html-reply/README.md`

Expected result:

- An agent calls `publish_html` -> a row with the title and the rendered page shows in its thread.
- HTML over 1 MiB -> the tool returns an error, and no row is added.
- An unknown id in the RPC -> the row shows a short "page not found" state.
- A page that loads a chart library from an `https:` CDN renders the chart.

Anti-goal: Timeline row data stays under 1 KiB per row; tripwire; read from a row-size test in `tests/` at completion.

## Verify

- In `hiep-plugins/plugins/html-reply`: `npm test` -> pass, including the row-size case (anti-goal read).
- In `hiep-plugins/plugins/html-reply`: `npm run typecheck` -> exit 0.
- In `hiep-plugins/plugins/html-reply`: `npm run lint` -> exit 0.
- Needs the live daemon: `npm run cli -- plugin install "$PWD/hiep-plugins/plugins/html-reply" --id html-reply`, then `npm run cli -- plugin ls` -> `html-reply` is listed and enabled.
- Browser check (verifier or human, needs live Electron or web): a live agent calls `publish_html` -> the page renders in its thread.
