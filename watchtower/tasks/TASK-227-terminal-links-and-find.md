# TASK-227 Terminal links and find match Electron

Group: R (standalone: [packages/desktop-gpui/crates/terminal](packages/desktop-gpui/crates/terminal))
Class: code

## Brief

Goal: Terminal links work across wrapped rows, the find bar remembers its query, and file links ask the daemon for suffix matches.

Change: one row per link, the query is lost on close, fuzzy lookup with a client filter, Lucide icons by asset path -> Electron behavior. Q-134.

Boundaries:

- Wrapped rows: `RenderedLine` (`emulator.rs` line 101) has only `text` and `runs`; add a wrap flag there so `links.rs` can join rows. Electron joins wrapped rows ([packages/app/src/terminal/local-links/terminal-local-link-provider.ts](packages/app/src/terminal/local-links/terminal-local-link-provider.ts) lines 72, 198-228). GPUI reads one row (`links.rs` line 5).
- Find query: Electron keeps the query after close and runs it again on open ([packages/app/src/terminal/find/index.tsx](packages/app/src/terminal/find/index.tsx) lines 46, 94-98). GPUI drops it (`view.rs` lines 631-636).
- Suffix match: use `Client::directory_suggestions_matching` with `MatchMode::Suffix`; keep the Q-112 rule (exact path or a single match). `resolve` already filters by suffix on the client (`links.rs` line 299); keep that filter as a guard.
- Icons: depend on `bachuc-icons = { path = "../icons" }`, as the other crates do, instead of Lucide asset paths. `Cargo.lock` changes by one line.
- An attached terminal has no cwd; it keeps opening only absolute paths. Planner call.

Files:

- [packages/desktop-gpui/crates/terminal/src/links.rs](packages/desktop-gpui/crates/terminal/src/links.rs)
- [packages/desktop-gpui/crates/terminal/src/find.rs](packages/desktop-gpui/crates/terminal/src/find.rs)
- [packages/desktop-gpui/crates/terminal/src/view.rs](packages/desktop-gpui/crates/terminal/src/view.rs)
- [packages/desktop-gpui/crates/terminal/src/emulator.rs](packages/desktop-gpui/crates/terminal/src/emulator.rs) (wrap flag)
- [packages/desktop-gpui/crates/terminal/Cargo.toml](packages/desktop-gpui/crates/terminal/Cargo.toml)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock)

Interfaces:

- Consumes: `MatchMode` enum `{ Fuzzy, Suffix }` from TASK-220.
- Consumes: `Client::directory_suggestions_matching(&mut self, cwd: &str, query: &str, include_files: bool, include_directories: bool, limit: u32, mode: MatchMode, cx: &mut Context<Client>) -> Task<anyhow::Result<Vec<DirectorySuggestion>>>` from TASK-220.

Expected result:

- A path that wraps onto a second row opens the whole path.
- Cmd+F, type "hello", Escape, Cmd+F -> the field shows "hello" and its matches.

Anti-goal: a 200,000-line flood keeps its time; drift gauge; flood time from the terminal bench before and after, at most 5% more.

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-terminal` -> passes, with a two-row and a three-row wrapped-link case and a kept-query case.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- UI check on live: Cmd+F, "hello", Escape, Cmd+F -> the query is still there.
- Owner hand test: Cmd-click a wrapped path.
