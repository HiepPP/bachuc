# Hiep Paseo plugins

See what your agents are doing. Keep projects organized. Bring task context and
workspace checks into Paseo without leaving your conversation.

Nineteen independent local plugins. Install only what you need; each owns its
dependencies, checks, and configuration.

Gallery refreshed September 22, 2026 from Paseo desktop **0.9.0-beta.2**.
The eight refreshed images are AI-edited screenshot illustrations: Codex Image Gen
crops the UI and replaces private conversation titles, project names, profile notes,
and local paths with sample text. They are visual previews, not pixel-exact test evidence.
The Next prompt actions image is the earlier synthetic-text capture from desktop 0.8.0.

## See every conversation at a glance

Board puts running and recently finished conversations side by side, grouped into
distinct pastel project blocks with bot avatars. Star a conversation to keep its
cluster above project groups; scale the board from 10% to 200% with **UI size**.

![Privacy-edited Board preview with pastel project groups, bot avatars, running and finished states, and UI size controls.](docs/images/board.png)

Parent conversations keep their subagents in collapsible, compact card panels.
Open a card to return to its chat; child threads offer **Jump To Parent**.
Removing a finished cluster hides its cards without deleting the conversations.

[Explore Board and install →](plugins/board/README.md)

## Give each project a place

Workspace Spaces adds numbered tabs to the desktop sidebar. Move a whole project
and its sessions from the existing project menu, then switch Spaces while your
current chat stays open.
Swipe horizontally across the project list to switch adjacent Spaces. Removing
a Space moves its projects to a neighboring Space; the final Space stays protected.

<img src="docs/images/spaces.png" alt="Privacy-edited project menu with Move to workspace destinations and numbered Space tabs below." width="420">

[Set up Spaces →](plugins/workspace-spaces/README.md)

## Turn a plan into context

Watchtower board brings progress, blockers, and task briefs into Explorer. Expand
a task to read its brief, then use **composer + → Watchtower task** to attach a
snapshot to your next message. Nothing sends automatically.

<img src="docs/images/watchtower.png" alt="Watchtower empty state with task counts and an explicit missing NEXT.md message." width="360">

This capture shows the missing-file state when the repository has no active
`watchtower/NEXT.md`.

[Open your Watchtower plan →](plugins/watchtower-board/README.md)

## Read the next step, then send it

Next prompt actions turns Recap into a compact branch, summary, and commit-status strip.
Single suggestions get direct **Edit** and **Send** controls.
Structured `next-prompts` v1 suggestions use radio groups for alternative directions
and checkboxes for other steps. **Edit selected** and **Send selected** combine only
explicitly permitted selections. Continue in the same conversation without copying and pasting.

Optional Jev auto-run lives in Command Center and starts off. Desktop only;
uses a private adapter for the existing Markdown blocks.

[Set up next prompt actions →](plugins/next-prompt-actions/README.md)

## Keep your branch beside the composer

Thread branch shows the conversation's current Git branch beside its other pills.
Open the branch menu to copy its name or refresh. When a matching pull request
exists, a separate **#PR** pill opens it in one click.

![Branch main beside the subagent pill, with Copy branch name and Refresh in its menu.](docs/images/thread-branch.png)

The capture shows `main` without a PR pill. PR discovery requires authenticated
`gh`; the branch itself only requires Git.

[Set up Thread branch →](plugins/thread-branch/README.md)

## Choose the models a task can use

Jev orchestrator's **Direct Jev routing** panel lets you choose saved profiles
and allowed effort levels before **Route and run** creates one agent in the
current workspace. Profile permissions and features stay attached to that choice.

![Privacy-edited Direct Jev routing panel with an empty task field and available profiles.](docs/images/jev-orchestrator.png)

Open it from Command Center or `/route`. This capture shows profile selection;
no task was submitted. Jev routing consumes API quota.

[Explore Jev routing →](plugins/jev-orchestrator/README.md)

## Ask a focused second opinion

Jev gives Codex and Claude agents a typed evaluation tool: boolean, choice, and
score questions over state you choose to share. Your coding agent remains in control.

![Installed plugin cards including Jev evaluator and orchestrator, with local paths redacted.](docs/images/jev.png)

Jev works through MCP, not a standalone page. This capture shows installation
status, not an evaluation result. Evaluations require a configured Vercel AI
Gateway key and consume API quota.

[Try a typed Jev evaluation →](plugins/jev-evaluator/README.md)

