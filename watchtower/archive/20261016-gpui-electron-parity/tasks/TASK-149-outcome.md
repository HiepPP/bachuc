# TASK-149 Outcome

## Status

BLOCKED (lead, 2026-10-16). Blocked: both UI fix rounds are used, and a heading directly followed by a code block keeps a 25 px gap against Electron's 37 px (Q-086). The badge crash, message spacing, and code block look pass. Live checks and CPU reads are `UNVERIFIED (unattended): live down`.

Builder status before the lead's re-check: DONE. Screen checks, live checks, and the anti-goal reads are deferred to the UI checker. This is the restart run; the first builder died in the out-of-memory reboot and left unverified parts (`links.rs`, `tools.rs`, `cards.rs`, `find.rs`, `tests/links.rs`, a `diff.rs` helper). This run checked them against the Electron sources, kept them, and fixed the question form pick order (`cards.rs` now keeps picks in click order, as the app's `Set`).

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

## Fix Round 1

Status: DONE. Fixes the four timeline mismatches of the K parity table (eva agent "hi", 1200 x 800). Files: `crates/timeline/src/lib.rs`, `crates/timeline/src/grouping.rs`. `TimelineView::new`, `find`, and `TimelineEvent` are unchanged.

- Timeline anchor: the list is `ListAlignment::Top` with `FollowMode::Tail`. A short timeline starts at the top. Once it fills the pane, the list follows the newest row while it streams. gpui's own follow-tail tests use this pair.
- Headings: the message text now uses the gpui-base `TextView` with an explicit style, because the gpui-kit wrapper style sets only the heading size. `heading_style` follows `markdown-styles.ts`: sizes are tier/14 of the content size, rounded (h1 28, h2 24, h3 21, h4 19, h5 and h6 17 at content 15). Line height is 1.3 of the size. h1 and h2 are bold with padding 8, a 1 px `border` rule, and 12 below. h3 to h6 are semibold. Top padding is 24, 24, 16, 16, 12, 12. `markdown_style` rebuilds gpui-kit's default colors (`base_text_view_style`), and is cached until the theme changes.
- Code block: Electron never shows a fence language; it shows a copy button on hover. `code_block_actions` now draws a copy button, which also removes the gpui-base language label and its 28 px top padding. The button shows when the pointer is over it, not over the whole block (no group hover on a text view action).
- Turn footer: a footer row closes each finished response that has an assistant message, as `layout.ts` places them; the running response has none. It holds the copy button (all assistant text of the response, joined by a blank line, Check icon for 1.5 s) and "Worked for N" (last entry time minus user message time, `formatDuration` of `utils/time.ts`). Fork is out of scope (rewind and fork, `CONTEXT.md`) and is left out.
- ADR-0039: the footer reads only facts the grouper caches (`TurnGrouper::is_user`, `is_assistant`, `at`), one check per row in the existing row loop. A turn row checks its steps for a note only while the response has no answer yet. No entry is read or hashed on a flush. Copy text is built on click.

Known mismatches, set on purpose:

- The footer shows no end time on hover and no TTFT or tokens-per-second parts.
- A response whose user message is on an older page shows only the copy button; Electron shows its end time. The crate's chrono has no local time zone.
- The copy icons do not turn `foreground` on hover. A gpui-kit Icon has no group hover.
- h6 is not uppercase and has no letter spacing. gpui has no text transform.

Verified:

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-timeline` -> PASS: 56 tests. New unit tests are `footer_duration_matches_format_duration` and `headings_match_markdown_styles`.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-timeline` -> PASS.
- `packages/desktop-gpui/cargo.sh clippy -p bachuc-timeline --all-targets` -> PASS: no timeline warning.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> PASS.
- GitNexus `impact` upstream: `render_markdown` LOW. `sync` HIGH: 12 impacted, all inside `TimelineView` (`new`, `refresh`, subscriptions); its signature is unchanged. The index is 27 commits behind.
- Side by side with the reference (eva "hi"): user bubble near y 120, "Next Steps" about 122 px wide with a rule, no `text` label, and a footer with copy and "Worked for 7s" -> DEFERRED (UI checker).
- A long mock stream starts at the top, then follows the bottom once full -> DEFERRED (UI checker).
- CPU read at turn age about 2 and 15 minutes -> DEFERRED (UI checker).

## UI Checks

