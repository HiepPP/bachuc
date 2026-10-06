# TASK-009 Outcome

## Outcome

Status: DONE

Changed:

- New plugin `hiep-plugins/plugins/html-reply/` with server and client entries: [index.server.ts](hiep-plugins/plugins/html-reply/index.server.ts) registers the `publish_html` tool and the `html-reply.get` RPC; [index.client.tsx](hiep-plugins/plugins/html-reply/index.client.tsx) registers the timeline renderer in [client/card.tsx](hiep-plugins/plugins/html-reply/client/card.tsx).
- Pure modules: [server/store.ts](hiep-plugins/plugins/html-reply/server/store.ts) (save, read, and remove by id, the 1 MiB check) and [shared/row.ts](hiep-plugins/plugins/html-reply/shared/row.ts) (row schema, RPC contract, title cleanup, row size).
- Tests in `hiep-plugins/plugins/html-reply/tests/`: store and row.
- [hiep-plugins/plugins/html-reply/README.md](hiep-plugins/plugins/html-reply/README.md) and a catalog row in [hiep-plugins/README.md](hiep-plugins/README.md).

Contract:

- ADR-0002 and Q-008: the page renders in `HtmlFrame` with `network: true`, the 360-point default height, and inner scroll. No extra sandbox rights.
- The row data is `{ id, title }`; the page is `$PASEO_HOME/plugin-data/html-reply/<id>.html`. Rows are not restored after a daemon restart, but the files stay (Q-008).
- HTML over 1 MiB, or an empty page, returns a tool error and adds no row. If the timeline append fails, the saved file is removed.
- Page ids must match `^[a-z0-9-]{1,64}$` before they reach the file system, because the RPC id comes from the app.
- Titles lose control characters and extra whitespace and are cut at 200 characters, so a row stays under 1 KiB.
- An unknown id renders "Page not found. Its file may have been removed."
- Native OpenCode and OMP agents cannot see the plugin tool; accepted per the spec.

Verified:

- In `hiep-plugins/plugins/html-reply`: `npm test` -> 10 passed. `npm run typecheck` -> exit 0. `npm run lint` -> 0 warnings, 0 errors. `npm run format` -> done; root `npm run format:check:files` on the plugin files -> exit 0.
- Live install (`npm run cli -- plugin install ...`, then `plugin ls`) -> UNVERIFIED (autonomous run). The live daemon on port 6768 was stopped.
- Browser check (a live agent calls `publish_html`, and a page that loads a chart library from an https CDN renders) -> UNVERIFIED (autonomous run).
- RPC size: no plugin RPC payload cap was found in the daemon source, so a 1 MiB page should load. A 1 MiB page through the relay is not checked.

Anti-goal:

- Final: test "keeps every row under 1 KiB, even for the worst-case title" passed for CJK, quote, backslash, control-character, and emoji titles; 2026-10-07 02:13.
- Result: PASS.

Lessons:

- oxlint warns on control-character ranges in a regex (`no-control-regex`); a code-point check avoids the warning and a disable comment.