## Keep threads tidy automatically

Thread janitor closes the runtimes of stale threads, so their provider and MCP processes
exit. By default a thread with no activity for more than 24 hours is closed. Threads stay
unarchived and resume when opened or prompted. A reconnect can resume dozens of threads, so
the janitor also sweeps 5 minutes after the last resume.

[Configure Thread janitor →](plugins/thread-janitor/README.md)

## Attach another thread's reply

Thread context attach brings a Paseo thread's last reply into your composer through
**+ → Thread**, replacing `/tmp` handoff files and copy-paste between Codex and Claude
threads. Nothing sends automatically; review the composer and press Send yourself.

[Set up Thread context attach →](plugins/thread-context-attach/README.md)

See each plugin's documentation for client support and limitations, including
Spaces' and Next prompt actions' private desktop adapters. Installed/running
status alone does not prove every plugin action works.

## Pick your plugins

| Plugin                                                           | Purpose                                                                                                                                |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| [claude-look](plugins/claude-look/README.md)                     | Claude Light theme, Claude desktop's 768px chat column, and its 1.5 reply line-height.                                                 |
| [edit-diffs](plugins/edit-diffs/README.md)                       | Show edit tool calls as diffs with file line numbers, open by default like Claude Code.                                                |
| [stop-orb](plugins/stop-orb/README.md)                           | Replace the composer stop button with the animated solving thinking orb on desktop.                                                    |
| [next-prompt-actions](plugins/next-prompt-actions/README.md)     | Send next-step prompts inside their blocks on desktop, with optional bounded Jev auto-run.                                             |
| [prompt-translate](plugins/prompt-translate/README.md)           | Show English translations under Vietnamese prompts, and rewrite drafts as English prompts with Cmd/Ctrl+Enter, on desktop.             |
| [plugins-nav](plugins/plugins-nav/README.md)                     | Add a Plugins row to the sidebar that opens the active host's Plugins settings page.                                                   |
| [skill-pins](plugins/skill-pins/README.md)                       | Pin skills such as watchtower in the desktop composer; each turn tells Claude or Codex to load them.                                   |
| [thread-branch](plugins/thread-branch/README.md)                 | Show the current branch beside the composer and open its PR through a separate pill.                                                   |
| [jev-orchestrator](plugins/jev-orchestrator/README.md)           | Direct Jev model/effort routing to one agent, plus optional scoped delegation with discovery, escalation, review, and ownership locks. |
| [Board](plugins/board/README.md)                                 | Group conversations by project with pastel colors, avatars, parent/subagent clusters, stars, and UI scaling.                           |
| [jev-evaluator](plugins/jev-evaluator/README.md)                 | Expose Jev evaluations as an MCP tool for Codex and Claude agents.                                                                     |
| [watchtower-board](plugins/watchtower-board/README.md)           | Browse read-only Watchtower tasks and attach task briefs in the composer.                                                              |
| [workspace-spaces](plugins/workspace-spaces/README.md)           | Group projects with numbered Spaces in the desktop sidebar; standalone fallback on other clients.                                      |
| [thread-janitor](plugins/thread-janitor/README.md)               | Close the runtimes of stale threads so their processes exit.                                                                           |
| [idle-runtime-closer](plugins/idle-runtime-closer/README.md)     | Close the processes of idle threads automatically; the threads stay and resume on the next prompt.                                     |
| [thread-context-attach](plugins/thread-context-attach/README.md) | Attach the last reply of another Paseo thread to your next message.                                                                    |
| [pr-watch](plugins/pr-watch/README.md)                           | Let an agent watch its GitHub PR and wake it when checks fail or pass, someone comments, or a conflict appears.                        |
| [assistant-cite](plugins/assistant-cite/README.md)               | Cite selected assistant text as a composer chip with an optional comment, and jump back to the quote from the chip.                    |
| [html-reply](plugins/html-reply/README.md)                       | Let an agent publish a sandboxed, themed HTML page that renders inline in its thread.                                                  |

## Install a plugin

```sh
cd plugins/jev-evaluator
npm ci
npm run typecheck
npm run lint
npm test
paseo plugin install "$PWD" --id jev-evaluator
```

After editing a plugin, run its checks and `paseo plugin reload <plugin-id>`.
See each plugin README for requirements and configuration.

## Add another plugin

Create `plugins/<plugin-id>/` with its own `paseo-plugin.json` and runtime entry.
Install and manage each plugin independently; no root workspace build is required.
