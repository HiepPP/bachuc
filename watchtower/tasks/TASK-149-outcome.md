# TASK-149 Outcome

## Status

DONE. Screen checks, live checks, and the anti-goal reads are deferred to the UI checker. This is the restart run; the first builder died in the out-of-memory reboot and left unverified parts (`links.rs`, `tools.rs`, `cards.rs`, `find.rs`, `tests/links.rs`, a `diff.rs` helper). This run checked them against the Electron sources, kept them, and fixed the question form pick order (`cards.rs` now keeps picks in click order, as the app's `Set`).

## Changed

- `packages/desktop-gpui/crates/timeline/Cargo.toml`: adds `bachuc-icons` and `bachuc-explorer` (for `syntax::{highlight, language_for, SyntaxColors}`).
- `src/lib.rs`: `TimelineEvent`, content column (max 820, centered), Electron message rows on `bachuc-theme` tokens and metrics, turn header and step rows restyled, step pick (strip segment or step header), cwd tracking, image, form, find, and lightbox state. The hard-coded `0x256ABF` is gone; the running step uses `dot_running`.
- `src/tool_view.rs` (new): `ExpandableBadge` tool rows and `ToolCallDetailsContent` sections per kind, syntax colors, and the edit-diffs edit row (toggle button, change bar, open-file button, numbered diff).
- `src/card_view.rs` (new): permission card, plan card, question form; all answers go through `respond_permission_with`.
- `src/image_view.rs` (new): assistant images through `read_file`, retries 100/400/1200 ms, failure state with a Retry button, lightbox.
- `src/find_bar.rs` (new): the find bar, match highlights through `TextViewState::set_range_highlights`, stepping with `reveal_range`.
- `src/markdown.rs`: a line that holds only `![alt](local path)` is an image segment.
- `src/diff.rs`: `detail_lines` (first builder) and `numbers` (line numbers as edit-diffs `numberRows`).
- `src/links.rs`, `src/tools.rs`, `src/cards.rs`, `src/find.rs` (first builder, checked).
- `tests/links.rs` (first builder), `tests/tools.rs`, `tests/cards.rs` (new).
- `examples/timeline.rs`: registers `bachuc_icons::Assets`, binds Cmd+F to `TimelineView::find`, prints each `TimelineEvent` to stderr.
- `packages/desktop-gpui/Cargo.lock`: updated by cargo.

## Contract

- Produces `bachuc_timeline::TimelineEvent::OpenFile { cwd: String, path: String, line: Option<u32> }`, emitted through `EventEmitter<TimelineEvent>`. Sources: a file link in assistant text (`links::resolve_click`; `path` resolved absolute under the agent cwd, `line` = start line), the open-file button of a read, edit, or write badge, and the open-file button of an edit row (`path` = the tool's `filePath`, `line` None).
- Produces `TimelineView::find(&mut self, window: &mut Window, cx: &mut Context<TimelineView>)`: opens the bar, or focuses it and selects its text.
- `TimelineView::new` is unchanged. `TimelineView` now also implements `Focusable`.
- Consumes `bachuc_theme::tokens`, `bachuc_theme::metrics`, `bachuc_icons::{IconName, icon}`, `Client::read_file` with `FileKind::Image`, and `Client::respond_permission_with`, with the names in the spec.
- Payloads (same as `agent-stream/view.tsx` line 1594 and `question-form-card.tsx`): action `{"behavior":"allow","selectedActionId":id}` or `{"behavior":"deny","selectedActionId":id,"message":"Denied by user"}`; plan reject is the `reject` action; question submit `{"behavior":"allow","updatedInput":{...input,"answers":{header:answer}}}`; dismiss `{"behavior":"deny","message":"Dismissed by user"}`, or a submit of empty answers when every question is optional free text.
- ADR-0039: per-flush work stays in proportion to the change. New per-sync work is the cwd compare, `retain` on the pending-permission maps, code-span linkify of changed assistant messages only, and image loads for new image segments. Syntax runs are made once per open section or edit (cached by text, and per edit in `Diff::runs`). Find searches only on a query change or a step after the timeline changed, never on a flush.
- Safe defaults taken (see questions): a picked step lists its non-edit tool calls as badges; user message images are not shown; a failed image shows a Retry button.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-timeline` -> PASS: 54 tests (10 unit, 8 cards, 16 grouping including `incremental_rows_match_full_regroup`, 11 links, 9 tools). File-link parse tests cover every `describe` of `assistant-file-links/parse.test.ts` except `normalizeInlinePathTarget` (no port: the app's file pane places the path). One test per tool detail kind: shell, read, edit, write, search, fetch, sub_agent, plan, plus unknown/thinking.
- `packages/desktop-gpui/cargo.sh clippy -p bachuc-timeline --all-targets` -> PASS: no timeline warning.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-timeline` -> PASS.
- `packages/desktop-gpui/cargo.sh build -p bachuc-app` -> PASS (the app still builds against the unchanged `TimelineView::new`).
- GitNexus `impact` upstream on `TimelineView` (struct) -> LOW, 12 impacted; only `crates/app/src/shell.rs` constructs it. Index is 16 commits behind.
- Bundle the example and compare with the reference app on 6770 (turn, each tool kind, plan card, question card) -> DEFERRED (UI checker).
- Click a file path in a reply; the example prints `OpenFile` with path and line -> DEFERRED (UI checker).
- Live: answer a question form and reject a plan on a claude agent in plan mode; `npm run cli -- logs <id>` -> DEFERRED (UI checker).
- A reply with a PNG from `/tmp/gpui-bench` renders and opens larger on click; a missing file shows the retry state -> DEFERRED (UI checker).
- Cmd+F, type a word, Enter twice -> DEFERRED (UI checker).
- `cua_repl` `getAXState`: diff toggle is its own button with a label; tool rows and cards have roles and labels -> DEFERRED (UI checker).

## Anti-goal

Gauge: the timeline example at most 10.0% CPU and 250 MB footprint, WebKit included, on a mock stream at about 2 and about 15 minutes.

- Checkpoint "before the restyle": NOT TAKEN. The first builder died before any read, and builders run no screen checks. The UI checker can read the committed baseline `0f1ed905f` build for it.
- Checkpoint "after the tool views land": DEFERRED (UI checker). `measure.sh 60 tl="pids:<pid>" wk="pids:<WebKit pids>"`.
- Checkpoint "completion": DEFERRED (UI checker). Same read at about 2 and about 15 minutes.

## Parity list

Not run. DEFERRED (UI checker). Values the code sets, from the cited Electron files:

| Element           | Layout and style set in code                                                                                                              | State      |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Content column    | max 820, centered, list padding 16 sides, column padding 8                                                                                | unverified |
| User message      | right aligned, `surface3`, radius 16 with top right 2, padding 16, `FONT_CONTENT`, line height 1.4, 16 above and below                    | unverified |
| Assistant message | padding 12 above and below, `FONT_CONTENT` markdown, code spans that look like files are links                                            | unverified |
| Turn header       | title 13.5 semibold, clock mono 12.5 muted, strip 170 x 8 gap 2 radius 3, summary 13 muted right; picked step dims others to 0.3          | unverified |
| Step row          | number 20 round, 1 px label border (running 1.5 `dot_running`), label mono 10 semibold upper, counts 12.5, time mono 11.5 w40, body 34/52 | unverified |
| Step rail         | 1 px `border` at x 10 from the first number to the last                                                                                   | unverified |
| Picked step       | `surface2` behind the row, radius 4                                                                                                       | unverified |
| Live box          | margin top 6, 1 px border, radius 8, padding 10 / 7, mono 12 line 19                                                                      | unverified |
| Edit row          | chevron 14, path mono 12.5, 7 squares of 7 px, +N / −M 13 `status_success` / `status_danger`, diff radius 8 `surface1` max 480            | unverified |
| Tool badge        | margin -13, padding 8 / 4, radius 8, icon slot 22 with 12 px icon, label and summary `FONT_BASE` muted; open `surface1` + border          | unverified |
| Tool details      | shell/edit/write full bleed `surface1`; others boxed `surface2` radius 4; padding 12; mono 12 line 18; max 400 (200 in a permission card) | unverified |
| Diff lines        | add `status_success` at 0.15, remove `status_danger` at 0.1, context and hunk `surface1` muted                                            | unverified |
| Plan card         | padding 12, radius 8, `surface1`, border, chevron 16, title `FONT_BASE` line 22                                                           | unverified |
| Permission card   | padding 12, radius 8, `surface1`; "How would you like to proceed?"; buttons padding 8 / 12, radius 6, `border_accent`, Check or X 14      | unverified |
| Question form     | padding 12 gap 12; tabs 28 high; options padding 12 / 8, 18 px control (`accent`); text field radius 8 `surface2`; Submit `accent`        | unverified |
| Find bar          | top 8 right 12, 340 wide, padding 6, `surface1`, border, radius 8, shadow md; field 28 `surface2`; status `FONT_SM` muted                 | unverified |
| Image             | at most 480 x 360; loading and failed frames 160 high                                                                                     | unverified |

Known mismatches, set on purpose:

- Running blue: Electron draws `#256ABF`; the spec removes it, so GPUI uses `dot_running` (`#268ae0` light).
- Diff tints: Electron uses `rgba(46,160,67,.15)` and `rgba(248,81,73,.1)`; GPUI uses the status tokens at the same alphas.
- Notification tints: Electron uses literal rgba; GPUI uses `tint_danger`, `tint_warning`, and `surface2`.
- No pulse on the running segment and number, and no shimmer on a running badge (CPU).
- A picked step also lists its non-edit tool calls as badges; Electron only highlights it.
- A failed image shows a Retry button under the message; Electron shows only the message after its automatic retries.
- Paseo tool detail sections (`buildPaseoToolDetailSections`) are not ported; Paseo calls show Input and Output.
- Todo list and compaction marker are close approximations of `TodoListCard` and `CompactionMarker`.

## Lessons

- In the GPUI AX tree a `Role::ListItem` with a label swallows its children's labels; `Role::Group` with a label keeps children as their own nodes (the old diff toggle gap). Step rows are now a plain container with a `Role::Button` header.
- `TextView::on_link_click` takes a `Send + Sync` closure; a `WeakEntity` of the view works inside it.
- `TextViewState::rendered_text`, `set_range_highlights`, and `reveal_range` give find highlights and scrolling inside markdown without layout changes.
- The daemon's `user_message` timeline item has no images (`packages/protocol/src/agent-types.ts` line 372). Electron shows user images only from the sending client's local attachment store, so the timeline cannot show them; TASK-155 needs a local store or an optimistic entry from the composer for that.
- gpui `div().role(..)` needs an `.id(..)` first; `flex_shrink` takes a factor; there is no `aria_disabled` or `aria_checked` (use `aria_toggled(Toggled::..)`).
- TASK-155: subscribe to `TimelineEvent` on the timeline entity, bind Cmd+F to `TimelineView::find`, and register `bachuc_icons::Assets` or no timeline icon draws.