Run by the UI checker on 2026-10-16, HEAD 9fec90f18 (commits d96647b64 and eda235491 included), light theme. Bundle `BachucTimeline` rebuilt before the run. Live (6768) is down, so every line that needs a mock stream is `UNVERIFIED (unattended): live down`. The example ran on `ws://127.0.0.1:6770/ws` with three reference agents: `7d4ad155` (made for this check: haiku-5-5, `/tmp/gpui-bench`, one turn with a fixed markdown reply; archived after), `380c019` ("hi", eva) and `286ec54` ("/caveman ultra", paseo). Opening the two closed agents moved them to idle on 6770. The reference is the in-place Electron build on 6770. The example window is 900 x 792 (title bar 32 px) and the reference 1200 x 800, so offsets are measured from the top of the timeline list. A GPUI window draws only while in front, and the reference stops painting while the example covers it, so each app was captured alone. Screenshots: `/tmp/gpui-ui-check/E/`; crash logs: `crash-shell-badge*.log`.

| Line                                       | Result     | Evidence                                                                                                                                                                                                                                                                                                                                  |
| ------------------------------------------ | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Tool row expand (shell badge)              | FAIL       | Pick step 1 of the "resume work" turn in `286ec54`, then click the `Shell: git status ...` badge: the process aborts with `panicked at gpui-pre-0.3.9/src/element.rs:557:18: must call prepaint before paint`, then `failed to initiate panic`. 3 of 3 tries, also on the second badge. Logs `crash-shell-badge.log`, `-2.log`, `-3.log`. |
| Other tool kinds (read, edit, write, ...)  | UNVERIFIED | The crash blocks every badge expand; the reference agents hold only shell, read, tool-search, and task-stop rows. Edit rows, search, fetch, sub-agent: no reference agent shows them.                                                                                                                                                     |
| Plan card, question card                   | UNVERIFIED | No agent on 6770 holds a plan or question card, and live is down.                                                                                                                                                                                                                                                                         |
| Short conversation starts at the top       | PASS       | `380c019`: the list opens at the top, the user bubble first, blank space below the footer.                                                                                                                                                                                                                                                |
| Message and block spacing                  | FAIL       | `380c019`, offsets from the list top, GPUI vs reference: bubble top 32 vs 48; bubble bottom to first reply line 41 vs 54; paragraph pitch 33 vs 48 (line height 24 in both); "Next Steps" rule to code block 13 vs 38; code block 105 vs 92 high; code line pitch 19.3 vs 17.3. Heading widths equal ("Next Steps" 122 vs 123 px).        |
| Headings with rules                        | PASS       | `7d4ad155`: "Check Heading" 200 px and "Second heading" 180 px wide in both; a 1 px rule under both; bold.                                                                                                                                                                                                                                |
| No fence label                             | PASS       | The `rust` fence shows no label in both.                                                                                                                                                                                                                                                                                                  |
| Code block look                            | FAIL       | `fn main() { println!("hi"); }`: GPUI block 48 px high, reference 42 px. Syntax colors differ: GPUI `fn`, `main`, `println` in dark blue bold; reference `fn` magenta, `main` blue, `println` dark grey regular.                                                                                                                          |
| Turn footer                                | PASS       | Copy button and "Worked for 5s" (`7d4ad155`), "Worked for 7s" (`380c019`), "Worked for 2m 28s" (`286ec54`). The reference adds the fork button and TTFT parts: known, out of scope.                                                                                                                                                       |
| Turn header and step rows                  | PASS       | "Worked for 2m 23s", 2-segment strip 171 px (reference 169), 2 steps, number circles, "EXPLORE ran 2 commands", times at right; header to step 1 41 vs 39 px, step to step 38 vs 37, step to body text 24 vs 22, body indent 34 in both.                                                                                                  |
| File link click                            | PASS       | Click on `main.rs`: stderr `OpenFile { cwd: "/tmp/gpui-bench", path: "/tmp/gpui-bench/src/main.rs", line: Some(3) }`. Click on inline `src/extra.rs`: `path: "/tmp/gpui-bench/src/extra.rs", line: None`.                                                                                                                                 |
| Image rendering                            | PASS       | `shot.png` from `/tmp/gpui-bench` renders at 478 x 62, as in the reference; a click opens the lightbox with a close button. The missing-file retry state was not run (it needs a second reply on a reference agent): UNVERIFIED.                                                                                                          |
| Find, Cmd+F                                | PASS       | Cmd+F opens the bar (338 px, reference 342); "heading" showed `3 of 4` after two Enter presses, current match darker. Mismatch: the reference field has a 2 px focus ring; the GPUI field shows none.                                                                                                                                     |
| AX roles and labels                        | PASS       | `list Timeline`, `User message: ...`, `Assistant message: ...`, `Turn, done, 2 steps` heading, `2 step strip` with `Step 1`, `Step 2` buttons, `Tool call Shell: ...` containers with a `button Shell: ...`, `Copy turn`, `Copy code`, `Open image attachment: shot`.                                                                     |
| Live answer or reject, CPU at 2 and 15 min | UNVERIFIED | Live (6768) is down; no mock provider on 6770.                                                                                                                                                                                                                                                                                            |

