# TASK-276 outcome

Status: DONE (CPU median 7.2%, cap 10% met; AX 79/79 nodes; screen checks pass)

## Step 1 profile

Source: this is the TASK-275 r1 sample `/tmp/gpui-sample-275.txt` of the `fff8e792d` check bundle (md5 `b0bed4e8...`, 1200x800 in front, during a running turn). It is NOT a fresh sample. `packages/` is unchanged at HEAD against `fff8e792d`, so it is a sample of the same code. The UI checker takes the fresh after-sample.

Method: parsed the main-thread call tree. `Window::draw` has 1019 samples (the 66 `MetalRenderer::draw` samples hang under `PlatformWindow::draw`, not under `Window::draw`). Each self sample goes to the nearest `bachuc_*` frame between the sample and `Window::draw`. A gpui-kit widget (Popover, DropdownMenu, Input, Icon) is billed to the view that owns it. A sample with no `bachuc_*` frame goes to the phase it sits in.

| Subtree                                                                   | Samples | Share of draw |
| ------------------------------------------------------------------------- | ------- | ------------- |
| Sidebar (list rows, kebab popovers, rings)                                | 255     | 25.0%         |
| Timeline list (rows, markdown, steps, background card)                    | 183     | 18.0%         |
| Whole-window taffy `compute_layout` (no owner; scales with all nodes)     | 175     | 17.2%         |
| Composer (card, textarea, pills, popovers, controls)                      | 164     | 16.1%         |
| gpui other (window border walk, scene finish, dispatch tree, arena drops) | 122     | 12.0%         |
| Tab bar and workspace chrome (`bachuc_workspace`)                         | 38      | 3.7%          |
| Plugin views and other crates (orb, spaces strip, board, explorer, ...)   | 36      | 3.5%          |
| A11y `begin_frame` and `end_frame`                                        | 23      | 2.3%          |
| Shell (`bachuc_desktop`: window shell, header)                            | 12      | 1.2%          |
| Taffy `clear`                                                             | 11      | 1.1%          |

