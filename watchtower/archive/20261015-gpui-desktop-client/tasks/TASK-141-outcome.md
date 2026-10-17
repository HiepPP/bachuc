# TASK-141 Outcome

## Status

DONE, with one risk: the CPU margin is thin and grows as a long mock stream grows (see Anti-goal).

## Changed

- `packages/desktop-gpui/crates/app/Cargo.toml`: new crate `bachuc-app`, binary `bachuc-desktop`. Deps: the six view crates, `gpui-kit =0.7.3`, `log`, and on macOS `objc2`, `objc2-app-kit` (`NSApplication`, `NSDockTile`, `NSResponder`), `objc2-foundation`.
- `packages/desktop-gpui/crates/app/src/main.rs`: startup (`gpui_kit::init`, then `bachuc_theme::init`), key bindings, the window, and a stderr logger.
- `packages/desktop-gpui/crates/app/src/shell.rs`: `Shell`, the root view. It owns the host list, the `Entity<Client>`, tabs, the new-agent dialog, notifications, and the dock badge.
- `packages/desktop-gpui/crates/app/src/chrome.rs`: the `Sidebar`, `TabBar`, and `StatusLine` views.
- `packages/desktop-gpui/crates/app/src/new_agent.rs`: the new-agent dialog (provider, model, folder, prompt).
- `packages/desktop-gpui/crates/app/src/state.rs`: pure state with the unit tests (tabs, host switch, sidebar groups, badge, notification choice, shortcut table, window size).
- `packages/desktop-gpui/crates/app/src/actions.rs`, `dock.rs`: GPUI actions and key bindings, and the `NSDockTile` badge.
- `packages/desktop-gpui/Cargo.lock`: updated by cargo (the new crate and the `objc2` entries).

## Contract

