# TASK-161 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/sidebar/src/model.rs`: new `TreeSource` trait (for `Client` and `ClientCore`) and `Facts` (attention, project keys, diff stats, pin times, read by `Facts::read`). `build_rows` takes `&Facts`. `WorkspaceRow` gains `diff` and `color_key`; `ProjectRow` gains `color_key`. Pinned rows sort by `pinned_at`, newest first. New `archive_prompt_for_stat`. Removed `diff_stat(files)`.
- `packages/desktop-gpui/crates/sidebar/src/sidebar.rs`: removed `diffs`, `fetch_diff`, and both `checkout_status` calls. The row diff and the letter color read the rows. `archive` reads `workspace_diff_stat`.
- `packages/desktop-gpui/crates/sidebar/tests/model.rs`: new tests; one old test trimmed.
- `packages/desktop-gpui/crates/sidebar/tests/fake_daemon.rs`: new fake-daemon test.
- `packages/desktop-gpui/crates/sidebar/Cargo.toml`: dev-dependencies `futures`, `serde_json`, `tungstenite`, `uuid` for the fake daemon. `packages/desktop-gpui/Cargo.lock` gains those four lines under `bachuc-sidebar`.

## Contract

- `Sidebar::new`, `Sidebar::set_selected_workspace`, and `SidebarEvent` are unchanged. `bachuc_sidebar::model::archive_prompt` is unchanged, so `crates/app` `shell.rs` still builds.
- Attention: `Facts::attention` holds `Client::agent_attention`; `bucket_for_agent` gets the flag and the reason name. A missing entry gives `(false, None)`.
- Letter color: `identity_color` hashes `Client::project_view_key`, else the project id. The pinned rows lead with the same key.
- Diff count: `Some((a, d))` from `workspace_diff_stat` shows `+a -d`, also `+0 -0`. `None` shows nothing.
- Pinned order: `pinned_at` descending; a missing time compares as an empty string; the sort is stable.
- Projects still sort by name.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-sidebar` -> exit 0; `tests/model.rs` 21 passed, `tests/fake_daemon.rs` 1 passed.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-sidebar` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> Finished `dev` profile, no error.
- `rg "checkout_status|checkout\.status" packages/desktop-gpui/crates/sidebar/src` -> no match.
- GitNexus impact: not run. The index is for the TypeScript tree; `rg` shows the only outside user of the changed exports is `crates/app/src/shell.rs`, which uses `archive_prompt` only.
- Sidebar view drawing (Attention dot, letter color, `+0 -0`): UNVERIFIED (unattended): code tests only.
- Live checks on 6768: UNVERIFIED (unattended): live is down.

## Anti-goal

- Checkpoint: `tests/fake_daemon.rs` counts `checkout_status_request` on the fake daemon after the tree loads and 300 ms more -> 0, source `cargo.sh test -p bachuc-sidebar`, 2026-10-16.
- Checkpoint: `rg "checkout_status|checkout\.status" crates/sidebar/src` -> no match, 2026-10-16.
- The fake-daemon test reads the tree through `Facts::read` and `build_rows` on `ClientCore`. It does not run the GPUI view, because the repo has no headless GPUI test setup. The view sends no request by the `rg` check.

## Lessons

- Workspace updates set `directory_changed`, so every `diffStat`, `pinnedAt`, or attention change raises `AgentsChanged` and the sidebar refreshes. `ProjectsChanged` alone would miss them.
- A glob import is shadowed by a local function of the same name. `tests/model.rs` uses this for a `build_rows` wrapper.

## Questions

- Safe default: the spec says to remove both `checkout_status` calls. The second call was the archive confirm. The archive now reads `workspace_diff_stat` and asks when it has a changed line, which is Electron's rule when the daemon sends no dirty flag. A dirty tree whose base-ref diff is `0/0` archives without a confirm.
- Safe default: `archive_prompt(name, files)` stays for `shell.rs`, which is outside this group's files and still calls `checkout_status` on archive.
