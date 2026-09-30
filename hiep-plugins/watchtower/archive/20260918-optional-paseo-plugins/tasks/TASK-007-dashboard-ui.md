# TASK-007 Dashboard UI

Group: A (shares the Watchtower board client with TASK-005)
Class: code

## Brief

Goal: Apply the reference Watchtower dashboard layout to the Paseo plugin.

Change: Replace large task cards with a compact summary and grouped task rows.

How:

- Follow the [reference image](../../../../../watchtower/media/watchtower-dashboard.png) from the local Watchtower extension.
- Show a compact plan header, refresh action, completion progress, and status counts.
- Group tasks by Active, Blocked, Todo, and Done with clear status badges.
- Show the selected brief, dependencies, errors, and full blockers through task details.
- Adapt spacing and text to wide and narrow panels using Paseo theme tokens.
- Preserve read-only operations and native composer attachments. Do not send prompts automatically.
- Include exact search keys in scoped attachment metadata so the host picker can match them.
- Use the existing Paseo 0.8 SDK. Do not add VS Code controls or other plugin features.

Files:

- [Client](../../../../plugins/watchtower-board/client): Render the dashboard and task details.
- [Attachment handler](../../../../plugins/watchtower-board/server/handlers.ts): Keep scoped results visible in the native picker.
- [Tests](../../../../plugins/watchtower-board/tests): Check progress and grouping behavior where applicable.
- [Plugin README](../../../../plugins/watchtower-board/README.md): Explain the updated interface.

Expected result:

- Users can scan progress and task status without large cards or long blocker logs.
- Task selection, refresh, errors, and attachment search remain usable on narrow panels.

## Verify

- Run `npm run format`, `npm run typecheck`, `npm run lint`, and `npm test` in the plugin directory.
- Check that mixed task statuses produce correct counts, group membership, and completion progress.
- Check that zero completed tasks have an empty progress fill.
- Reload `watchtower-board` on the local host and confirm `running` with no load error.
- Exercise selection, group disclosure, refresh, and attachment selection without sending a prompt.
- Compare wide and narrow layouts with the reference hierarchy. Check theme legibility and error states.
- Remove task-owned test attachments and restore temporary UI settings. Record any missing runtime proof.
