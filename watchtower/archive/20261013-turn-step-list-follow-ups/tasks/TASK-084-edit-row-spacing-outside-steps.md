# TASK-084 Edit row spacing outside steps

Group: A (writes `edit-row.tsx` and `turn/view.tsx`, shared with TASK-085)
Class: code

## Brief

Goal: In the stream, at the `detailed` level and on compact screens, edit-diffs rows get back the outer spacing they had before PR #60. Inside a step, the row keeps its compact look.

Change: no outer spacing on every edit row -> the old outer margins in the stream, cancelled by the step list.

Boundaries:

- Owner choice, 2026-10-13: restore the old outer margins (`marginHorizontal: -13`, `marginBottom: 4`) in the plugin, and cancel them where the step list places a plugin row. Add no plugin API.
- The pencil icon and the verb label stay removed everywhere. The chevron, path, change bar, and +N −M line stay.
- The host cancels by item kind only (`plugin`), never by plugin ID ([ADR-0016](watchtower/decisions/ADR-0016-host-turn-step-list-over-plugin-group-renderer.md)). Write a comment at the cancel site that names the plugin margins it offsets.
- Paseo edit rows (edit diffs off) do not get the offset.

How:

- In [hiep-plugins/plugins/edit-diffs/client/edit-row.tsx](hiep-plugins/plugins/edit-diffs/client/edit-row.tsx), put the old outer margins back on the row container.
- In [packages/app/src/tool-calls/detail-level/turn/view.tsx](packages/app/src/tool-calls/detail-level/turn/view.tsx), wrap a `plugin` edit item in a view with equal and opposite margins.
- Add a view test that a `plugin` edit item in a step gets the offset wrapper and a Paseo edit item does not.

Files:

- [hiep-plugins/plugins/edit-diffs/client/edit-row.tsx](hiep-plugins/plugins/edit-diffs/client/edit-row.tsx) (outer margins back)
- [packages/app/src/tool-calls/detail-level/turn/view.tsx](packages/app/src/tool-calls/detail-level/turn/view.tsx) (offset wrapper for plugin edit items)
- [packages/app/src/tool-calls/detail-level/turn/view.test.tsx](packages/app/src/tool-calls/detail-level/turn/view.test.tsx) (offset wrapper case)
- [hiep-plugins/plugins/edit-diffs/README.md](hiep-plugins/plugins/edit-diffs/README.md) (one line on the spacing contract with the step list)

Expected result:

- At Full detail on live, two edit rows in a row have a 4px gap and the old side bleed, as before PR #60.
- At Summary on live, edit rows inside a step look as they do now.

Anti-goal: existing edit-diffs tests keep 0 edited assertions; tripwire; read with `git diff -U0 -- hiep-plugins/plugins/edit-diffs/tests` and `npm test` in the plugin before the change and at completion.

## Verify

- `cd hiep-plugins/plugins/edit-diffs && npm test && npm run typecheck && npm run lint` -> exit 0.
- `cd packages/app && npx vitest run src/tool-calls/detail-level/turn/view.test.tsx --bail=1` -> pass, including the offset wrapper case.
- `npm run typecheck` -> exit 0.
- `npm run cli -- plugin reload edit-diffs` -> reloads on live without an error, only when live already runs.
- Manual on live (needs a screen): the turn-step-check thread at Full detail and at Summary -> the gap is back in the stream, and steps look unchanged.
- Anti-goal: `git diff -U0 -- hiep-plugins/plugins/edit-diffs/tests` -> only added lines.
