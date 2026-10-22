# NEXT

## Current Active Plan

- Title: GPUI diff wrap and markdown render, board builder model, selection translate, and selectable next prompts
- Slug: 20261022-gpui-diff-wrap-markdown-render
- Status: ACTIVE
- Updated: 2026-10-22

## Tracker

One row per TASK. Group ties together items that ship as one transaction.

| Order | TASK                                                          | Group | Status | Spec                                                                                                                         | Deps               | Context                                        | Notes                                          |
| ----- | ------------------------------------------------------------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------------- | ------------------ | ---------------------------------------------- | ---------------------------------------------- |
| 1     | TASK-290 Markdown block diff model                            | C     | TODO   | [watchtower/tasks/TASK-290-markdown-block-diff-model.md](watchtower/tasks/TASK-290-markdown-block-diff-model.md)             | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Pure logic, no UI.                             |
| 2     | TASK-288 One-file diff toolbar with wrap                      | A     | TODO   | [watchtower/tasks/TASK-288-one-file-diff-toolbar-wrap.md](watchtower/tasks/TASK-288-one-file-diff-toolbar-wrap.md)           | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares diff_view.rs with TASK-291.             |
| 3     | TASK-289 File viewer rendered markdown mode                   | B     | TODO   | [watchtower/tasks/TASK-289-file-viewer-rendered-markdown.md](watchtower/tasks/TASK-289-file-viewer-rendered-markdown.md)     | TASK-290           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Uses is_markdown_path.                         |
| 4     | TASK-291 Rich markdown diff in one-file diff                  | A     | TODO   | [watchtower/tasks/TASK-291-rich-markdown-diff.md](watchtower/tasks/TASK-291-rich-markdown-diff.md)                           | TASK-288, TASK-290 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Builds on the TASK-288 toolbar.                |
| 5     | TASK-292 Board card shows builder provider, model, and effort | D     | TODO   | [watchtower/tasks/TASK-292-board-card-builder-model.md](watchtower/tasks/TASK-292-board-card-builder-model.md)               | -                  | -                                              | Watchtower board, separate from the diff work. |
| 6     | TASK-293 Host selection result card API                       | E     | TODO   | [watchtower/tasks/TASK-293-host-selection-result-card.md](watchtower/tasks/TASK-293-host-selection-result-card.md)           | -                  | -                                              | New plugin-host API and timeline card.         |
| 7     | TASK-294 selection-translate plugin server                    | F     | TODO   | [watchtower/tasks/TASK-294-selection-translate-server.md](watchtower/tasks/TASK-294-selection-translate-server.md)           | -                  | -                                              | Copies the prompt-translate LLM call.          |
| 8     | TASK-295 selection-translate GPUI client                      | G     | TODO   | [watchtower/tasks/TASK-295-selection-translate-gpui-client.md](watchtower/tasks/TASK-295-selection-translate-gpui-client.md) | TASK-293, TASK-294 | -                                              | The "Dịch" button and settings screen.         |
| 9     | TASK-296 Selectable text in next prompt cards                 | H     | TODO   | [watchtower/tasks/TASK-296-next-prompt-selectable-text.md](watchtower/tasks/TASK-296-next-prompt-selectable-text.md)         | -                  | -                                              | Prompt and reason lines become selectable.     |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-explorer` -> all tests pass, including the new markdown_diff tests.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-explorer -- -D warnings` -> no warnings.
- Owner check on a live GPUI build: open a changed `.md` file from Changes, switch to Rendered, and see added blocks green and removed blocks red; open the same file from Files and see it rendered.

## Review Focus

- A `.md` file that changed on disk after the diff loaded, so the patch no longer matches -> the view shows the line diff and a short notice, not a wrong render; owner TASK-291.
- A new untracked `.md` file (old side empty) and a deleted `.md` file (new side gone) -> new file renders all Added; deleted file hides the Rendered toggle; owner TASK-291.
- A `.md` file opened from search or a link with a line number -> it opens in Source and scrolls to the line; owner TASK-289.
- A builder whose snapshot has no model or no thinkingOptionId -> the card skips that part and never shows an empty comma; owner TASK-292.
- The selection-translate server stops while the GPUI client still shows "Dịch" -> the card shows the error and no Copy button; owner TASK-293.

## Handoff

- Next action: run `/watchtower implement`.

## Archive

- None.
