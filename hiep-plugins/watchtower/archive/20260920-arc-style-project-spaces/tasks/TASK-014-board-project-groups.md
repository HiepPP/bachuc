# TASK-014 Group Board conversations by project

Group: A (shares Board files with TASK-006)
Class: code

## Brief

Goal: Keep conversations from the same project together, with starred conversations above all project groups.

Change: Each flat status column becomes a starred section followed by project groups.

How:

- Use the [Codex image concept](../../../designs/TASK-014-board-project-groups.png) as the visual proposal.
- The [generation prompt](../../../designs/TASK-014-board-project-groups-prompt.txt) records the built-in image_gen request.
- Keep Running and Just finished as separate columns.
- Split starred cards from unstarred cards before grouping. Render every conversation exactly once.
- Show starred cards first, outside project containers, with their project labels.
- Group remaining cards by project. Show each project name and its unstarred card count.
- Preserve existing card order within sections. Order project groups by their first card in the current list.
- Hide empty groups and the empty starred section. Column totals include starred and unstarred cards.
- Unstarring returns a card to its project. Status transitions retain its star.
- Reuse the existing star RPC and in-memory lifetime. Do not add persistent settings.
- Check host project identity before grouping. Separate distinct projects with matching display names.
- Keep conversation navigation, Remove, Needs input, live updates, and connection states.
- Preserve themes, compact stacking, and UI scaling. Keep the existing fixed UI size toolbar.
- Do not add collapse behavior from the mockup chevrons.

Files:

- [Board page](../../../../plugins/board/client/page.tsx): render sections and project headers.
- [Shared contracts](../../../../plugins/board/shared/board.ts): grouping support and project identity if needed.
- [Snapshot](../../../../plugins/board/server/snapshot.ts) and [store](../../../../plugins/board/server/store.ts): retain project identity if needed.
- [Tests](../../../../plugins/board/tests): cover grouping, star transitions, and duplicate-name projects.
- [README](../../../../plugins/board/README.md): describe grouping and starred placement.

Expected result:

- Cards from one project sit together within each status column.
- Starred cards sit above project groups, without duplicate cards or incorrect counts.

## Verify

- Run `npm --prefix plugins/board run format`, then inspect the diff for unrelated changes.
- Run `npm --prefix plugins/board run typecheck`, `npm --prefix plugins/board run lint`, and `npm --prefix plugins/board test` -> pass.
- Test mixed projects, identical project names, all-starred, no-starred, and empty columns -> correct membership and counts.
- Test star, unstar, and running-to-finished transitions -> one card per conversation in the correct section.
- Reload Board and inspect plugin status/logs -> running without load errors.
- In the live UI, star and unstar cards -> placement and counts update across connected clients.
- Check navigation, Remove, Needs input, light/dark themes, compact layout, and UI scaling -> existing actions remain usable.
- Record tested clients and limitations. Reload still resets stars and observed history.
