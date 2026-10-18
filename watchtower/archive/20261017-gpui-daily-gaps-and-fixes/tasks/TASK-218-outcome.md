# TASK-218 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/history/Cargo.toml` (new): the `bachuc-history` crate. Path dependencies `bachuc-client`, `bachuc-icons`, `bachuc-theme`; `gpui-kit.workspace = true`; and `chrono` with the entry the board and thread-branch plugins use (`default-features = false`, `std` and `clock`).
- `packages/desktop-gpui/crates/history/src/lib.rs` (new): the pure rules and 14 tests. They cover sort, merge, date sections, local day, time ago, the `text-match.ts` highlight port, the body and footer state, and the effective search.
- `packages/desktop-gpui/crates/history/src/screen.rs` (new): `HistoryScreen` and `HistoryEvent`.
- `packages/desktop-gpui/Cargo.lock`: one new `bachuc-history` entry, 11 added lines. No other entry changed.

## Contract

- `HistoryScreen::new(client: Entity<Client>, window: &mut Window, cx: &mut Context<HistoryScreen>) -> HistoryScreen` fetches the first page at once.
- `HistoryEvent::OpenAgent { agent_id: String, workspace_id: Option<String> }` is emitted on a row click and on an AX press. `HistoryScreen` implements `EventEmitter<HistoryEvent>`. It does not implement `Focusable`.
- The screen consumes `HistoryEntry`, `HistoryPage`, `Client::fetch_agent_history`, and `Client::history_search_supported` from TASK-205 with their exact names.
- Search: the input is applied 200 ms after the last change (`SEARCH_DEBOUNCE`), trimmed. It is sent only when it is not empty and `history_search_supported()` is true, which the screen reads on each draw. The field is hidden when search is not supported. A new search drops the list and shows the loading state, as the Electron query key change does. A generation counter drops replies for an older search.
- Sort: newest `last_activity_at` first. An entry with no time, or one that does not parse, goes last and into Older. Pages are merged without duplicate `agent_id`s and sorted again.
- Groups: local day numbers through `chrono::Local`. 0 days or a future day is Today, 1 is Yesterday, 2-7 is This week, 8-30 is This month, and more is Older.
- Row: the workspace name (or the project name), a chevron, the provider logo, and the title; then the project (132 px), the branch (132 px), and the time ago (72 px, right-aligned). Badges: Archived (with the Archive icon) and "N pending" (warning tone). Search matches are marked in the label, the title, the project, and the branch with the Electron mark style: `surface3` background and `foreground` text.
- Footer: a truncated search shows "Too many matches — narrow your search". Otherwise "Load more" shows while `has_more` is true, and it reads "Loading..." while the next page loads. A failed Load more keeps the rows.
- Texts were copied from `en.ts` and `sessions-screen.tsx`: "History", "Search history", "Clear search", "No sessions yet", "No sessions match", "Unable to load sessions", "Try again", "Load more", "Loading...", "New session", and "Archived".

## Verified

- `./cargo.sh test -p bachuc-history`: 14 passed. These include `date_sections_follow_the_electron_day_limits`, `groups_keep_order_and_skip_empty_sections`, the four `highlight_*` tests, and `body_picks_the_empty_no_match_and_error_texts`. That test covers the empty text, the no-match text, and the error text.
- `./cargo.sh build -p bachuc-history`: exit 0, with no crate warnings.
- `./cargo.sh clippy -p bachuc-history --tests`: no `bachuc-history` warnings. The 2 warnings come from the existing `bachuc-protocol` build script.
- `./cargo.sh fmt --check -p bachuc-history`: exit 0.
- `./cargo.sh metadata --format-version 1`: exit 0. `grep -c 'name = "bachuc-history"' Cargo.lock`: 1.
- `./cargo.sh build -p bachuc-app`: exit 0.
- Anti-goal: UNSATISFIABLE as written, because a clean build needs `cargo clean`, which the builder rules forbid. `bachuc-app` does not depend on `bachuc-history` yet (TASK-200 adds it), so this TASK cannot change its build time. No-op builds took 0.83 s before and 0.68 s after. One build in between took 15.8 s, because the other builder's commit `c542cd8f8` recompiled explorer and timeline. Replace the check with a timing of `bachuc-history` alone, or measure it at TASK-200.
- UI check (open History, rows in date groups, search narrows them): DEFERRED (UI checker), after TASK-200.

## Lessons

- The initial load shows the text "Loading..." in place of the Electron spinner. The empty state has no Back or Import buttons, because both are out of scope.
- The time-ago labels and date groups are computed when the list changes, not on each draw. They do not age while the screen stays open.
- `chrono::DateTime<Tz>::format` needs `Tz::Offset: Display`. For a generic `Tz: TimeZone`, format the result of `naive_local()` instead.
- Clippy flags `vec![a..b]` (`single_range_in_vec_init`). Use `std::iter::once(a..b).collect()`.

## UI Checks

- Run 3 (2026-10-18, recorded in the TASK-200 outcome): History opens with Today, Yesterday, and This week groups and Archived pills: PASS (`/tmp/r3-02-history.png`).
- Search narrows the rows: UNVERIFIED (unattended): not run by the UI checker. Owner hand test, Q-133.

## Owner Hand Test

- 2026-10-18: Hiep ran the Q-133 checks of this TASK by hand on the "Bachuc GPUI Test" bundle built from `fc70abf5a`, on live: PASS.
