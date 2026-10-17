# TASK-196 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/client/src/core.rs`: `WorkspaceMeta` gains `is_dirty: Option<bool>`, read from `gitRuntime.isDirty` in `upsert_workspace` (the page fetch and the live `workspace_update` upsert both go through it). Adds `ClientCore::workspace_is_dirty`.
- `packages/desktop-gpui/crates/client/src/lib.rs`: adds `Client::workspace_is_dirty`.
- `packages/desktop-gpui/crates/client/tests/rpcs.rs`: adds `workspace_is_dirty_reads_the_git_runtime_flag`.
- `packages/desktop-gpui/crates/sidebar/src/model.rs`: adds `archive_prompt_for_checkout(workspace_name, stat, is_dirty) -> Option<ArchivePrompt>`.
- `packages/desktop-gpui/crates/sidebar/src/sidebar.rs`: `Sidebar::archive` reads the stat and the dirty flag and calls `archive_prompt_for_checkout`.
- `packages/desktop-gpui/crates/sidebar/tests/model.rs`: adds the 9-case prompt table.

## Contract

- Produces `Client::workspace_is_dirty(&self, workspace_id: &str) -> Option<bool>` (and the same on `ClientCore`). `None` when the flag is absent or null, or the workspace is unknown. A workspace with no project info has no `WorkspaceMeta`, so it also gives `None`, as `workspace_diff_stat` does.
- `archive_prompt_for_checkout` keeps the stat rule first. A stat with a changed line gives the stat message. Otherwise `Some(true)` gives the same dialog with the message "Uncommitted changes". `None` and `Some(false)` keep today's rule.
- Every existing public item is unchanged: `archive_prompt_for_stat`, `archive_prompt`, `TreeSource`, and `Client::workspace_diff_stat` stay as they were. `TreeSource` gets no new method, because a required trait method would break an outside implementor; the view reads the client directly.
- The GPUI view change in `Sidebar::archive` is checked by reading only.
- GitNexus `impact upsert_workspace --direction upstream`: HIGH (33 impacted, 2 direct); the edit only adds one field read and leaves the other fields as they were. `impact archive_prompt_for_stat`: LOW (3).

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-client` -> exit 0, every suite ok, 0 failed; `workspace_is_dirty_reads_the_git_runtime_flag ... ok` covers `isDirty` true, false, null, absent, an unknown workspace, and a live `workspace_update` upsert from true to false.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-sidebar` -> exit 0; `tests/model.rs` 22 passed (21 existing + the 9-case table), `tests/fake_daemon.rs` 1 passed, `tests/plugin.rs` 24 passed, 0 failed.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-client -p bachuc-sidebar` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0, Finished; 0 crates compiled because the build was already current with these sources; the only warning is `next_theme` never used in `crates/app/src/state.rs` (not this TASK).
- The archive confirm on screen for a dirty 0/0 workspace -> UNVERIFIED (unattended): code tests only. Live daemon 6768 is down.

## Anti-goal

- Checkpoint 0, before any change (17:45): `archive_prompt_for_stat` asks only for a stat with a changed line, so stat none or 0/0 with `isDirty: true` archived at once (2 of the 5 changed cases missed); source `crates/sidebar/src/model.rs` and `tests/model.rs`.
- Checkpoint 1, at completion (17:55): `archive_from_the_checkout_asks_when_the_stat_or_the_dirty_flag_shows_changes` asserts the prompt shows in exactly 5 of 9 cases (stat +1 with any flag, and `isDirty: true` with stat none or 0/0); passed in the final `test -p bachuc-sidebar` run.
- Tripwire not hit.

## Lessons

- The spec pointed at `core.rs` near line 2205 for the workspace update path; that line is the checkout status reply. The workspace fields are read in `upsert_workspace`, which both the page fetch and the live `workspace_update` upsert call.
