# Board

Paseo conversation board. One Board shows all configured hosts together. Running and Just finished form the two status columns; every host lane names them with their counts. Each host is a horizontal lane that opens with a one-line rail: a device icon (a laptop for names containing MacBook, laptop, or notebook; a generic machine otherwise), the host name without its `.local` suffix, a connection dot and state, a hairline, and the running and finished counts. On desktop the rail stays in view while its lane scrolls. Cards are the only surface: the project label, the title, one muted line with outcome, time, and duration, then the model and effort tags. Status color appears only for attention and non-success outcomes. The only motion is the thinking orb, the host spinner, and the needs-input pulse; the pulse stops when the system asks for reduced motion. There are no host columns or host tabs. Compact clients stack the two status columns inside each lane. Offline, stale, loading, and unavailable states appear inside the owning lane.
Requires the Paseo fork `>=0.10.2-beta.900` with host-targeted plugin RPC. Host discovery uses `useHosts`; calls borrow the app's existing connections and target the Board installation on each host. No extra connections or credentials are created. A missing installation or offline host stays unavailable; calls never fall through to another host.

## Behavior

- Use **− / +** at the right of the Board toolbar to scale Board text, cards, and spacing from 10% to 200% in 10% steps. 100% is the compact default (the former 70%). Click the percentage to reset to 100%. The control sits in a fixed toolbar outside the scrolling, scaled content, keeping its size and click position unchanged; Paseo navigation and other pages are unaffected. The size is saved in host settings, so it survives plugin reloads and app restarts and is shared by clients of the same host.

- Parent and subagent conversations form nested clusters with a **subagents** toggle. Clusters start expanded; collapse state lasts while Board stays mounted, including refreshes and column changes. Compact layouts use a smaller indent. Collapsed groups summarize how many members are running or need input; subagents are rows inside the parent card and omit the project when it matches the parent. Two or more subagents use two columns when the card is wide enough.
- Relationships come from Paseo parent metadata, never titles or project names. Missing or removed parents leave their children visible as standalone cards. Provider-internal subagents without a separate Paseo conversation are not listed.
- A cluster stays in Running while any member runs; otherwise it moves to Just finished, ordered by its latest completion. Counts include all conversations, even collapsed descendants. Each card keeps its own status, star and open actions. Remove appears once a card and all of its subagents have finished, labelled **Remove all · N** on parents; removing a card also removes its finished subagents on all connected clients. Removing from the thread view while a subagent is still running keeps that subagent and its own subtree visible as standalone cards.
- Cards are flat, single-border surfaces tinted in their project's color. Each card opens with its project: the project mark (rounded square with the project's initial, as in the sidebar) and the name in the same hue, dark enough for 4.5:1 contrast on the card. Click it to start a conversation in that project. Star sits at the right of that row; Remove sits at the right of the model and effort row once the card and its subagents have finished. Running cards show a small spinner beside the status; needs-input cards show a warning badge and failed cards a red status label. Subagents render as compact rows inside their parent card, each led by a small elbow mark and a local bot avatar. The panel uses two columns when its measured width permits, otherwise one; nested groups and UI scaling follow the same rule. Avatars are stable per conversation ID (20 shared designs, cropped from one generated 5 × 4 grid; no remote image requests). Child metadata combines provider, status, and duration; differing projects retain a project line. Needs-input children have an amber border. Subagent Star and finished Remove actions appear on hover or keyboard focus on web and stay visible on touch and native clients; icon-only Remove retains its full accessible cascade label. Keyboard focus draws a 2px ring in the theme accent around the focused card or control; clicks do not draw it.
- Unstarred clusters are grouped by host project identity within each column; cards of one project stay together and each carries the project label. Same-name projects stay separate; when placement is unavailable, the full working directory identifies the group. Empty groups disappear.
- Project mark colors are saved in host settings by Paseo project ID. New IDs receive the pastel hue farthest from all assigned hues; existing assignments survive reloads and restarts. Unresolved projects use a neutral mark until an ID is available. Removing the plugin registration deletes its settings. With many projects, colors become less distinct.
- A star on any member puts the whole cluster above project groups, without duplicating or detaching children. The cluster returns to its root conversation's project when no member is starred. Group order follows the existing column order.

