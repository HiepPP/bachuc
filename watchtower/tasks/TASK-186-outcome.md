# TASK-186 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/plugins/thread-branch/Cargo.toml` (new): crate `bachuc-plugin-thread-branch`; depends on `bachuc-plugin-host`, `bachuc-client`, `bachuc-theme`, `bachuc-icons`, `gpui-kit.workspace = true` (no `features`), `serde`, `serde_json`, `anyhow`, `log`, `chrono`, and `base64`; no view crate.
- `src/branch.rs` (new): the `BranchInfo` wire type and the pure pill rules: `pill_label`, `draft_pill_label`, `pill_title`, `sync_state`, `sync_tone`, `repo_name`, `branch_pill`, `pr_pill`, `changes_pill`, `branch_menu`, `draft_menu`, `copy_text`.
- `src/turn.rs` (new): the turn diff wire types and rules: `turn_diff_header`, `image_type`, `parse_unified_diff`, `badge_label`, `max_line`, `gutter_width`, `too_long`, `bar_blocks`, `bar`, `file_row_pressable`, `file_kind`, `split_path`, `turn_time`, `decode_data_uri`.
- `src/selection.rs` (new): `FileSelection`, `selected_row`, and `Selection`, the one `Rc<RefCell<Option<FileSelection>>>` store of the cards and the panel, with an entity that redraws observers on a change.
- `src/store.rs` (new): `Branches`, the `BranchInfo` store per cwd (`State` is the pure part: `ask`, `due`, `apply`, `failed`), the 15 s poll task, the read, copy, open, fetch, and refresh actions, and `live_cwds`.
- `src/pills.rs` (new): the three agent pills (`thread-branch`, `thread-branch-pr`, `thread-branch-changes`), the draft pill (`thread-branch-draft`), and the `BranchMenu` popover view (rows drawn like the Electron pill menu).
- `src/card.rs` (new): the turn diff card (`render_card`) and the timeline view `TurnCardView`.
- `src/panel.rs` (new): the Turn diff panel `TurnDiffPanel` (history, file diff, image preview).
- `src/lib.rs` (new): `plugin()`, the pills, the `turn-diff` panel, the `thread-branch-turn-diff` renderer, the `open-turn-diffs` command, and the `github-pr` attachment source.
- `tests/plugin.rs` (new): the plugin id is `thread-branch`.
- `packages/desktop-gpui/Cargo.lock`: generated. It also holds the entries of the other builders' crates.
- The server part `hiep-plugins/plugins/thread-branch` is unchanged.

## Contract

