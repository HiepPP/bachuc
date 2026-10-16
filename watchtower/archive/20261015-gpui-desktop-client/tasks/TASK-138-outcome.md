# TASK-138 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/timeline/Cargo.toml`: new crate `bachuc-timeline`. Deps: `bachuc-client`, `bachuc-diagram`, `bachuc-theme`, `gpui-kit =0.7.3` with the features `tree-sitter-bash`, `-javascript`, `-python`, `-rust`, `-tsx`, and `-typescript` (JSON comes with the base `tree-sitter` feature), `chrono`, `serde_json`, and `similar`.
- `packages/desktop-gpui/crates/timeline/src/grouping.rs`: the Rust port of `prepareTurnHistory`, `groupLiveTurns`, `buildTurnGroup`, the `buildOverviewGroup` counts, and `collectTurnNoteHosts`, over projected entries.
- `packages/desktop-gpui/crates/timeline/src/markdown.rs`: splits message markdown into text, mermaid, and display math segments. A failed diagram falls back to a fenced code block.
- `packages/desktop-gpui/crates/timeline/src/diff.rs`: unified diff parsing, a line diff for edits without a diff, and write content.
- `packages/desktop-gpui/crates/timeline/src/lib.rs`: `TimelineView`, the virtualized list, rows, steps, diffs, diagrams, and permissions.
- `packages/desktop-gpui/crates/timeline/tests/grouping.rs`: the 15 grouping cases ported from `model.test.ts`.
- `packages/desktop-gpui/crates/timeline/examples/timeline.rs`: the example. It connects to a host, finds the agent by ID or prefix, and shows its timeline.
- `packages/desktop-gpui/Cargo.lock`: updated by cargo, and shared with groups E and F.

## Contract

- Produces `bachuc_timeline::TimelineView` (a GPUI view) with `TimelineView::new(client: gpui_kit::Entity<bachuc_client::Client>, agent_id: String, diagrams: gpui_kit::Entity<bachuc_diagram::DiagramRenderer>, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<TimelineView>) -> TimelineView`. Create it with `cx.new(|cx| TimelineView::new(...))`.
- `new` calls `Client::view_agent`. Releasing the view calls `Client::unview_agent`. A reconnect (`Connected`) views the agent again.
- Extras: `TimelineView::agent_id()`, plus the public modules `grouping` (`group_turns`, `build_turn_group`, `turn_note_hosts`, `Row`, `TurnGroup`, `TurnStep`, `TurnRun`, `StepCounts`), `markdown` (`segments`, `blocks`, `Segment`, `Block`), and `diff`.
- Consumes, unchanged from TASK-136: `Client`, `ClientEvent`, `view_agent`, `unview_agent`, `timeline`, `load_older`, `respond_permission`, `TimelineState`, and `agents()` (agent status `running` tells whether the turn is active). From TASK-137: `DiagramRenderer::render`, `DiagramState`, `DiagramReady`, `image_size`, and `DiagramKind::label`. From TASK-134: `scheme`, `TurnLabel`, and `turn_label_colors`.
- Example: `timeline <agent id or prefix> [ws url]`. The URL defaults to `ws://127.0.0.1:6768/ws`. `BACHUC_TIMELINE_TRACE=1` prints each client event with the timeline version to stderr.

## Verified