- Star a card using the button at the right of its project row to put it first in either column. Stars follow conversations between columns and new turns, are shared across connected clients, and survive plugin or daemon restarts. The 50-finished-card retention still applies.
- Each card represents one Paseo agent conversation. New turns update the same card and move it between columns.
- Finished cards you opened since their last turn fade: the card loses its project tint and its content dims, while Star and Remove keep full contrast. Hover or keyboard focus restores the content. The Just finished header counts the cards you have not opened as **N new**. The read state is Paseo's own unread flag, the one behind the sidebar's finished dot, so opening the thread anywhere marks it read, and a new finished or failed turn marks it unread again. Board stores nothing for it and picks up changes on the next two-second refresh. Subagents follow their parent card.
- Click a card to open its conversation on the card’s host. Remove is a separate action and does not navigate.
- On desktop, the opened conversation scrolls to its latest prompt instead of the end of the reply, so you can reread the question first. Scrolling, typing, or clicking while it opens keeps Paseo's normal position. Native clients keep Paseo's default.
- Finished conversations also show a Remove button at the top-right of the thread (in the composer track on compact layouts). Successful removal returns to Board; failures keep the conversation open. Membership refreshes every two seconds.
- Every Remove from the app also releases processes: Remove on a Board card, and Remove and Remove & New Thread in the thread. The thread's own host closes the runtimes of all its subagents at any depth first, then the removed thread's own. Subagents are found through the `paseo.parent-agent-id` label, whether or not they are on Board. The provider process and MCP servers exit, the conversation stays, and opening it resumes it. A thread that is running, initializing, or waiting for permission keeps its runtime, and so does every ancestor of a running thread. One failed close does not stop the rest; it shows an error after the removal and never undoes it. The `board_remove` tool does not close runtimes.
- Finished conversations also show **Remove & New Thread** directly below Remove. It removes the conversation from Board, then opens the New workspace screen for its project so you can prompt a new thread. Failed removal keeps the conversation open; native clients do not offer it.
- Child conversations show **Jump To Parent** while running or finished. The orange button sits below the Remove actions (or in their place while running); it opens the direct parent without changing Board membership. Compact layouts show it in the composer track.
- The project name in Paseo's workspace header (beside the workspace title) is shown as a bold chip with the project mark. The chip uses the project's Board color once Board has assigned one, otherwise a neutral tint. Paseo hides that name when it matches the workspace title. The chip is registered through `client.addWorkspaceHeaderSubtitle`.
- A removed conversation appears again when it starts a new turn. Status and duration describe the latest turn.
- Remove hides a finished card, and its finished subagents, from Board on all connected clients. It never deletes or archives the agent or chat.
- Every agent with Paseo tools can call `mcp__paseo__board_remove`, served by the daemon MCP server. An agent can remove only its own thread or its subagent threads (default: its own); the daemon names the calling agent. A finished thread is removed at once with its finished subagents. A running thread is removed when its current turn completes; failed or cancelled turns stay visible.
- Each online lane refreshes independently every two seconds while the page is open. Archived or deleted agents disappear on the next successful refresh, including cards restored after a restart. Idle unarchived conversations keep their finished cards.
- Completion, failure, and cancellation come from actual lifecycle outcomes. Idle never implies success.
- Missing terminal events show Outcome unknown. A later terminal event can resolve that state.
- Keeps the latest 50 finished conversations and running metadata in `$PASEO_HOME/plugin-data/board/runs.json` (default `~/.paseo`) with mode `0600`. Stars and removed-card state survive plugin reload, disable, and daemon restart. When upgrading an in-memory Board, visible cards must be saved before the first reload; the old snapshot cannot recover cards already removed.
- On the first Board read after startup, saved running cards are checked against the host. Cards still running stay active; unarchived cards that stopped move to Just finished with Outcome unknown unless a terminal event confirms their result. Archived or deleted cards are hidden. Other runs already active when Board opens are recovered from the host.
- Start times use the daemon timestamp when available. Hook-only times and end times are observed locally.
- A missing start time has no duration. Lifecycle delivery is best-effort; this is not an audit log.
- Project names use the host placement when available, otherwise the working directory name.
- Data stays on each owning host and the connected client. The runs file contains card metadata such as title and working directory, but no credentials, prompts, or transcripts; the Recaps log keeps only each reply's `## Recap` block.

## Recaps

- Use **Runs / Recaps** in the Board header to switch views. Both views keep every host visible in its own lane. Stars, removals, parent clusters, project colors, recaps, and project navigation stay scoped to the owning host, even if IDs or project names match across hosts.
- When a turn completes, Board reads the `## Recap` section at the end of the last assistant reply: the lines up to the next `## ` heading or the end. It records `Branch:`, `Did:`, and `Commit/push:` (with or without a leading `- `) plus the raw block, cut to 2,000 characters. Failed or cancelled turns and replies without `## Recap` add nothing.
- Entries are appended to `$PASEO_HOME/plugin-data/board/recaps.jsonl` (default `~/.paseo`) with mode `0600`. A repeated delivery of the same turn is skipped; a later turn that reuses a turn id after a session reload is kept. The log keeps at most 90 days and 2,000 entries. Logging starts when the plugin is installed; older threads are not backfilled.
- Recaps shows the last 7 days, newest first. Each day lists projects, named like run cards (host placement, else the working directory name) with the project color when one is assigned, then one row per recap: thread title, did, branch, and commit/push. Click a row to open its thread.
- **Copy** on a day puts that day on the clipboard as markdown: `## <day>`, `### <project>`, and one bullet per recap.
- Days use the host's local time zone. The RPC `board.recaps` accepts `{ days }` from 1 to 30 (default 7).

## Sidebar

On macOS desktop, Cmd+D opens Board, including from the chat composer. The listener is removed when Board stops.
Cmd+D and Back to Board after send use the Board sidebar action to retain the conversation's host when multiple hosts are connected.
Clear the conflicting Start/stop dictation shortcut in Paseo Settings → Shortcuts; this was done on the current client.
The microphone button remains available. Browser and mobile clients use the Board menu.

Board uses the native sidebar contribution. With the default navigation and current plugin set, it follows Schedules.
Paseo owns ordering; customized clients can move Board below Schedules in Settings → Appearance → Sidebar navigation.
The plugin does not overwrite user preferences or modify Paseo source. The Remove, Remove & New Thread, and Jump To Parent buttons are composer pills registered with `placement: "corner"`, in that stacking order. Remove & New Thread and the project action open the new workspace screen through `client.openNewWorkspace`. Cmd+D opens the Board on the active host; the first loaded installation owns the shortcut. Opening a run passes `anchor: "latest-prompt"` to `navigation.openAgent`.

## Checks and install

```sh
npm ci
npm run format
npm run typecheck
npm run lint
npm test
paseo plugin install "$PWD" --id board
paseo plugin reload board
paseo plugin ls board --json
paseo plugin logs board
```

Lifecycle listeners are removed on disable, and in-flight list results are discarded after stop.
The UI supports host themes and stacks columns on compact clients; runtime acceptance must record tested clients separately.
