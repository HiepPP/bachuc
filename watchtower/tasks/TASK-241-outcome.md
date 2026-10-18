# TASK-241 Outcome

## Status

DONE. The break was in the GPUI client crate. The lead added the client files to this TASK (Q-147,
DEFAULTED) and the fix is applied.

## Changed

- `packages/desktop-gpui/crates/client/src/connection.rs`: `CAPABILITIES` is `[&str; 9]` and
  has `"plugin_timeline_items"`. The hello test asserts it.
- `packages/desktop-gpui/crates/client/tests/sync.rs`: the hello assertion covers it.
- `packages/desktop-gpui/crates/timeline/tests/grouping.rs`: the stale header note is fixed.

## Contract

Root cause: the GPUI `hello` frame does not advertise `plugin_timeline_items`. The daemon sends a
`type: "plugin"` timeline item only to clients that advertise it. The thread-branch card is that
kind of item, so the GPUI timeline never receives it.

- Break layer: GPUI client crate (host code), not the thread-branch server part, not the
  thread-branch renderer, not the timeline crate.
- File: `packages/desktop-gpui/crates/client/src/connection.rs:66`. `CAPABILITIES` is `[&str; 8]`
  and lacks `"plugin_timeline_items"`.
- Daemon filter: `packages/server/src/server/session.ts:1284-1290` (`supportsTimelineItem`), used
  for live streams (`:1324`, `:1335`), fetches (`:1838`, `:4811`, `:7713`, `:7976`).
- Fix (applied): add `"plugin_timeline_items"` to `CAPABILITIES` and change the array length
  from 8 to 9. Name comes from `packages/protocol/src/client-capabilities.ts`
  (`pluginTimelineItems`). Add it to the hello assertion in
  `packages/desktop-gpui/crates/client/tests/sync.rs:111`.
- Also update the stale note at the top of `packages/desktop-gpui/crates/timeline/tests/grouping.rs`
  ("the client does not ask for plugin timeline items").
- After the fix, the GPUI app must be rebuilt and reconnected. No server or plugin change is needed.

## Verified

- Live (6768), plugin state: `npm run cli -- plugin ls` -> thread-branch `running`, enabled.
- Live trace: created a temp git repo `/tmp/tb241`, ran a Claude agent that appended one line to
  `a.txt`. The server part wrote a journal entry (`a.txt` +1 -0) and appended the timeline row
  (`turn-diff:...`, kind `thread-branch-turn-diff`, `pluginId` thread-branch, seq 22).
- Wire check: a client that advertises the capability (`DaemonClient`,
  `packages/client/src/connection/index.ts:142`) fetches the row with the expected shape.
  The GPUI hello list (`connection.rs:66`) does not advertise it.
- Static check of the GPUI chain, all consistent with the wire shape: `daemon_item`,
  `renderer_for_item`, `ContributionIndex` renderer registration, `TurnCardView` parse, and
  `render_daemon_item`. None of it can run, because the item never arrives.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-client -p bachuc-plugin-thread-branch -p bachuc-timeline`
  -> exit 0, all pass. `./cargo.sh fmt --check` on the three crates -> exit 0.
- Other plugin items the daemon now sends: jev-orchestrator has a GPUI renderer. html-reply has no
  GPUI crate, so its item draws the generic note "Plugin timeline item unavailable."
  (`crates/timeline/src/plugin_view.rs:242`). No crash path. The note is noise for html-reply rows.
- UI checker lines are DEFERRED (UI checker). The running GPUI app needs a rebuild and reconnect.
- Cleanup: the test agent was archived and `/tmp/tb241s` was removed. `/tmp/tb241` stays as a
  git repo with one extra line.

## Anti-goal

Zero extra cards: DEFERRED (UI checker). The server part already returns null for a turn with no
file change and no commit (`server/turn-diff.ts:414`), so a turn with no change makes no row.

## Lessons

- A plugin timeline item reaches a client only with the `plugin_timeline_items` capability. When a
  daemon item is missing in GPUI, compare the hello capabilities first.
- A fetch from the TS client is a quick wire check: `connectToDaemon` with an endpoint target and
  `fetchAgentTimeline`.
