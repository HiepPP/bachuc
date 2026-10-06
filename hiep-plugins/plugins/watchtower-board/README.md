# Watchtower board for Paseo

Read Watchtower plans in a workspace panel and attach a task brief through the composer picker.
Requires the fork daemon and client 0.10.2-beta.900+, Node 22+, and enabled trusted plugins.

## Use

With a thread open, click **Watchtower** in the sidebar, below **Board**, or choose
**Open Watchtower board** in the Command Center. Both open the board in Explorer, beside
**Files** and **Changes**, for the current workspace. On the new workspace screen, the board of the
chosen project opens in the side panel; it reads the project root and has no attachment search key.
On desktop and web the sidebar item is disabled elsewhere outside a thread, such as on the Board
page. In a web window narrower than 720 px, the compact Explorer has no plugin tabs, so the sidebar
item opens the Watchtower page on the current workspace instead.
Mobile keeps a sidebar page that picks a workspace from a row sorted by recent activity.
The header identifies its project. Each workspace keeps its own board and task selection.
The board reads `watchtower/NEXT.md` in that workspace, not another checkout or its parent repository.
The panel summarizes completion and counts for Active, Blocked, Todo, and Done tasks.
**Waiting on the owner** lists the open questions that block tasks and counts proposed ADRs.
**Autorun** shows the run's runner, profile, schedule, start and finish, and its five latest
iterations, newest first. Each task row shows its class next to its ID.
Expand or collapse a status group, then select a compact task row to read its full title, brief,
dependencies, class, blocking questions, blocker, and file error. Refresh to read current files.

To attach: open composer **+ → Watchtower task**, search by workspace or task, then select the result.
The picker searches the host's 30 most recently active workspaces and returns up to 20 valid tasks.
Workspace names and paths distinguish duplicate task IDs.
For an exact task or an older workspace, select it in the board, click **Copy attachment search**,
and paste that key into the picker. This targets the exact workspace ID.
Paseo 0.8 has no public API to insert an attachment directly from a workspace panel.
The picker owns the draft attachment; only the user's normal Send action submits it.
Attachments are snapshots. Remove and reattach after changing a task brief.

## Format and limits

The Tracker table needs TASK, Status, Spec, Deps, and Context columns. Notes are optional.
Statuses are TODO, IN PROGRESS, BLOCKED, and DONE. Tasks need unique `TASK-NNN` IDs.
Spec links may be relative to NEXT.md or start with `watchtower/` from the workspace root.
Specs must have a matching H1 task ID and a non-empty `## Brief` section.
The board displays Markdown brief text without rendering links or executing content.
For BLOCKED tasks, it reads `Blocked:` from the matching task outcome, falling back to Tracker notes.
The class comes from the `Class:` line above the spec's first H2; a missing class reads as risky.
Three optional files sit beside NEXT.md, in the Watchtower skill's formats:

- `QUESTIONS.md`: rows with Status `OPEN` and a `TASK-NNN` in Blocks. Other rows are skipped.
- `RUN.md`: the `Runner`, `Profile`, `Schedule`, `Started`, and `Finished` lines and the `## Log` table.
- `DECISIONS.md`: rows of the `## Index` table with Status `proposed`.

A missing optional file hides its section; an unreadable one shows a warning. Attachment search
reads none of them. Missing files, unsupported legacy formats, invalid links, and unreadable specs
show messages.
Invalid or unavailable briefs cannot be attached.

Files must stay inside the workspace's Watchtower directory, including resolved symlink targets.
Limits: 128 KiB per Markdown file, 200 tasks, and 1 MiB of manifest and spec content per board.
The three optional files count only against the per-file limit.
No repository file is written. No agents are created or messaged. No external service or credential is needed.
The plugin reads other workspaces on the same host only when the user opens the attachment picker.

## Install and verify

```sh
npm ci
npm run format
npm run typecheck
npm run lint
npm test
paseo plugin install "$PWD" --id watchtower-board
paseo plugin reload watchtower-board
paseo plugin ls watchtower-board --json
paseo plugin logs watchtower-board
```

Confirm `running`, then exercise the board and picker in the installed desktop and compact clients.
The source directory must remain available. Disable with `paseo plugin disable watchtower-board`.
