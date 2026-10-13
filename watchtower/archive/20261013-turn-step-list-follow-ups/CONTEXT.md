# Plan Context

## Shared Context

- The turn step list shipped in PR #60, merged to `main` as `145feb5c9`. The plan, its review, and its lessons are in [watchtower/archive/20261012-turn-step-list](watchtower/archive/20261012-turn-step-list), with [LEARN.md](watchtower/archive/20261012-turn-step-list/LEARN.md).
- The step model is in [packages/app/src/tool-calls/detail-level/turn/model.ts](packages/app/src/tool-calls/detail-level/turn/model.ts), the turn grouping in [packages/app/src/tool-calls/detail-level/grouping.ts](packages/app/src/tool-calls/detail-level/grouping.ts), and the view in [packages/app/src/tool-calls/detail-level/turn/view.tsx](packages/app/src/tool-calls/detail-level/turn/view.tsx).
- The edit-diffs row is drawn by [hiep-plugins/plugins/edit-diffs/client/edit-row.tsx](hiep-plugins/plugins/edit-diffs/client/edit-row.tsx). It draws in the step list and in the stream at the `detailed` level and on compact screens.
- Run app tests from `packages/app`, one file at a time: `npx vitest run <file> --bail=1`. Never pass `--dir`; it does not limit the run.
- Format plugin files with the plugin's own `npm run format`; the root `format:files` skips `hiep-plugins/`.
- A live UI check needs a window reload after a Metro error, or it shows a stale bundle.

## Decisions

- A note opens its step. See [ADR-0021](watchtower/decisions/ADR-0021-note-opens-its-step-over-tools-before-the-note.md).
- The answer rule stays: every assistant message after the last tool call. See [ADR-0018](watchtower/decisions/ADR-0018-all-trailing-messages-over-last-message-as-answer.md).
- The edit-diffs row gets back its old outer spacing in the stream; the step list cancels it, so steps keep the compact row. No new plugin API. The pencil and the verb label stay removed everywhere (owner choice, 2026-10-13).
- Label hues stay as a documented design exception with one text and border color per scheme; the change bar uses `statusSuccess` and `statusDanger`. See [ADR-0020](watchtower/decisions/ADR-0020-scheme-label-colors-over-palette-tokens-for-turn-steps.md).

- The label tones live in the theme as `theme.turnLabels`. See [ADR-0022](watchtower/decisions/ADR-0022-scheme-label-tones-in-the-theme-over-color-scheme-branches.md).

## Open Decisions

- None.

## References

- [docs/design.md](docs/design.md) §13 and §14 (identity colors exception, forbidden colors)
- [packages/app/src/styles/identity-colors.ts](packages/app/src/styles/identity-colors.ts) (`identityForeground(name, colorScheme)`, the per-scheme pattern to follow)
- [watchtower/decisions/ADR-0016-host-turn-step-list-over-plugin-group-renderer.md](watchtower/decisions/ADR-0016-host-turn-step-list-over-plugin-group-renderer.md)
