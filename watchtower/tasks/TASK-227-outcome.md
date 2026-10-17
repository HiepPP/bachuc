# TASK-227 outcome

## Status

DONE. The screen checks, the owner hand test, and the flood timing are deferred.

## Changed

- `packages/desktop-gpui/crates/terminal/src/emulator.rs`: `RenderedLine.wrapped`, read from the `WRAPLINE` flag of the row's last cell. 1 test.
- `packages/desktop-gpui/crates/terminal/src/links.rs`: `RowLink` and `link_in_rows`, which join wrapped rows (up to `MAX_LINK_LENGTH` = 500 bytes before and after, as Electron) and return the link's range in each row. `link_at` is gone. 3 tests.
- `packages/desktop-gpui/crates/terminal/src/find.rs`: `Search` (query, matches, limit, current, stale) with `run` and `close`. 1 test.
- `packages/desktop-gpui/crates/terminal/src/view.rs`: the view owns a `Search` that outlives the bar; Cmd-click and Cmd-hover use `link_in_rows`; file links call `directory_suggestions_matching` with `MatchMode::Suffix`; the bar icons come from `bachuc_icons`.
- `packages/desktop-gpui/crates/terminal/Cargo.toml`: `bachuc-icons = { path = "../icons" }`.
- `packages/desktop-gpui/Cargo.lock`: one line, `"bachuc-icons"` in `bachuc-terminal`.

## Contract

- `RenderedLine { text, runs, wrapped }`. `wrapped` is true when the row's text goes on in the next row.
- `links::link_in_rows(rows: &[(&str, bool)], row, col) -> Option<RowLink>`. `RowLink { target: LinkTarget, spans: Vec<(row, Range<usize>)> }`.
- `find::Search` keeps the query when the bar closes and drops the matches. Cmd+F on a closed bar puts the kept query in the field, searches it again, and shows the first match from the top of the screen down. The field text is selected, so typing replaces it.
- File links ask `Client::directory_suggestions_matching(cwd, query, true, false, 100, MatchMode::Suffix, cx)`. `links::resolve` keeps the Q-112 rule and its client suffix filter as a guard.
- Unchanged: an attached terminal has no cwd and opens only absolute paths.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-terminal` -> 55 passed (50 before plus 5 new): `a_path_wrapped_over_two_rows_opens_whole_from_either_row`, `a_path_wrapped_over_three_rows_opens_whole_from_the_middle`, `a_wrapped_url_opens_whole`, `a_closed_search_keeps_its_query_and_finds_it_again_on_open`, `a_row_that_wraps_is_marked_and_a_line_break_is_not`.
- `./cargo.sh fmt --check -p bachuc-terminal` -> exit 0.
- `CARGO_BUILD_JOBS=4 ./cargo.sh clippy -p bachuc-terminal --tests --examples` -> no warnings in the terminal crate. The 2 build-script warnings come from `bachuc-protocol`.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0. `Cargo.lock` diff is one added line.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> builds. Its one warning (`next_theme` unused, `crates/app/src/state.rs:424`) was there before.
- GitNexus impact: `RenderedLine` -> LOW. `link_at` resolved to a different function in `plugins/assistant-cite` (HIGH there); `rg` shows the terminal `link_at` was used only in its own crate. The view methods were not in the index; `rg` confirms they are private to `view.rs`.
- UI check on live: Cmd+F, "hello", Escape, Cmd+F -> DEFERRED (UI checker).
- Owner hand test: Cmd-click a wrapped path -> DEFERRED (owner).
- Anti-goal, 200,000-line flood timing -> DEFERRED (UI checker). By reading the code: a feed is unchanged apart from one `Option` check; each rebuilt row reads one more cell flag; a draw collects the screen line references into a `Vec` and joins rows only while Cmd hovers a link.

## Lessons

- Only screen rows are joined. A path whose start wrapped from a row above the screen top is cut at the top row.
- A wide character that does not fit at the row end leaves a spacer cell that draws as a space, so a path with a wide character at the wrap point is cut there.
- `InputState::set_value` sends no `Change` event, so the view searches the kept query itself on open.
