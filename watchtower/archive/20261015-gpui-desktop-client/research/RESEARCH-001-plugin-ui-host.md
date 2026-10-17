# RESEARCH-001 How should the GPUI client host plugin UI?

## Meta

Question: How should the GPUI client host plugin UI, so a later plan can port the 16 client plugins?
Date: 2026-10-16
Scope: `hiep-plugins/plugins/*/index.client.tsx`, `packages/plugin/src/client/contracts.ts`, `packages/desktop-gpui`
Status: DONE
Related: TASK-143

## Summary

The 16 plugins make 14 kinds of contribution, and most are small data items (pills, commands, settings, attachment sources). Only pages, panels, and some renderers need a free-form surface. Host the data kinds natively in GPUI and decide the free-form kinds per plugin, with a hidden-WebView PNG snapshot as the fallback (see ADR-0038).

## Core Evidence

`packages/plugin/src/client/contracts.ts:117`

The client contract is a set of `add*` calls. Each call takes a React `ComponentType` or a plain data object, for example `addSurface(id, Component, options)` at line 119 and `addComposerPill(contribution)` at line 137.

Count method: `rg -o "client\.(add|open)[A-Za-z]+\(" hiep-plugins/plugins/*/index.client.tsx`, one plugin counted once per kind. All 16 entries contribute UI.

| Contribution kind                                                                        | Plugins | Which                                                                                          | Needs free-form UI |
| ---------------------------------------------------------------------------------------- | ------- | ---------------------------------------------------------------------------------------------- | ------------------ |
| Page (`addSurface`)                                                                      | 4       | board, plugins-nav, watchtower-board, workspace-spaces                                         | yes                |
| Workspace panel (`addWorkspacePanel`)                                                    | 2       | jev-orchestrator, thread-branch                                                                | yes                |
| Timeline renderer (`addTimelineRenderer`)                                                | 6       | edit-diffs, html-reply, jev-orchestrator, next-prompt-actions, prompt-translate, thread-branch | mixed              |
| Timeline transformer (`addTimelineTransformer`)                                          | 3       | edit-diffs, next-prompt-actions, prompt-translate                                              | no (data)          |
| Settings screen (`addSettingsScreen`)                                                    | 5       | board, edit-diffs, next-prompt-actions, prompt-translate, skill-pins                           | form               |
| Composer pill (`addComposerPill`)                                                        | 4       | idle-runtime-closer, prompt-translate, skill-pins, watchtower-board                            | small              |
| Attachment source (`addAttachmentSource`)                                                | 4       | assistant-cite, thread-branch, thread-context-attach, watchtower-board                         | list picker        |
| Command center item (`addCommandCenterItem`)                                             | 5       | edit-diffs, jev-orchestrator, next-prompt-actions, thread-branch, watchtower-board             | no (data)          |
| Sidebar item (`addSidebarItem`)                                                          | 2       | board, plugins-nav                                                                             | no (data)          |
| Sidebar section, project filter, project menu (`addSidebar*`)                            | 1       | workspace-spaces                                                                               | small              |
| Slash command (`addSlashCommand`)                                                        | 1       | jev-orchestrator                                                                               | no (data)          |
| Composer stop button and interceptor (`addComposerStopButton`, `addComposerInterceptor`) | 2       | stop-orb, prompt-translate                                                                     | small              |
| Selection action and header subtitle                                                     | 2       | assistant-cite (selection action), board (header subtitle)                                     | small              |
| Theme (`addTheme`)                                                                       | 1       | claude-look                                                                                    | no (data)          |

Measured facts:

- One hidden WebView for diagrams adds about 55 to 83 MB of footprint and about 0% CPU when idle ([TASK-137 outcome](../tasks/TASK-137-outcome.md), [TASK-141 outcome](../tasks/TASK-141-outcome.md)).
- The app shell reads 9.7% CPU on a young stream and 13.4% on a 15-minute turn (Q-065). The 10% budget has almost no margin, so each live surface must cost near 0%.
- Live WebViews do not clip inside GPUI scroll lists ([ADR-0037](../decisions/ADR-0037-hidden-webview-png-snapshots-over-svg-and-inline-webviews.md)).

## Runtime Flow

1. The server builds each client entry to a React bundle ([compiler.ts](../../packages/server/src/server/plugins/compiler.ts)). The bundle calls `client.add*` on load.
2. In the Electron app, React renders the returned components into the app tree. GPUI has no React.
3. Any GPUI option must replace step 2 for each contribution kind.

## ASCII Flow

```text
bundle -> client.add*(...) -> [data item]      -> GPUI native view   (A)
                           -> [Component]      -> WebView surface    (B)
                           -> [declarative UI] -> GPUI renders tree  (C)
                           -> [rewrite]        -> Rust per plugin    (D)
```

## Options

Effort uses the plugin counts above. Memory is per open surface.

| Option                                 | Effort per kind                                                                                          | Memory per open surface            | Risk                                                                                          |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------- | ---------------------------------- | --------------------------------------------------------------------------------------------- |
| B. React in a WebView per surface      | Low for pages and panels (6 plugins); high for data kinds (a bridge for each of 9 kinds)                 | 55 to 83 MB each, near 0% CPU idle | Live WebView does not clip in scroll lists (ADR-0037); each pane costs memory; no native a11y |
| C. Declarative UI that GPUI renders    | One contract and renderer once; then per plugin a rewrite of components to the contract                  | Native, a few MB                   | New contract to design; free-form UIs (board, watchtower-board) need many widget types        |
| D. Move into host or rewrite in Rust   | High for every kind; 16 plugins                                                                          | Native                             | Loses plugin hot reload; ADR-0017 allows it for host-bound features                           |
| A. Native data kinds + C or B for rest | Data kinds (10 of 14) are small natives; free-form kinds (4 plugins pages, 2 panels) use B behind a pane | Only open free-form panes cost     | Two systems to keep in step                                                                   |

Treatment of ADR-0037 for B: a plugin pane is a `TabContent` variant that owns one live WebView in the tab area, outside any scroll list. Timeline renderers use the PNG snapshot path of ADR-0037, not a live view.

## Open Questions

- Do board and watchtower-board fit a declarative contract, or do they keep a WebView? Port one page first to find out.
- Does a visible WebView pane keep near 0% CPU while it redraws? Only the hidden idle case is measured.

## File References

- `packages/plugin/src/client/contracts.ts:117` - the contribution calls the plugins use.
- `hiep-plugins/plugins/*/index.client.tsx` - 16 entries, each with at least one `client.add*` call.
- `watchtower/tasks/TASK-141-outcome.md` - shell chrome views hold a `WeakEntity<Shell>`; a plugin pane would be another `TabContent` variant.
- `watchtower/tasks/TASK-137-outcome.md` - hidden WebView cost facts.