- Produces: `pub fn plugin() -> Box<dyn bachuc_plugin_host::NativePlugin>` with `id()` `"thread-branch"`; it is the only public item.
- RPCs, all through `PluginContext::rpc_as`: `thread-branch.get` (`{cwd}`, `{cwd, force: true}`, `{cwd, fetch: true}`), `thread-branch.turn-history`, `thread-branch.file-diff`, `thread-branch.file-image`. The picker gets `search_method` `thread-branch.search-prs`.
- Agent pills: `PillPlacement::Track`, `ShowOnDraft::No`. An agent with no workspace id or no cwd gets none. The branch pill is a flush `Popover` (menu rows Copy branch name, Open repo, divider, Fetch, Refresh; hidden rows and stray dividers drop as `visibleMenuItems` does); the PR and changes pills are `Action`s.
- Draft pill: `ShowOnDraft::Only`, `PillPlacement::Track`, flush `Popover` with Copy branch name, divider, Fetch, Refresh; the cwd is `ComposerDraft::cwd`.
- Loading: `Branches` keeps `BranchInfo` per cwd. A pill `state` closure calls `ask`, which starts a read for a missing cwd with `App::spawn`. The poll task (held in the store, which only the contribution closures hold) reads each asked cwd that an agent or a live draft still uses, once per 15 s tick. `PluginContext::notify` runs only when the info of a cwd changed. A failed read keeps the last info.
- Turn diff renderer: kind `thread-branch-turn-diff`, version 1. A file row presses only with a `source` and not `large`; a press stores the selection and calls `open_panel("turn-diff", PanelTarget { workspace_id: source.workspace_id, agent_id: Some(agent_id), location: None })`.
- Panel `turn-diff`: title `Turn diff`, icon `FileDiff`, `PanelContext::Agent`, `PanelLocation::Workspace`. History re-reads every 15 s while no file of the agent is open. Diffs and images are cached by root, from, to, and path for the panel's life. At most 2,000 rows; the Wrap switch starts on and its value lives for the session (`Rc<Cell<bool>>` in `contribute`).
- Command `open-turn-diffs` (`CommandContext::Agent`) clears the selection and opens the panel; no workspace id does nothing.
- Attachment source `github-pr` with the title, picker title, and placeholder of the spec.
- Colors: markers use `diff_addition` and `diff_deletion`; the two row tints are the named constants `ADD_ROW_TINT` and `REMOVE_ROW_TINT` in `panel.rs` that cite `diff-colors.ts:4-5`.
- Icons: `CloudDownload` (Fetch rows), `FileImage` and `FileCode` (card file rows), and `File` for a binary file.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-plugin-thread-branch` -> exit 0; 37 lib tests and 1 integration test pass (`/tmp/tb-test4.txt`), 2026-10-17 01:22. They include every case of `tests/label.test.ts` (`label_shows_branch_without_pr_suffix_detached_sha_and_truncation`, `new_workspace_label_names_the_selected_projects_branch_or_why_there_is_none`, `title_carries_upstream_sync_counts_dirty_state_and_pr_state`, `pr_pill_is_a_one_click_action_hidden_without_a_pull_request`, `branch_pill_adds_nonzero_push_and_pull_counts_and_tints_its_icon`, `branch_menu_opens_the_origin_repository_hidden_without_a_web_remote`, `branch_menu_offers_fetch_only_when_the_branch_has_an_upstream`, `sync_state_puts_pulling_first`, `changes_pill_shows_file_and_line_counts_only_when_the_work_tree_is_dirty`), the `imageType` case (`knows_which_paths_are_previewable_images`), the `parseUnifiedDiff` case (`numbers_old_and_new_lines_and_reads_the_file_status`, plus `parse_unified_diff_reads_added_deleted_and_binary_files`), and `turn_diff_header_cases`, `branch_label_counts`, `pills_hidden_rules`, `draft_label_rule`, `menu_rows_visibility`, `bar_blocks_rule`, `gutter_width_rule`, `file_row_pressable_rule`, `selected_row_rule`, `data_uri_decodes`, `turn_time_format`, `poll_one_read_per_cwd`, and `same_signature_no_notify`.
- `rg -n "bachuc-(composer|sidebar|timeline|command|settings|workspace|explorer|app)" packages/desktop-gpui/plugins/thread-branch/Cargo.toml` -> no match (exit 1).
- `rg -n "gpui-kit" packages/desktop-gpui/plugins/thread-branch/Cargo.toml` -> one line, `17:gpui-kit.workspace = true`.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-plugin-thread-branch` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 (`/tmp/tb-app.txt`). `bachuc-app` does not link this crate until TASK-191; its one warning (`next_theme` never used) is in `crates/app`.
- `cargo.sh clippy -p bachuc-plugin-thread-branch --all-targets` -> exit 0; no warning in this crate (warnings remain in `crates/plugin-host` and `crates/protocol`, not changed here).
- UI check (agent composer shows the branch pill `feat/bachuc-gpui`) -> DEFERRED (UI checker).
- UI check (turn diff card with file rows and diffstat bars) -> DEFERRED (UI checker).
- The view wiring (pill menus, card, panel, scrolling lists, image decode, the live reads) needs a window and a daemon: UNVERIFIED (unattended): code tests only. Live daemon down: no live check.
- GitNexus impact: this TASK only adds a crate and edits no existing symbol, so no impact run applied.

## Anti-goal

- Checkpoint 1, when the store landed: `poll_one_read_per_cwd` passes. Two agents and a draft in `/repo` give one cwd in `live_cwds`, so `due` returns `/repo` once; a second `due` before the answer returns none; an unasked cwd and a cwd that no agent or draft uses get no read. Source: first test run, `/tmp/tb-test2.txt`; 2026-10-17 01:10.
- Checkpoint 1b, same time: `same_signature_no_notify` passes. `State::apply` returns false for an unchanged info and true only for a changed field, and `failed` keeps the last info. Source: `/tmp/tb-test2.txt`; 2026-10-17 01:10.
- Checkpoint 2, at completion: both tests pass again in the run with 37 tests, plus `a_failed_first_read_waits_for_the_next_tick`; source `/tmp/tb-test4.txt`; 2026-10-17 01:22. The notify call sits in `Branches::load` and runs only when `apply` returns true.

## Lessons

- A pill `state` closure runs on every composer draw and holds only `&App`. A first read that fails would restart on each draw, so a failed first read waits for the next tick (`State.failed`).
- Rows with wrapped text have different heights, so the Wrap-on diff uses the variable-height `list` and the Wrap-off diff uses `uniform_list` with horizontal sizing. `mono_font_family` lists the installed fonts, so the panel looks it up once per draw and not once per row.
- A cargo command in this checkout failed twice because another builder's crate (`next-prompt-actions`, `jev-orchestrator`) had a `Cargo.toml` but no `src/lib.rs` yet. Waiting a few minutes cleared it.
- Safe defaults taken (see questions): the menu popover reads the store each time it draws, so its rows follow the last info; the "too long" note sits under the diff list, not at the end of the scrolled content; the history poll runs while the panel view lives and a file of the agent is not open, and the host may keep a hidden panel view; a plain space separates the time and PM in `Oct 16, 03:45 PM`, where ICU may use U+202F; the gutter width follows the 12 px code size constant, not the code size setting; a failed diff or image read is read again the next time a file opens.
