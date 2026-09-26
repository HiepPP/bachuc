# Board

Paseo conversation board. The Board sidebar item opens Running and Just finished columns.
Supports Paseo daemon and client 0.8.x and 0.9.0-beta.2. Uses standard plugin surfaces, sidebar registration, RPCs, and lifecycle hooks.

## Behavior

- Use **UI size − / +** in the Board header to scale Board text, cards, and spacing from 10% to 200% in 10% steps. 100% is the compact default (the former 70%). Click the percentage to reset to 100%. The control sits in a fixed toolbar outside the scrolling, scaled content, keeping its size and click position unchanged; Paseo navigation and other pages are unaffected. The size is saved in host settings, so it survives plugin reloads and app restarts and is shared by clients of the same host.

- Parent and subagent conversations form nested clusters with a **subagents** toggle. Clusters start expanded; collapse state lasts while Board stays mounted, including refreshes and column changes. Compact layouts use a smaller indent. Collapsed groups summarize how many members are running or need input; subagent cards omit the project row when it matches the parent.
- Relationships come from Paseo parent metadata, never titles or project names. Missing or removed parents leave their children visible as standalone cards. Provider-internal subagents without a separate Paseo conversation are not listed.
- A cluster stays in Running while any member runs; otherwise it moves to Just finished, ordered by its latest completion. Counts include all conversations, even collapsed descendants. Each card keeps its own status, star and open actions. Remove appears once a card and all of its subagents have finished, labelled **Remove all · N** on parents; removing a card also removes its finished subagents on all connected clients. Removing from the thread view while a subagent is still running keeps that subagent and its own subtree visible as standalone cards.
- Cards are flat, single-border surfaces. Project groups are tinted containers headed by a project mark (rounded square with the project's initial, as in the sidebar), the name, and a count when more than one conversation is inside. Running cards show a small spinner beside the status; needs-input cards show a warning badge and failed cards a red status label. Subagents render as compact two-line cards with local bot avatars on an inset panel inside their parent card. The panel uses two columns when its measured width permits, otherwise one; nested groups and UI scaling follow the same rule. Avatars are stable per conversation ID (20 shared designs, cropped from one generated 5 × 4 grid; no remote image requests). Child metadata combines provider, status, and duration; differing projects retain a project line. Needs-input children have an amber border. Star and finished Remove actions appear on hover or keyboard focus on web and stay visible on touch and native clients; icon-only Remove retains its full accessible cascade label.
- Unstarred clusters are grouped by host project identity within each column, with a project name and count. Same-name projects stay separate; when placement is unavailable, the full working directory identifies the group. Empty groups disappear.
- Project block colors are saved in host settings by Paseo project ID. New IDs receive the pastel hue farthest from all assigned hues; existing assignments survive reloads and restarts. Unresolved projects use a neutral border until an ID is available. Removing the plugin registration deletes its settings. With many projects, colors become less distinct.
- A star on any member puts the whole cluster above project groups, without duplicating or detaching children. The cluster returns to its root conversation's project when no member is starred. Group order follows the existing column order.

- Star a card using its top-right button to put it first in either column. Stars follow conversations between columns and new turns, are shared across connected clients, and survive plugin or daemon restarts. The 50-finished-card retention still applies.
- Each card represents one Paseo agent conversation. New turns update the same card and move it between columns.
- Click a card to open its conversation on the selected host. Remove is a separate action and does not navigate.
- On desktop, the opened conversation scrolls to its latest prompt instead of the end of the reply, so you can reread the question first. Scrolling, typing, or clicking while it opens keeps Paseo's normal position. Native clients keep Paseo's default.
- Finished conversations also show a Remove button at the top-right of the thread on desktop (beside the composer on native clients). Successful removal returns to Board; failures keep the conversation open. Membership refreshes every two seconds.
- Finished conversations also show **Remove & New Thread** directly below Remove on desktop. It removes the conversation from Board, then opens the New workspace screen for its project so you can prompt a new thread. Failed removal keeps the conversation open; native clients do not offer it.
- Child conversations show **Jump To Parent** while running or finished. On desktop, the orange button sits below the Remove actions (or in their place while running); it opens the direct parent without changing Board membership. Native clients show the action beside the composer.
- On desktop, the project name in Paseo's workspace header (beside the workspace title) is shown as a bold chip with the project mark. The chip uses the project's Board color once Board has assigned one, otherwise a neutral tint. Paseo hides that name when it matches the workspace title, and project names shared by several projects stay neutral. Colors and names refresh every 10 seconds. The chip is a scoped style over Paseo's `workspace-header-subtitle` element; if Paseo renames that element, the header keeps its default look. Native clients keep Paseo's header.
- A removed conversation appears again when it starts a new turn. Status and duration describe the latest turn.
- Remove hides a finished card, and its finished subagents, from Board on all connected clients. It never deletes or archives the agent or chat.
- Running cards refresh from the host every two seconds while the page is open.
- Completion, failure, and cancellation come from actual lifecycle outcomes. Idle never implies success.
- Missing terminal events show Outcome unknown. A later terminal event can resolve that state.
- Keeps the latest 50 finished conversations and running metadata in `$PASEO_HOME/plugin-data/board/runs.json` (default `~/.paseo`) with mode `0600`. Stars and removed-card state survive plugin reload, disable, and daemon restart. When upgrading an in-memory Board, visible cards must be saved before the first reload; the old snapshot cannot recover cards already removed.
- On the first Board read after startup, saved running cards are checked against the host. Cards still running stay active; missing cards move to Just finished with Outcome unknown unless a terminal event confirms their result. Other runs already active when Board opens are recovered from the host.
- Start times use the daemon timestamp when available. Hook-only times and end times are observed locally.
- A missing start time has no duration. Lifecycle delivery is best-effort; this is not an audit log.
- Project names use the host placement when available, otherwise the working directory name.
- Data stays on the selected host and connected client. The runs file contains card metadata such as title and working directory, but no credentials, prompts, or transcripts; the Recaps log keeps only each reply's `## Recap` block.

## Recaps

- Use **Runs / Recaps** in the Board header to switch views. Runs is the board above and works as before.
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
The plugin does not overwrite user preferences or modify Paseo source. The desktop Remove placement uses a scoped DOM adapter; the action remains agent-scoped and owned by Paseo.

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
