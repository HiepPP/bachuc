# TASK-074 Edit diffs row inside a step

Group: C (writes hiep-plugins/plugins/edit-diffs only; standalone)
Class: code

## Brief

Goal: Make the edit-diffs row fit inside an Edit step of the turn step list, as in the final design.

Change: a standalone edit row with a large gap -> a compact file row (chevron, path, change bar, +/−) with the diff open below.

Design: [watchtower/CONTEXT.md](watchtower/CONTEXT.md) (design link; see the Edit step row in its Spec table)

Boundaries:

- Follows [ADR-0016](watchtower/decisions/ADR-0016-host-turn-step-list-over-plugin-group-renderer.md): edit-diffs keeps drawing the diff; the host only places the row.
- The diff is open by default. A click on the file row hides or shows it. Keep the existing "Auto-expand edit diffs" setting as the default source.
- The change bar has 7 squares of 7px, split by the added and removed share, green #3D9A4E and red #C94A4A.
- Keep the thread on/off switch and the line number lookup unchanged.

How:

- Update [hiep-plugins/plugins/edit-diffs/client/edit-row.tsx](hiep-plugins/plugins/edit-diffs/client/edit-row.tsx): file row layout, change bar, remove outer spacing that the step already gives.
- Add or extend a test under [hiep-plugins/plugins/edit-diffs/tests](hiep-plugins/plugins/edit-diffs/tests) for the change bar split.
- Reload the plugin on live: `npm run cli -- plugin reload edit-diffs`.

Files:

- [hiep-plugins/plugins/edit-diffs/client/edit-row.tsx](hiep-plugins/plugins/edit-diffs/client/edit-row.tsx) (row layout and change bar)
- [hiep-plugins/plugins/edit-diffs/tests](hiep-plugins/plugins/edit-diffs/tests) (change bar test)
- [hiep-plugins/plugins/edit-diffs/README.md](hiep-plugins/plugins/edit-diffs/README.md) (one line on the change bar)

Expected result:

- An edit row shows the chevron, the path, the change bar, and +N −M on one line, with the diff open below.
- A click on the file row hides the diff; a second click shows it.
- Cases: a write (all added lines) -> 7 green squares; a pure delete -> 7 red squares; a thread with edit diffs off -> Paseo rows, unchanged.

Anti-goal: the existing edit-diffs tests pass with 0 edited assertions; tripwire; read with `git diff` and `npm test` in the plugin before changes and at completion.

## Verify

- `cd hiep-plugins/plugins/edit-diffs && npm test` -> pass, including the change bar cases.
- `cd hiep-plugins/plugins/edit-diffs && npm run typecheck && npm run lint` -> exit 0.
- `npm run cli -- plugin reload edit-diffs` -> the plugin reloads on live without an error.
- Manual on live (needs a screen): an agent edit shows the new file row with the diff open.
- Anti-goal: `git diff -U0 -- hiep-plugins/plugins/edit-diffs/tests` -> only added lines.
