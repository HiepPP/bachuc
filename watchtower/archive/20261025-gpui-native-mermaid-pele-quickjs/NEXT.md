# NEXT

## Current Active Plan

- Title: Native Mermaid in GPUI with Pele on QuickJS
- Slug: 20261025-gpui-native-mermaid-pele-quickjs
- Status: ARCHIVED
- Updated: 2026-10-25

## Tracker

| Order | TASK                                                  | Group | Status | Spec                                                                                                                                         | Deps               | Context                                        | Notes                                                    |
| ----- | ----------------------------------------------------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ | ---------------------------------------------- | -------------------------------------------------------- |
| 1     | TASK-301 Spike Pele on rquickjs                       | A     | DONE   | [watchtower/tasks/TASK-301-spike-pele-on-rquickjs.md](watchtower/tasks/TASK-301-spike-pele-on-rquickjs.md)                                   | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Go or no-go gate. If it fails, stop the plan.            |
| 2     | TASK-302 ADR-0067 supersedes ADR-0037                 | B     | DONE   | [watchtower/tasks/TASK-302-adr-0067-supersedes-adr-0037.md](watchtower/tasks/TASK-302-adr-0067-supersedes-adr-0037.md)                       | TASK-301           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Start order only; no shared file. Write only after a go. |
| 3     | TASK-303 Pele engine, theme vars, resvg PNG           | A     | DONE   | [watchtower/tasks/TASK-303-pele-engine-theme-vars-resvg-png.md](watchtower/tasks/TASK-303-pele-engine-theme-vars-resvg-png.md)               | TASK-301, TASK-302 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | UI check PASS (Unicode labels).                          |
| 4     | TASK-304 Mermaid through Pele, WebView for KaTeX only | A     | DONE   | [watchtower/tasks/TASK-304-mermaid-through-pele-webview-katex-only.md](watchtower/tasks/TASK-304-mermaid-through-pele-webview-katex-only.md) | TASK-303           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-191 answered: light and dark checked by Hiep.          |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.

## Plan Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-diagram -p bachuc-timeline` -> all tests pass.
- `rg -n "mermaid.min.js|renderMermaid|MERMAID_WIDTH" packages/desktop-gpui/crates/diagram/src` -> no match.
- Screen check (needs a human or the UI checker): open the `diagrams` example or the live app on a turn with the flowchart, sequence, and state samples and a KaTeX formula -> the three diagrams show sharp text inside their boxes, in theme colors, and the formula still shows.
- After the checks, `./cargo.sh clean --profile dev` per [CLAUDE.md](CLAUDE.md), unless another cargo process runs.

## Review Focus

- A Mermaid source that Pele cannot parse, for example `flowchart LR\n A --> (` -> the row shows the source as a code block with the error text; owner TASK-304.
- The theme changes from light to dark, or to a custom theme, while diagrams are cached or a Mermaid job runs on the engine thread -> every diagram renders again in the new colors, and the stale result is dropped; owner TASK-304.
- A very large source, over Pele's size limit -> Pele returns an error, the row shows a code block, and the app does not hang; owner TASK-303.
- A Mermaid row and a KaTeX row in the same turn, before the WebView is ready -> the Mermaid row renders without waiting for the WebView; owner TASK-304.
- A diagram with a non-Latin label, for example Vietnamese text -> usvg finds a font and draws the label; owner TASK-303.

## Handoff

- Next action: none. The plan is archived.

## Archive

- Archived: 2026-10-25 -> watchtower/archive/20261025-gpui-native-mermaid-pele-quickjs/