- `packages/desktop-gpui/bundle.sh bin bachuc-desktop "Bachuc GPUI" io.github.hieppp.bachuc-desktop` builds `packages/desktop-gpui/target/bundles/Bachuc GPUI.app` (executable `bachuc-desktop`).
- One window, one host at a time. It opens on the host named `live` in `~/Library/Application Support/BachucGpui/hosts.json`, or on the host named by `BACHUC_HOST`. `BACHUC_WINDOW=WxH` sets the size (default 1100x760).
- Sidebar: host switcher, "New agent", "New terminal", and agents grouped by workspace name (workspace order, then "Other"). A row shows a status dot, the title, and a permission count. `closed` agents show; archived agents are not in the client list.
- Tabs: an agent tab shows `TimelineView` and `Composer`. A terminal tab shows `TerminalView`. Cmd+N (new agent), Cmd+T (new terminal in the active agent's workspace folder, else the first workspace), Cmd+W (close tab; a terminal tab calls `TerminalView::close`), Cmd+1 to Cmd+9 (nth tab).
- Host switch closes every tab (terminals are killed), drops the old client after 2 s, and connects the new host.
- Status line text: connection, host, agent count, waiting permissions. A refused connection shows `Disconnected: <reason>, retrying`.
- Notifications on `PermissionRequested` and `AgentFinished` only while the window is not active. The dock badge is the sum of `pending_permissions` over all agents.
- Accessibility labels (`getAXState`): "Sidebar", "Host switcher: <host>" (button), "New agent", "New terminal", "Agents" (list), "Workspace <name>" (heading), "Agent <title>, <status>[, N permission waiting]" (list item), "Tabs", "Tab <title>", "Close <title>", "Agent pane", "Terminal pane", "Dialog New agent", and the status line text. The composer keeps its own labels.

## Verified

- `packages/desktop-gpui/cargo.sh build --release -p bachuc-app` -> exit 0.
- `packages/desktop-gpui/cargo.sh test -p bachuc-app` -> 21 passed (tabs, host switch, sidebar groups, badge, connection text, notification choice, shortcut table, provider and model choice, window size, status text).
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-app` -> exit 0. `cargo.sh clippy -p bachuc-app --all-targets` -> no warning in `bachuc-app`.
- Bundle built with `bundle.sh bin bachuc-desktop "Bachuc GPUI" io.github.hieppp.bachuc-desktop`. `getAXState` lists the sidebar rows, the tabs, the host switcher, and the composer (`text entry area Message`, `Attach images`, `Mode`, `Model`, `Stop`, `Send`) with labels. PASS.
- Cmd+N on live, provider Mock, model Thirty minute stream, folder `/tmp/gpui-bench`, prompt "start bench", Create: a new tab opened and streamed (`Step 1, Explore, ...`), and `npm run cli -- ls -a -g` listed the agent (`ecae428`, then `9b897b2` on the final binary). PASS.
- Closed port: `hosts.json` set to `ws://127.0.0.1:6790/ws`. The app showed `Disconnected: connect error: Connection refused (os error 61), retrying`, the sidebar showed "Waiting for the host.", and the process stayed up. A listener on that port that accepts and closes saw connects at +1.2, 1.7, 2.7, 4.7, 8.7, 16.7, 26.7, and 36.7 s: backoff gaps 0.5, 1, 2, 4, 8, 10, 10 s. PASS. `hosts.json` was restored to the seeded content (135 bytes, release 6767 and live 6768).
- Host switch with two tabs open (an agent tab and a terminal tab): the status line and sidebar showed `release` with 187 agents (screenshot, `Host: release`, "Connected | release | 187 agents"), no tab was left, and the live terminal was gone from `terminal ls --all`. Then switched back to live. Only the daemon `hello` and directory requests went to release (read-only; no prompt, create, stop, or archive). `lsof` showed the app on both 6768 and 6767 during the switch. PASS.
- Permission and badge: live claude agents `536d1c2` and `90737b3` in `default` mode asked to approve `Write`, with the app in the background (Finder in front). Log lines: `dock badge set to Some("1"), reads back Some("1")`, then `Some("2")`; the sidebar row showed a yellow dot and a count of 1; the status line showed "2 permission waiting". After `npm run cli -- permit allow <id> --all` for both: `Some("1")`, then `None, reads back None`. PASS for the badge.
- Notification: macOS refused it. Log lines: `system notification authorization failed: The operation couldn't be completed. (UNErrorDomain error 1.)` and `failed to deliver system notification: ... (UNErrorDomain error 1.)`. Recorded as a gap, per the spec. The app is not signed.
- UNVERIFIED: "Zero agents" (the sidebar text "No agents yet. Press Cmd+N to start one." is code only; live always has agents) and "wrong password" (the live daemon has no password, so no rejection came back; the status line shows any `Disconnected { reason }`, which the connection-refused check exercises). The new-agent dialog error path and the "Browse" picker were not run.
- UNVERIFIED: Cmd+W and the tab "x" button on a live tab (only the host switch closed tabs), and Cmd+2 to Cmd+9. Cmd+T and Cmd+1 ran. The unit tests cover the tab model.
- UNVERIFIED (unattended): a click on a system notification (it opens the agent) cannot run, because macOS refuses notifications for this bundle.
- Verify Extras: `cargo.sh fmt --check -p bachuc-app` -> exit 0. No JS or TS changed, so no lint run. `npm run format:check:files -- watchtower/tasks/TASK-141-outcome.md` -> exit 0.
- Cleanup: the app and the `open` keep-alive loops stopped; every terminal killed (`terminal ls --all` empty); the 11 agents I made (9 mock, 2 claude) stopped and archived; the listener script removed; `hosts.json` restored.
- GitNexus impact: not run, because every file is new.

## Anti-goal

Limit: at most 10.0% CPU and 250 MB `phys_footprint`, WebKit helpers included, with one mock stream tab and one terminal tab open (stream tab shown), in the foreground for 60 seconds. Tool: `packages/desktop-gpui/measure.sh 60 app=pids:<app> wk=pids:<3 WebKit pids matched by start time>`. Times are UTC, 2026-10-15.

Early checkpoint reads (monolithic sidebar, before the fixes; the first window opened by hand with `cua_repl` raise and title-bar clicks):

- 15:45:30: app 10.8%, footprint 156 + 59 = 215 MB. Tripwire crossed.
- 15:46:59 (re-read): app 11.9%, footprint 163 + 59 = 222 MB. Crossed again.
- 15:50:01: app 12.3%, footprint 161 + 58 = 219 MB (agent at step 26).
- Diagnosis (`sample` and `ps -M`): the main thread draws the window 13 times a second while the stream runs, 5 to 8 ms each. Draw time split: timeline about 49%, composer 10%, sidebar 10%, tab bar and status 3%, layout of the rest 27%. Window size and an accessibility client did not change the CPU. The standalone timeline example read 7.4% (fresh agent) and 8.6% (agent about 22 minutes old) under the same conditions, so the shell adds about 3 points.
- Fixes: lean sidebar rows (3 elements, one line); the sidebar buttons and the tab close button are one element each instead of `Button`; the new-agent dialog returns before it copies the provider list; headings are single lines.

Final reads, final binary, UI-driven set-up (Cmd+N mock agent, Cmd+T terminal, Cmd+1), the window kept in front by `open` every 2.5 s (no `cua_repl` call during the read):

- 17:35:29 to 17:36:29, with a `BACHUC_OPEN` test hook (removed afterwards), agent about 1.5 minutes old: app 8.3%, footprint 141 MB. PASS.
- 17:52:19 to 17:53:19, final binary, agent about 40 s old, `wk=pids:92416,92417,92418`: app 10.1% (WebKit 0.0%), footprint 149 + 55 = 204 MB. Over by 0.1.
- 17:53:28 to 17:54:28 (the one re-read, window in front the whole time): app 9.7% (WebKit 0.0%), footprint 154 + 55 = 209 MB. PASS.
- Result: PASS on the re-read, with no margin.
- Not valid: 17:56:38, 0.9%, the keep-alive loop had ended and the window was covered.
- Beyond the protocol, 18:06:43 to 18:07:43, the same agent 15 minutes old (a mock turn of about 60 steps): app 13.4%, footprint 163 + 55 = 218 MB. The timeline cost grows with the turn (TASK-138 read 9.5% at step 56 for the timeline alone), so a long stream passes the 10% limit.
- Lead read, 2026-10-16: the spec's 60-second read on a young stream passes (9.7%), so the row is `DONE`. The 13.4% read on a 15-minute turn shows the limit does not hold for long turns. Q-065 asks the owner whether to fix the timeline cost or raise the budget; the Plan Verify read stays open on it.
- Lead re-check: `cargo.sh build --release -p bachuc-app` exit 0, `cargo.sh test -p bachuc-app` 21 passed, `fmt --check` exit 0, the bundle exists, `hosts.json` holds the seeded hosts, and live has no running agent left.

## Lessons

- `cua_repl` and GPUI, the facts that cost this run hours:
  - GPUI draws only while its window is visible. Right after `cua_repl` returns, another app (the host app) is in front, so frames stop, the AX tree stays at the last drawn frame, and `getScreenshot` can show an older frame. State changes still apply. Re-read after a second `getAXState` call (the first one can return the earlier tree), or keep the window in front with `open "<bundle>"` in a loop while you read.
  - Do not click the title bar to wake the window. Repeated title-bar clicks are double-clicks, which zoom the window, and a zoomed GPUI window then draws a clipped area.
  - An AX index from a tree read before the first paint can name the minimize button; look an element up by its label in a fresh tree.
  - The `cua_repl` kernel resets after about 30 s of one call; pass `timeout_ms` for long loops. After a reset the app list shows the long documentation again.
- A cached GPUI view (`Entity::cached(style)`) skips layout and paint until it is notified (about 2 points of CPU here), but it drops out of the accessibility tree and loses its accessibility actions. An assistive client connected on every launch on this machine (`Accessibility activated` in the log, even with no `cua_repl` call), so the chrome views stay uncached. `Window::is_a11y_active()` could gate the cache, but it would never run here.
- A `Role::ComboBox` div is exposed as a pop-up button, and `cua_repl` `click` on it did not run the click handler. The host switcher is `Role::Button` with `aria_expanded`. A list item, a button, and a tab answer `click` by index.
- Accessibility warnings in the log: `a11y: focused element (FocusId(5v1)) has no accessibility node (it has an id but no role)`. The shell root now has a role; an element in another crate still has an id and no role (the focus moves there after a terminal or composer opens). The audit TASK should find it with `a11y` debug output.
- GPUI re-renders every uncached view on every frame, and a view with a fixed size and `overflow_hidden` inside a flex column shrinks to zero height unless it has `flex_none`. A workspace heading needed `flex_none`.
- `Shell` re-renders about 13 times a second with the stream, so `Shell::render` and the chrome views must stay cheap: copy no list there. `render_new_agent` returns first when no dialog is open.
- `TimelineChanged` is ignored by the shell; the timeline view follows it. `AgentsChanged` covers status and permission changes; the badge and the sidebar follow it.
- `ClientEvent::PermissionRequested` does not fire for a permission that was already pending at start. The badge still shows it, because it sums `pending_permissions` from the directory.
- Terminals: `Cmd+T` uses the folder of a known workspace, because `create_terminal` rejects other folders. Killing the app leaves the daemon terminal running; `terminal kill <full or 8-character id>` works for kill.
- A host switch keeps the old client for 2 s so a queued terminal kill reaches the old daemon before its socket closes. `lsof` showed both sockets in that window.
- macOS refuses notifications for the unsigned bundle (`UNErrorDomain error 1`). GPUI reports this only through the `log` crate, so the app installs a small stderr logger. The log shows `dock badge set to ... reads back ...` for each change.
- For the plugin UI design: the sidebar, tab bar, and status line are separate entities that read a `WeakEntity<Shell>` and call back through `Shell` methods; a plugin pane would be another `TabContent` variant.

## Questions

- Should the shell get its own CPU budget? The 10% limit passes only for a young stream (9.7% to 10.1% at an agent age under 3 minutes) and fails for a 15-minute mock turn (13.4%), where the timeline example alone reads about 8.6% to 9.5%. Q-064 defaulted to keeping 10%; the margin for the shell is now about 0.
- The `BACHUC_OPEN` and `BACHUC_TERMINAL` test hook that helped one read is not in the code. Add a documented hook if later TASKs need hands-free measure runs.
