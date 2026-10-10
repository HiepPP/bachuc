# Watchtower board for Paseo

Read Watchtower plans in a workspace panel or a process overview, act on what waits for you, and
attach a task brief through the composer picker.
Requires the fork daemon and client 0.10.2-beta.900+, Node 22+, and enabled trusted plugins.

## Use

The board has one view, the Watchtower page. Open it with the **Watchtower** pill above an agent
composer, with **Open Watchtower board** in the Command Center, or with Cmd+G (Ctrl+G off macOS).
All three open it over the current screen on that workspace, and Cmd+G closes it again. Cmd+G
needs a host built on or after 2026-10-10. The plugin adds no sidebar item and no Explorer panel.
On the new workspace screen, the pill shows the chosen project's tasks in the side panel instead; it
reads the project root, and it has no run, agent, or attachment search key.
The Watchtower page shows one workspace only, with no workspace picker. If the opener names a
workspace, the page shows that one or says "Workspace not found."; it never shows another. If no
opener named one, such as after a plugin reload, it shows the most recently active workspace.
The header identifies its project. Each workspace keeps its own board and task selection.
The board reads `watchtower/NEXT.md` in that workspace, not another checkout or its parent repository.
Each task row shows its class next to its ID. Expand or collapse a status group, then select a
compact task row to read its full title, brief, dependencies, class, blocking questions, blocker,
and file error. Refresh to read current files.

The **Watchtower** pill above every agent composer, beside Tasks and Subagents, opens the
Watchtower page over the current screen, on that composer's workspace. A wide layout shows it as an
overlay with a dimmed backdrop; a compact layout shows a page. The pill reads `Watchtower 5/10`,
adds `, 1 question` while an open question blocks a task, and reads `Watchtower` on New workspace,
which has no workspace yet; there a press shows the chosen project's board in the side panel.
From 720 px wide the page has a fixed 360 px left rail, the former Explorer board, and the
timeline-first view on its right; each side scrolls on its own. Below 720 px, such as on a phone,
everything stacks in one scroll. The rail reads the board RPC, and the right side reads
`watchtower.overview.read`. The rail shows:

- the completion percentage and the Active, Blocked, Todo, and Done counts;
- **Waiting on the owner**: the open questions that block tasks and the count of proposed ADRs;
- **Tasks**: the status groups, Done collapsed; select a task for its brief and attachment search;
- **Autorun**: runner, profile, schedule, start and finish, and every log row, newest first.

The right side lays itself out by its own width: three columns from 900 px, and the timeline is
one strip below 520 px. It shows:

- the lifecycle: Plan, Implement, Verify, Review, and Archive, with the current step marked;
- the Autorun strip: run state (running, stopped, or finished), runner, start, iterations, and last
  activity;
- the branch timeline: done, running, and planned tasks on lanes, linked to their deps, with a now
  line, hour ticks, the estimated finish, and four pace stats. An idle gap longer than
  max(2 h, 4 x the average task) collapses to a 48 px break labeled with its length, such as
  `2d 23h`, and each period after a break shows its day. Below 520 px the timeline is one strip;
- **Running now**: each running task with its minutes against the average, and **Up next**: planned
  and blocked tasks in order with their expected start;
- **Needs you**: open and defaulted questions, proposed ADRs, and pending manual checks, each with
  actions (below). A blocking question shows the time to answer by;
- the pending manual checks;
- **Decisions**: every ADR, proposed first, then newest ID first. Select one to read its file in a
  dialog. The dialog shows headings, bullets, paragraphs, and code as plain text, with no links or
  HTML. It reads `watchtower/decisions/<ID>.md` or `<ID>-*.md` through `watchtower.decision.read`;
- **Archived plans**: the 20 newest archive folders.

The Autorun log, Needs you, Manual checks, Decisions, and Archived plans list five rows. **Show N more** expands a
list, and **Show less** collapses it.

The Needs you actions edit no file themselves. Each one sends a prompt that starts with
`Watchtower:` and names the watchtower skill to an agent of the same workspace, and that agent
changes the files. The target is the most recently updated agent that is not archived, not a
subagent, and not working: a prompt sent to a working agent interrupts its turn, and the client API
cannot ask to queue it instead. If every agent is working, the actions are disabled and the cell
reads "Agent busy". If the workspace has no agent, it reads "No agent in this workspace". A toast
confirms "Sent to <agent>" or shows the error. The cell shows the target agent above its rows.

- A question has **Answer**, which opens a text field with **Send** and **Cancel**, and, when the
  question has a default, **Use default**.
- A proposed ADR has **View** (opens the dialog), **Accept**, and **Reject**. Reject asks for a
  second press, **Confirm reject**.
- The manual checks row has **Mark done**.

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

The overview also reads the Tracker Group column, `## Plan Verify` bullets, `## Handoff` bullets
that start with `Manual check pending:`, every `RUN.md` log row, `DEFAULTED` questions, every ADR
row, and the `Title:` and `LEARN.md` of the 20 newest real folders in `archive/`. Log times count
from `Started:`. A clock time such as `22:10` rolls over at midnight once; write a dated time such
as `2026-10-09 22:10` after a gap of a day or more. The run log shows only the clock part. A
running task starts at the latest log end. Planned
tasks run one at a time in Tracker order at the average done duration, or 20 minutes.

A missing optional file hides its section; an unreadable one shows a warning. Attachment search
reads none of them. Missing files, unsupported legacy formats, invalid links, and unreadable specs
show messages.
Invalid or unavailable briefs cannot be attached.

Files must stay inside the workspace's Watchtower directory, including resolved symlink targets.
Limits: 128 KiB per Markdown file, 200 tasks, and 1 MiB of manifest and spec content per board.
The three optional files count only against the per-file limit. The overview's own reads stop at
1 MiB. An ADR file read follows the same 128 KiB and inside-Watchtower rules.
The plugin writes no repository file and creates no agent. It messages an existing agent only when
the owner presses a Needs you action. No external service or credential is needed.
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