Cleanup: the example and the reference app were quit; the lock was released; agent `7d4ad155` archived; 6767 untouched; no terminal was created.

## Fix Round 2

Status: DONE. Screen checks are deferred to the UI checker. Files: `crates/timeline/src/lib.rs` and `crates/timeline/src/tool_view.rs`. `TimelineView::new`, `find`, and `TimelineEvent` are unchanged, and the crate exports nothing new.

### Crash: "must call prepaint before paint"

- Root cause: the icon slot of a collapsed, openable tool badge (`tool_view.rs`, `render_badge`, the old lines 243-253) swapped the tool icon and the chevron with `group_hover(|s| s.hidden())` and `hidden().group_hover(|s| s.flex())`.
- GPUI decides group hover twice per frame. Prepaint tests the new group hitbox id against the last frame's hit test (`gpui-pre-0.3.9/src/window.rs:3740` runs `mouse_hit_test` only after prepaint). Hitbox ids never reset (`window.rs:5299`), so no new hitbox is hovered in prepaint. Paint tests again against the fresh hit test.
- `Div::prepaint` skips its children when `display` is none (`elements/div.rs:2028`), and `Div::paint` checks `display` again (`elements/div.rs:2084`). With the pointer over the badge, the chevron layer was hidden in prepaint and flex in paint, so paint reached a chevron that was never prepainted: the panic at `element.rs:557`. Any redraw with the pointer on a collapsed badge did it, which is why every badge crashed.
- Fix: the two layers stack absolutely in the slot and swap `visibility` (`invisible()` / `visible()`). Prepaint never skips children for visibility, and paint only skips painting them. The crate has no other `display` change on hover.
- Test: no GPUI test context without the gpui-kit `test-support` feature (CONTEXT.md forbids it), so `toggle_opens_and_closes_a_key` covers the pure state change (`toggle`, now shared by `toggle_expanded` and `toggle_diff`). The expand path itself is DEFERRED (UI checker).

### Spacing

The web stream's model, from `strategy-web.tsx`, `spacing.ts`, `layout.ts`, `message.tsx`, and `markdown-styles.ts`:

- List top: content padding 16 plus the 32 px history slot, so the first row starts 48 below the list top (was 32).
- User message: no outer margin on web (`disableOuterSpacing`). The bubble keeps the trailing row under it (8 + 24, hidden until hover), then `getGapBetweenStreamItems`: 0 above an assistant message, 4 above a user message, 16 above anything else.
- Assistant message: padding 12 on top. Below: the last block's 12 and padding 12, then the gap (4 above tool calls or a turn, 16 above others). Above a footer it is 12 only (`compactBottom`), so the footer spacing is unchanged.
- The space depends on the next row, so `sync` mixes it into the message row's fingerprint, and `apply_rows` remeasures the row when the row under it changes. The pass reads only grouper facts, plus one item type for the row under a message (ADR-0039).
- Markdown blocks: a paragraph keeps `marginBottom: 12` and each blank-line block 12 more, so `paragraph_gap` is 24. Headings add the block's 12 to their bottom (h2 margin 24, h3 padding 20).
- Expected offsets from the list top: bubble top 48, bubble bottom to the first reply line box 44 (32 + 0 + 12), paragraph pitch 48 at line height 24, heading rule to code block about 37.

### Code block

- Fence: margin top 12, padding 12, `surface2`, 1 px border, radius md, the bachuc mono family, code size 12, line height `round(12 * 1.45)` = 17. One line is 43 px high (reference 42).
- Syntax colors: the Electron default syntax theme is "one" (`use-settings/storage.ts:137`), not GitHub. `one_colors` ports `oneLight` and `oneDark` through `expandRolePalette`. Fences highlight with it through one shared highlighter `Arc` per theme, so a block highlights once, not on every frame. Tool detail code and diffs use the same colors.
- Limits: tree-sitter captures are not lezer tags. A macro name keeps the text color as in the app (`function.macro` has no role). Other captures can differ from lezer at the edges. Only the crate's grammars highlight: rust, typescript, tsx, javascript, python, bash, and json through the explorer. Another fence language shows plain. The Electron syntax theme setting is not read; GPUI always uses "one".

### Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-timeline` -> PASS: 60 tests. New: `toggle_opens_and_closes_a_key`, `fence_languages_follow_the_app_aliases`, `fence_colors_follow_the_one_theme` (`fn` #a626a4, `main` #4078f2, `"hi"` #50a14f, `println` no color), `headings_hold_the_block_gap_below`.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-timeline` -> PASS.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh clippy -p bachuc-timeline --all-targets` -> PASS: no timeline warning; only the existing `bachuc-protocol` build script warnings.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> PASS.
- GitNexus `impact` upstream: `sync` HIGH (12, all inside `TimelineView`; signature unchanged). `render_entry` LOW. `render_row` UNKNOWN: it has no caller edges because it is passed to `list` through `cx.processor`. The text search shows only `render`. `render_badge`, `markdown_style`, and `syntax_runs` are not in the index, which is 33 commits behind. They are private, and the text search shows callers only in the crate.
- Pick step 1 in `286ec54` and hover, then click a `Shell: ...` badge: no crash, and the details open -> DEFERRED (UI checker).
- Offsets on `380c019` against the reference (bubble top, bubble to reply, paragraph pitch, rule to code, code block height and colors) -> DEFERRED (UI checker).

### Lessons

- Never toggle `display` (`hidden()`, `flex()`) from `hover` or `group_hover` on an element with children in GPUI. Toggle `visibility`, or stack layers. Prepaint and paint see different hover states, and a child shown only in paint panics.
- Process note: one edit in this round, which turned three new `pub fn` into `pub(crate) fn` in `tool_view.rs`, used `sed -i` instead of the edit tool. The content is as intended.

### Re-check 2

Run by the UI checker on 2026-10-16, HEAD abf2569f1 (fix commit abf2569f1), light theme. Bundle `BachucTimeline` rebuilt before the run. The example ran on `ws://127.0.0.1:6770/ws` (live is down). Reference: the in-place Electron build on 6770. The pointer and foreground owner rules arrived mid-run: every click below ran before them; the 6770 reference agents are `380c019` ("hi"), `286ec54` ("/caveman ultra"), and `99f8fc28` (made for this check: haiku-5-5, one turn with a fixed markdown reply, archived after). Offsets are measured from the list top (GPUI window 900 x 792 with a 32 px title bar, reference 1200 x 800). Screenshots: `/tmp/gpui-ui-check/E/*3*.png`.

