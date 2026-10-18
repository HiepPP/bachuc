# TASK-220 outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/client/src/timeline.rs`: `TimelineSync::epoch()`.
- `packages/desktop-gpui/crates/client/src/core.rs`: `timeline_epoch`, `discard_changes_supported`, `directory_suggestions_matching`, and the shared `directory_suggestions_message` builder.
- `packages/desktop-gpui/crates/client/src/lib.rs`: `Client` wrappers for the three calls, and the `MatchMode` export.
- `packages/desktop-gpui/crates/client/src/model.rs`: `MatchMode`, `PrCheck.duration` and `traits`, and the three `WorkspaceScript` proxy URL fields.
- `packages/desktop-gpui/crates/client/tests/rpcs.rs`: the 3 literals and 5 new tests.

## Contract

- `Client::timeline_epoch(&self, agent_id: &str) -> Option<&str>`: the epoch of the latest page. It is `None` for an agent with no viewed timeline or before the first page.
- `Client::discard_changes_supported(&self) -> bool` reads `features.checkoutDiscardChanges`. `discard_changes` itself is not gated.
- `MatchMode { Fuzzy, Suffix }` is exported from `bachuc_client`.
- `Client::directory_suggestions_matching(cwd, query, include_files, include_directories, limit, mode, cx)` sends the old request plus `matchMode`. It has no feature gate, because an older daemon strips the unknown key.
- `PrCheck` has `duration: Option<String>` and `traits: Vec<String>`. A missing `traits` gives an empty vector.
- `WorkspaceScript` has `proxy_url`, `local_proxy_url`, and `public_proxy_url`, all `Option<String>`. A null value gives `None`.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-client` -> pass. The 5 new tests pass: `timeline_epoch_follows_the_page_and_a_reset_page`, `discard_changes_supported_follows_the_host_feature`, `directory_suggestions_matching_sends_the_match_mode`, `pr_checks_parse_duration_and_traits`, and `workspace_scripts_parse_their_proxy_urls`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> builds.
- Anti-goal: `git diff` of `tests/rpcs.rs` has only added lines. `directory_suggestions_send_their_fields_and_read_entries_or_directories` is unchanged and passes. The old request is built by the same helper with the same keys.
- `./cargo.sh fmt --check -p bachuc-client` -> exit 0.
- `./cargo.sh metadata --format-version 1` -> exit 0.
- GitNexus impact for `PrCheck` and `WorkspaceScript` -> UNKNOWN (stale index, target not found). `rg` confirmed the struct literals are only at the sites the spec lists.
- Known open: `cargo test -p bachuc-explorer` and `-p bachuc-app` do not compile until TASK-222 and TASK-231 fix their literals. This TASK did not run them.

## Lessons

- To test a reset page with the fake daemon, change `timelines[agent].epoch` and call `drop_connections`. The resume cursor then has the old epoch, and the daemon answers `reset: true`.
- `Client::directory_suggestions_matching` has 8 parameters, so it carries `#[allow(clippy::too_many_arguments)]`.