By phase of `Window::draw`: `prepaint_as_root` 563 (views' prepaint 387 plus `compute_layout` 175), `request_layout` including each view's `render` 199, `paint` 167.

Bachuc-owned render functions, inclusive samples (the part this TASK can change): `Composer::render` 43, `Sidebar::render_row` 31 (`render_workspace` 23), `TimelineView::render_row` 23 (`render_source_entry` 7, background card 6, `render_step` 3), composer `plugin_pill` and `pill_states` 14, `Shell::render` 8, `Shell::render_header` 7. Together about 130 of 1019 samples (13%). The rest is element build, prepaint, taffy layout, and paint of the nodes those functions return.

Avoidable copies and allocations the table names, largest owner first:

| Table row | Where                                                                                                     | Evidence in the sample                                                                                 |
| --------- | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Sidebar   | `Sidebar::render_row` clones the whole `Row` of every visible row each frame                              | `render_row` 31, `WorkspaceRow::clone` 3 self; many `String` malloc and free                           |
| Sidebar   | `render_workspace` clones the `WorkspaceRow` again for the kebab menu closure                             | `render_workspace` 23, memmove 11 self                                                                 |
| Composer  | `Composer::render` clones the whole `ContributionIndex` (about 20 `Vec`s and 2 `HashMap`s) each frame     | `plugin_index` 5, `Vec<Contributed<SettingsScreen/SelectionAction/TimelineTransformer>>::clone` 1 each |
| Timeline  | `render_markdown` calls `markdown::blocks`, which copies every markdown segment text of a visible message | `Block::Markdown(text.clone())`, never read by the draw                                                |

Not cut (reason): taffy `compute_layout` 175 and the element walk scale with the node count; fewer nodes would change AX nodes or pixels (ADR-0053 Keep). The sidebar kebab popover per row (41 samples) is an AX node. The list re-renders its visible rows each frame by design in gpui-pre.

## Changed

- `packages/desktop-gpui/crates/sidebar/src/sidebar.rs`: `rows` is `Rc<Vec<Row>>`. `render_row` borrows the row from a cloned `Rc` instead of cloning the `Row`; the kebab menu closure of `render_workspace` holds the `Rc` and the index instead of a cloned `WorkspaceRow`.
- `packages/desktop-gpui/crates/composer/src/composer.rs`: `Composer::render` takes the plugin index from `index_cache` (`Option<(Option<u64>, Rc<ContributionIndex>)>`), keyed by the host `timeline_revision()`. New pure `cached_index` and one unit test.
- `packages/desktop-gpui/crates/timeline/src/markdown.rs`, `lib.rs`: `Block::Markdown(String)` is now the unit variant `Block::Markdown`. `render_markdown` read the text from `message.states`, never from the block, so each frame copied the text of every visible message for nothing.

## Per-frame cuts

| Cut | Step 1 table row targeted                                               | What no longer happens each frame                                                       |
| --- | ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| 1   | Sidebar (255 samples; `render_row` 31, `render_workspace` 23)           | one `Row` clone per visible row, and one `WorkspaceRow` clone per visible workspace row |
| 2   | Composer (164 samples; `plugin_index` 5, `Vec<Contributed<..>>::clone`) | one `ContributionIndex` clone (about 20 `Vec`s and 2 `HashMap`s of contributions)       |
| 3   | Timeline (183 samples; `render_markdown`)                               | one `String` copy per markdown segment of every visible assistant message               |

Order: by owner row, largest first (Sidebar, Composer, Timeline). The sample cannot size a cut alone; the sample counts above are the inclusive cost of the function that held the copy, not the saving. Expected saving is small: the three functions are 13% of `Window::draw` together, and the copies are part of them. The bulk (taffy `compute_layout` 175, element build and paint) scales with the node count and stays.

## Contract

- No new public API. `bachuc_theme::motion` and `bachuc_timeline::motion` untouched.
- `Block::Markdown` lost its `String`. It is a `pub` enum in `bachuc_timeline::markdown`; `rg` finds no use outside `crates/timeline` (the draw and one test).
- Behavior for existing input, classified with the old value and the new value:
  - Sidebar menu: the kebab closure read a `WorkspaceRow` copied at render; it now reads the `Row` at the same index of the `Rc` shared at render. Same row, same snapshot. Intended (this cut).
  - Composer pills: the index is rebuilt when `timeline_revision()` changes, and that revision grows whenever the contribution set changes (`set_active` returns `changed`) and on `invalidate_timeline`. A host that appears later changes the key from `None` to `Some`, so it rebuilds. Same pills. Intended.
  - Markdown: `Block::Markdown(text)` -> `Block::Markdown`. The test `broken_mermaid_renders_as_a_code_block_when_the_renderer_fails` asserted `text.starts_with("Here")` on the block; it now asserts the variant, and the text stays asserted on `segments(BROKEN)` in the same test. Intended.
- Pixels and AX nodes: no element, style, label, or id changed.

## Verified

- Group-level run (shared with TASK-277, one command): `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app -p bachuc-sidebar -p bachuc-timeline -p bachuc-composer` -> exit 0. `bachuc-app` 90, `bachuc-composer` lib 126 (incl. new `a_frame_copies_the_plugin_index_only_when_the_host_revision_changes`) and Hiep's `text_replacement` 4, `bachuc-sidebar` lib 14, `fake_daemon` 1, `model` 43, `plugin` 32, `bachuc-timeline` lib 32 and every integration test file. No Known Failure hit.
- `./cargo.sh build` (workspace) -> Finished. One pre-existing warning (`next_theme` never used, `crates/app/src/state.rs:458`).
- `./cargo.sh fmt --check -p bachuc-timeline`, `-p bachuc-composer`, `-p bachuc-sidebar` -> exit 0.
- `rg -n "\.cached\(" packages/desktop-gpui/crates packages/desktop-gpui/plugins` -> no match (exit 1).
- Outcome check: the Step 1 table above splits the `Window::draw` samples by subtree and was written before the first TASK-276 source edit.
- Release bundle: `./bundle.sh bin bachuc-desktop "Bachuc GPUI Check" io.github.hieppp.bachuc-desktop-check` -> exit 0. md5 of `target/bundles/Bachuc GPUI Check.app/Contents/MacOS/bachuc-desktop` and of `target/release/bachuc-desktop`: both `1b1d271557162efd966eef6e5c4dd2be`. `strings -a` finds `Scroll to bottom` and `scroll-to-bottom` (TASK-277). Not opened.
- `git status`: Hiep's files under `vendor/gpui-base/**` and `crates/composer/tests/` have the same entries as at the start; `Cargo.lock` unchanged.
- GitNexus: the index is stale. `render_row` is private to `sidebar.rs` with one caller (the list processor); `Composer::render` and `render_markdown` are called by GPUI and one method. No HIGH or CRITICAL result; `render` by name is ambiguous (UNKNOWN), confirmed by `rg`.

DEFERRED (UI checker): binary proof (bundle md5 equals `target/release`, differs from BASE `b0bed4e8...`); AX dump during a running turn with the same node count and click actions on the `fff8e792d` bundle and the new bundle; typing in the composer during a running turn; two screenshots about 200 ms apart with moving streamed text, orb, ring, and loader; theme, zoom, and window-resize states in every subtree.

## Anti-goal

DEFERRED (UI checker): three `measure.sh 60 check=pids:<pid>` reads during a running turn at 1200x800 in front, median at most 10%, plus a new `sample` top 10. Not measured here (no app window opened). The start is the TASK-275 median 14.9%. UNVERIFIED: the three cuts are small next to the 175 `compute_layout` samples and the element walk, so the cap may not be met. If the median stays above 10%, ADR-0053 `Revisit If` applies: record the read and the remaining top frames, and set the TASK `BLOCKED` with a question; no rejected option (`.cached`, slower frame rate, folded steps, higher budget).

## Lessons

- The sample splits cost by view only through the nearest `bachuc_*` frame; gpui-kit widgets (Popover, Input) belong to the view that builds them. The `PlatformWindow::draw` name also matches a `Window4draw` substring search, so exclude it when you count `Window::draw`.
- Of 1019 `Window::draw` samples, the render functions of Bachuc code are about 13%. A cut in a `render` removes a copy; it does not remove the layout and paint of the nodes it returns.
- A `render` that needs `&mut self` and a row of `self` can share the rows as `Rc<Vec<Row>>` instead of cloning the row.

## UI Checks

Checker: UI checker, Sonnet 5.5, 2026-10-20 10:21 to 10:48 local, live `127.0.0.1:6768`. Owner override: front allowed, no cursor use beyond `cua_repl` AX actions. Release daemon and release apps untouched. Evidence files are in `/tmp` (listed per line).

### Binary proof - PASS

- NEW bundle `Bachuc GPUI Check.app` binary md5 `1b1d271557162efd966eef6e5c4dd2be` = `target/release/bachuc-desktop` `1b1d271557162efd966eef6e5c4dd2be`.
- BASE `/tmp/wt-base-276/Bachuc GPUI Base 276.app` md5 `b0bed4e801d46cbe4a25b6db6c555cb3` (differs from NEW).
- `strings -a` finds `Scroll to bottom` in the NEW binary (1 hit).

### Setup

- Running turn: agent `44dd140` (Claude Haiku 5.5, `/tmp/wt-mock-275`, title `wt-276 running`), `sleep 580` steps, started 10:21:19. Its turn ended on its own before 10:46 after 3 steps (status idle at 10:46:51), so the later checks ran after the turn.
- The failed first `run` attempt (bad mode `bypass`) created an extra empty workspace `wks_e0bfe142019503de` named `wt-mock-275` on live. It stays on live (no agent in it).
- Window 1200x800 (default size, `BACHUC_WINDOW` unset), default layout (sidebar open, one agent tab, composer), release plugin set as live has it.

### AX drift gauge - PASS

Same thread (`wt-276 running`, step 1 running, `sleep 580` Background card), both apps launched with `BACHUC_NO_ACTIVATE=1` and not focused, 1200x800, one dump each in the same minute. Counts are lines of the AX text with a node index; clickable = button + tab + splitter roles.

| App  | Nodes | button | tab | container | click-capable (button+tab) |
| ---- | ----- | ------ | --- | --------- | -------------------------- |
| BASE | 79    | 45     | 4   | 14        | 49                         |
| NEW  | 79    | 45     | 4   | 14        | 49                         |

- Normalized trees (indexes and the TTFT text removed) are identical (`diff` empty): `/tmp/ax-276-baseC-1.txt`, `/tmp/ax-276-newB-1.txt`; two dumps per app gave the same count.
- "Scroll to bottom" shows in neither dump (the thread was at the bottom, following).
- Focus matters: a focused composer adds one `container` child under the text entry (BASE 80 nodes when focused: `/tmp/ax-276-baseB-1.txt`). Compare apps in the same focus state.
- The AX text has no frame or action list. Click actions are counted by role only.

### CPU (anti-goal) - PASS by value, with two caveats

`measure.sh 60 rN=pids:<pid>` on NEW pid 43329, window 1200x800, composer, sidebar open, one agent tab, during the running turn (step 1, then step 2 running).

| Set | Read | Start    | CPU % | Note                                                                                     |
| --- | ---- | -------- | ----- | ---------------------------------------------------------------------------------------- |
| 1   | r1   | 10:29:42 | 7.4   | screen stale in this window (see caveats), `sample` ran during r1                        |
| 1   | r2   | 10:30:42 | 7.6   | same                                                                                     |
| 1   | r3   | 10:31:42 | 6.9   | same                                                                                     |
| 2   | r1   | 10:35:28 | 6.9   | clock advanced in screenshots at 10:35:29, 10:36:25, 10:37:22, 10:37:50 (14:10 to 16:31) |
| 2   | r2   | 10:36:28 | 7.2   | same                                                                                     |
| 2   | r3   | 10:37:28 | 7.3   | same; `sample` ran during r1                                                             |

- Counted set: set 2. Median 7.2% (values 6.9, 7.2, 7.3). Set 1 median 7.4%. Both at most 10%. TASK-275 start: 14.9%.
- Caveat 1: in set 1 the window showed a frozen timeline (clock stuck at 7:16 from about 10:29:30 to 10:34, step 2 not drawn) although `ps` showed CPU time and `sample` showed `Window::draw`. A click revived it. In set 2 `cua_repl` screenshots every 28 s kept the window live and the clock moved.
- Caveat 2: BASE and NEW share the bundle id `io.github.hieppp.bachuc-desktop-check`. `open <path>` and `cua.getApp(path)` may reach the other copy. The CPU is from `ps` on the pid, so it is NEW's. Whether NEW was fully visible for the whole read is not proven. Treat 7.2% as a measured value, and not as proof of the cut size: the three cuts are small, and the drop from 14.9% is larger than they predict. A BASE read under the same procedure was not taken (time).
- Front log (`/tmp/front-log-276.txt`, one line per 5 s over set 1 and 2 reads): NEW in front except one line at 10:31:22 (another app, 3 s or less).

### Sample (`sample 43329 10`, `/tmp/gpui-sample-276b.txt`, during set 2 r1) vs TASK-275 (`/tmp/gpui-sample-275.txt`)

Same parser on both files (`/tmp/parse-sample-276.py`; it reproduces the TASK-275 numbers 1019 and 18.0%).

| Metric                       | TASK-275 (BASE) | NEW (set 2 r1) |
| ---------------------------- | --------------- | -------------- |
| Main-thread samples          | 5666            | 7140           |
| `Window::draw` samples       | 1019            | 390            |
| `Window::draw` share of main | 18.0%           | 5.5%           |

Top 10 self frames under `Window::draw` on NEW: `_platform_memmove` 38, taffy `compute_flexbox_layout` 25, `_xzm_xzone_malloc_tiny` 24, gpui `Style::refine` 13, taffy block `compute_inner` 11, `_xzm_free_main` 9, `<deduplicated_symbol>` 9, gpui `BoundsTree::insert` 8, gpui `ElementId::eq` 6, gpui `Div::prepaint` 6 (smol_str `Repr::new` 5 and an iterator `Filter` 5 follow). Same shape as BASE: memmove, taffy layout, malloc. `_platform_memmove` fell from 132 to 38 samples.

### Screen checks

- Typing in the composer - PASS (text), caret UNVERIFIED in the same shot. `setValue "typing check 276"` on the composer (not sent); AX read back `Value: typing check 276`; the screenshot `/tmp/ui276-e.jpg` shows the text. After clearing, the empty composer shows the caret (`/tmp/ui276-i.jpg`). Streamed text landed in the timeline: "Step 1 done." appeared as a step message while the turn ran (`/tmp/ui276-e.jpg`).
- Motion in two screenshots - PASS. Four shots at 0, 91, 165, 237 ms (`/tmp/ui276-m0..3.jpg`): pixel sums differ from shot 0 in the composer orb (38657 to 56665), sidebar ring (11996 to 18088), tab spinner (11489 to 16928), and loader dots (30867 to 52623). The clock text region did not change (1 fps).
- Zoom change - PASS. `View > Zoom In`: sidebar, header, tabs, timeline, and composer all rescaled in one shot (`/tmp/ui276-zin.jpg`); `Zoom Out` returned to the same layout (`/tmp/ui276-zout2.jpg`).
- Window resize - PASS. The window-zoom action maximized the window; every subtree reflowed in one shot (`/tmp/ui276-wzoom.jpg`); the second action restored 1200x800 with full paint (`/tmp/ui276-wback.jpg`).
- Theme change - PASS. Settings > Appearance > Claude dark: after Back, sidebar, header, tab bar, timeline, and composer were all dark in one shot (`/tmp/ui276-theme-dark-main.jpg`). Restored to Claude Light (`/tmp/ui276-theme-light-main.jpg`).
- Read check: `rg -n "\.cached\(" packages/desktop-gpui/crates packages/desktop-gpui/plugins` -> no match (see Verified above).

### Restore

See the TASK-277 outcome `## UI Checks` Restore for the shared state (apps, agent, zoom, theme).