| Line                                      | Result  | Evidence                                                                                                                                                                                                                                                                                                                                 |
| ----------------------------------------- | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Shell badge expand, no panic              | PASS    | `286ec54`: picked step 1 with the pointer on the step row, then clicked the first `Shell: git status ...` badge 3 times (open, close, open): the process stayed alive, stderr stayed empty (0 bytes), and the details opened (`gpui3-286-open1.png`). The second badge (`Shell: vm_stat ...`) opened too (`gpui3-286-badge2.png`).       |
| Other tool kinds                          | PARTIAL | Shell and "Task notification" (details show the `<task-notification>` text) open without a panic. Read, edit, write, search, fetch, sub-agent, plan: UNVERIFIED, no reference agent on 6770 holds them (live is down).                                                                                                                   |
| Message spacing (`380c019`)               | PASS    | Bubble top 48 vs 48. Bubble bottom to first reply ink 51 vs 50. Paragraph pitch 48 vs 47. Heading ink top to its rule 31 vs 32. Heading to the next paragraph 32 vs 30. Bubble 53 px high in both.                                                                                                                                       |
| Heading directly followed by a code block | FAIL    | `380c019` "Next Steps" rule to code block top: GPUI 25 px (rule y 360, block y 385), reference 37 px (rule y 397, block y 434). 12 px short. Block height 94 vs 94, line pitch 17 vs 17. With a paragraph between (`99f8fc28`): heading rule to paragraph 32 vs 30, paragraph to block 53 vs 52, block to next paragraph 35 vs 33: PASS. |
| Code block, one line                      | PASS    | `99f8fc28`: block 43 px high (y 535 to 577), reference 43 (y 502 to 544; the earlier read was 42). `fn` magenta, `main` blue, `"hi"` green, `println` near black and regular, as in the reference. The one-theme hex values (#a626a4, #4078f2, #50a14f) could not be read exactly: thin glyph edges blend with the fill.                 |
| Footer, headings, no fence label          | PASS    | Copy button and "Worked for 14s" (reference adds fork and TTFT: out of scope); heading and rule widths match; no `rust` label.                                                                                                                                                                                                           |

Not run (owner rules, 2026-10-16): the hover-then-click badge step was run with pointer clicks before the pointer rule; a repeat is `UNVERIFIED (unattended): needs pointer, owner rule 2026-10-16`. CPU reads and live answers stay `UNVERIFIED (unattended): live down`.

Cleanup: the apps were quit; agent `99f8fc28` archived; its workspace `wks_671d9c2c2abfbaa1` (`main`, `/tmp/gpui-bench`, on 6770) was left because the archive command was declined; no terminal was created in this group; 6767 untouched.

## Fix Round 3

Status: DONE. Fixes the "Heading rule to code block" row (Q-086): GPUI 25 to 26 px, reference 37. File: `crates/timeline/src/lib.rs`. `TimelineView::new`, `find`, `TimelineEvent`, and the per-flush work of ADR-0039 are unchanged; the change is in the cached markdown style only.

### Cause

- gpui `div` defaults to `Display::Block` (`gpui-pre-0.3.9/src/style.rs:774`), and gpui-base's TextView root and block wrappers are plain divs (`gpui-base-0.7.3/src/text/document.rs:234`, `gapped` in `node.rs:2906`). taffy 0.13 block layout collapses adjoining vertical margins as CSS does (`taffy-0.13.0/src/compute/block.rs:1185` and `1222`).
- An h1 or h2 holds its space under the rule as a bottom margin of 24, because the rule is its bottom border. The fence had a top margin of 12. The two collapsed to 24, plus the 1 px rule: the measured 25.
- The web stream stacks blocks in flex columns, which never collapse: h2 `marginBottom: 12`, the block's 12, and the fence's `marginVertical: 12` give 36, plus the rule: 37.
- A paragraph's gap is gpui-base padding (`gapped`), so paragraph to fence was already 24 + 12 = 36. h3 to h6 keep their space as padding, so they were not affected.
- No margin pair can give 36 and keep the paragraph cases: collapse takes the larger margin.

### Fix

- `fence_spacing`: the fence has no top margin. It moves down 12 with a relative `top` offset and has a 12 bottom margin, so the blocks under it keep their place. An offset never collapses. Heading to fence is now max(24, 0) + 12 = 36, and paragraph to fence stays 24 + 0 + 12 = 36.
- A last fence's bottom margin leaves the text view and stops at its flex-column parent (the assistant message `v_flex` and the step-note `v_flex`), so the row height includes it. The visible block ends where it did before.
- Test `fence_gaps_match_the_web_stream` computes both gaps from `heading_style`, `fence_spacing`, `BLOCK_GAP`, and `SPACE_3` with the collapse rule. It fails with the old `mt(12)`, which gives 24.

### Text row drift (TASK-155 K row)

- The K row shows reply text 2 to 4 px lower than the reference: first line 224 vs 222, second paragraph 272 vs 269, heading 369 vs 365. It grows about 1 px per block.
- Not the same cause: no two margins adjoin on that path. The bubble-to-reply gap is row padding, paragraph gaps are padding, and a heading after a paragraph uses padding on top. The 1 px per block matches the paragraph pitch of Re-check 2 (48 vs 47), which is line-box rounding. Not fixed in this round.

### Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-timeline` -> PASS: 61 tests, new `fence_gaps_match_the_web_stream`.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-timeline` -> PASS.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh clippy -p bachuc-timeline --all-targets` -> PASS: no timeline warning; only the existing `crates/protocol/build.rs` warnings.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> PASS.
- GitNexus `impact` upstream `markdown_style`: not found, because the index is behind. It is private, and the text search shows one caller, `render_markdown`, in the crate.
- `380c019` "Next Steps" rule to code block top is 37 px, and paragraph to block stays 52 to 53 -> DEFERRED (UI checker).

### Lessons

- gpui divs are block layout, and taffy collapses adjoining vertical margins. If a gap must add to a neighbor's margin, use padding or a relative offset with an equal bottom margin. Do not use a top margin.