- `packages/desktop-gpui/cargo.sh test -p bachuc-timeline` -> 9 unit tests and 15 grouping tests pass. The grouping tests cover every `model.test.ts` case, except two parts. The edit-diffs plugin case ("joins plugin edit rows through the stream presentation") is skipped, as the spec says. The compact-screen half of "regroups history" is not ported, because GPUI has no compact turn mode. Plugin edit rows became plain edit calls. PASS.
- `packages/desktop-gpui/cargo.sh test -p bachuc-timeline broken_mermaid` -> 2 passed. `broken_mermaid_renders_as_a_code_block_when_the_renderer_fails` asserts a fenced mermaid code block and the error text. PASS.
- `packages/desktop-gpui/cargo.sh test -p bachuc-timeline math_block` -> 2 passed. `math_block_becomes_a_math_request` asserts the requests `(DiagramKind::Math, source)`. PASS.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-timeline` -> exit 0. `cargo.sh clippy -p bachuc-timeline --all-targets` -> no warnings in the crate (2 old warnings in `crates/protocol/build.rs`). PASS.
- Agent check on the mock stream: `bundle.sh example timeline BachucTimeline io.github.hieppp.bachuc-timeline`, opened with mock agent `bdf1653`, then `7df092ba` and `5ccd3ad6`. `getAXState` lists "Heading Turn, working, N steps", "Step 1, Explore, read 1 file, searched 1 time", "Step 2, Edit, edited 1 file, ran 1 command", ... "Step N, Verify", with "Diff of hooks/use-scroll-anchor.ts, 5 added, 2 removed" and "Hide diff of ..." buttons. Screenshots show the label colors, the numbered steps, and the red and green diff lines. PASS.
- Agent check with live claude agent `f6bb0fc` in `default` mode: its answer showed Rust code highlighted by tree-sitter, a mermaid flowchart image, the broken mermaid block as a code block with "Parse error on line 2:", and the Gaussian integral as a formula image. `getAXState` listed "Assistant message: # Render Test", "Mermaid diagram", "Diagram source, failed to render", and "Math formula". A finished turn showed "Worked for 17s", the answer below the steps, and the rule. PASS.
- Scroll check on mock `7df092ba`: after a scroll up, the visible rows stayed "Step 9" and "Step 10" for 8 s while the stream reached step 14. After a scroll back down, the view followed the tail from "Step 16" to "Step 17". PASS.
- Long history on live: closed agent `bdf1653` had 200 projected entries and `has_older`. Before the fix, a scroll to the top loaded the older page and jumped to the first row. Fix: the anchor search falls back to the first row that reaches the old top entry. After the fix, the trace showed `removed=0..2 inserted=43`. The view stayed near the old first step, which became step 41 of 97, and later scrolls reached "User message: start bench". PASS.
- Permission check with live claude agent `f6bb0fc` (`default` mode, Write tool): "Permission request: Write" with "button Allow" and "button Deny" appeared. Allow created `allow.txt`, `probe2.txt`, `probe3.txt`, `probe4.txt`, and `probe6.txt`. Deny left out `deny.txt`, `probe.txt`, and `probe5.txt`, and the agent replied "refused". After each click, the request row went away when the daemon resolved it. PASS.
- Verify Extras: `cargo.sh fmt --check -p bachuc-timeline` -> exit 0. No JS or TS files changed, so lint does not apply. `npm run format:check:files -- watchtower/tasks/TASK-138-outcome.md` -> exit 0.
- Cleanup: agents `bdf1653`, `7df092ba`, `5ccd3ad6`, and `f6bb0fc` were stopped and archived. `terminal ls --all` is empty. The app bundle is quit.
- GitNexus impact: not run, because every file is new; no existing symbol changed.

## Anti-goal

Limits: at most 10.0% CPU and 250 MB phys_footprint, WebKit helpers included, on the mock stream in the foreground for 60 seconds. Reads use `packages/desktop-gpui/measure.sh 60 tl="BachucTimeline.app" wk="pids:<WebKit pids matched by start time>"`.

- Started 2026-10-15. The live daemon on 6768 was running (`npm run cli -- daemon status`).
- Checkpoint 1, before the turn grouping lands (flat rows, plain markdown, renderer created), 2026-10-15T10:55:19Z to 10:56:20Z: mock agent `bdf1653` (`thirty-minute-stream`), window raised every 2 s, `wk="pids:50403,50405,50408"` -> CPU 3.6% (app 3.6 + WebKit 0.0), footprint 164 MB (111 + 53). Within limits.
- Checkpoint 2, after the turn grouping landed (one list item per turn), 2026-10-15T11:05:29Z to 11:06:29Z: same agent, window raised every 2 s, `wk="pids:86276,86277,86280"` -> CPU 74.4% (app 74.4 + WebKit 0.1), footprint 183 MB (129 + 54). Tripwire crossed on CPU. The window was raised, so the read is valid. Root cause: the mock thread is one turn of 40+ steps, and one list item held the whole turn, so GPUI laid out every step, note, and diff on each frame.
- Fix: a turn now takes one list item for its header and one per step, so only steps on screen lay out.
- Checkpoint 2 re-read, after the fix, 2026-10-15T11:16:29Z to 11:17:29Z: same agent, window raised every 2 s, `wk="pids:41339,41340,41342"` -> CPU 3.9% (app 3.8 + WebKit 0.1), footprint 115 MB (62 + 53). Within limits.
- Checkpoint 3, after diagrams landed: the first agent had finished its 30 minutes, so a new mock agent `7df092ba` ran. First read 2026-10-15T14:37:02Z to 14:38:02Z with Raise only, `wk="pids:34535,34537,34547"` -> CPU 0.7% (0.7 + 0.0), footprint 102 MB (47 + 55). Not valid: the window still showed "Connecting..." and did not paint, because Raise alone did not make it visible.
- Checkpoint 3 re-read, 2026-10-15T14:38:38Z to 14:39:39Z, with Raise plus a title-bar click every 2.8 s; screenshots showed the stream moving -> CPU 5.4% (5.4 + 0.0), footprint 187 MB (132 + 55). Within limits.
- Checkpoint 4, at completion (final code, the paging anchor fix included), 2026-10-15T15:13:46Z to 15:14:49Z: new mock agent `5ccd3ad6` at steps 50 to 56, Raise plus a title-bar click every 2.7 s, AX showed the steps advance, `wk="pids:59217,59218,59219"` -> CPU 9.5% (9.4 + 0.1), footprint 196 MB (136 + 60). Within limits.
- Result: PASS. Checkpoint 2 crossed the tripwire once, and the fix brought it back under. Every final read is within limits. The completion read has a margin of only 0.5 points. It ran on a timeline of 56 steps, and the click every 2.7 s also costs redraws.

## Lessons

- Never put a whole turn in one GPUI `list` item. A mock turn has 50+ steps, and one item made the list lay out all of them on every frame (74% CPU). One item per step header and step keeps the cost near 4 to 9%.
- In this run, `performSecondaryAction(0, 'Raise')` alone did not always make the window visible to GPUI, so it did not paint. Several reads and AX states were stale, which looked like a stalled timeline. A title-bar click (`app.click([600,20])`) after Raise brought painting back. The `BACHUC_TIMELINE_TRACE` stderr trace showed that the view kept syncing while the window was not painted.
- `ListState` with `FollowMode::Tail` gives the follow-only-at-bottom rule. Splice only the changed key range: GPUI keeps `logical_scroll_top` for rows after the splice. An older page can regroup the top turn, so find the old top entry again (`anchor`, then the first row whose last entry reaches it) and `scroll_to` that row.
- `use gpui_kit::*` brings GPUI's own `test` attribute into scope. A `mod tests { use super::*; }` then fails with "recursion limit reached while expanding #[test]". Import names one by one in tests.
- Of the gpui-kit tree-sitter features, `css`, `html`, `toml`, and `yaml` made cargo fail to resolve `cocoa` (the `=0.26.0` pin of `gpui-fast-macos` against the locked 0.26.1). `bash`, `javascript`, `python`, `rust`, `tsx`, and `typescript` resolve.
- `StyledExt` (for `font_semibold`) is in `gpui_kit::base`, and `FluentBuilder` (for `when`) is in `gpui_kit::prelude`.
- A mock `thirty-minute-stream` ends after 30 minutes. If builds wait on the shared cargo lock for a long time, start a new mock before a read.
- Process note: I ran `osascript -e 1` once by mistake while I quit the app. It was a no-op and did no UI action. I used `caffeinate -u -t 1800` to keep the display awake during reads; the assertion ends by itself.

## Questions

- None blocking. The plan's anti-goal margin is thin on long mock timelines: 9.5% at step 56. The app shell (TASK-141) adds more views to the same window, so it may need its own budget.
