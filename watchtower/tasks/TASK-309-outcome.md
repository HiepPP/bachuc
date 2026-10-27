# TASK-309 Outcome

Status: DONE

## Changed

- `packages/desktop-gpui/crates/client/src/core.rs`: `AgentExtra.attention_timestamp`, set in `upsert_agent` after the old-snapshot early return, beside `attention_reason`; `ClientCore::agent_attention_timestamp`.
- `packages/desktop-gpui/crates/client/src/lib.rs`: `Client::agent_attention_timestamp`.
- `packages/desktop-gpui/crates/client/tests/rpcs.rs`: red test `agent_attention_timestamp_reads_the_snapshot_value`.

## Contract

- `ClientCore::agent_attention_timestamp(&self, agent_id: &str) -> Option<String>`
- `Client::agent_attention_timestamp(&self, agent_id: &str) -> Option<String>`
- Null, absent, blank (whitespace-only), or unknown agent gives `None`. A non-blank value is returned unchanged (not trimmed).

## Verified

- Red: before the change the test failed to compile (E0599, missing `agent_attention_timestamp`).
- `./cargo.sh test -p bachuc-client --test rpcs agent_attention` -> 2 passed.
- Group-level run, `./cargo.sh test -p bachuc-client --no-fail-fast`: all pass except `sync::one_thousand_workspace_updates_stay_cheap` (known timing failure; it also failed in the baseline run before the change). Alone it passes (`--test sync one_thousand_workspace_updates_stay_cheap` -> ok).
- `./cargo.sh fmt --check -p bachuc-client` -> exit 0.
- Clippy `--no-deps -D warnings`: one error, `manual_contains` at `core.rs:2802` (`viewed_provider_subagents`), the known failure (old line 2711); my hunks are at lines 285, 702, 3781.
- GitNexus impact on `AgentExtra`: UNKNOWN (no callers resolved); confirmed with rg that it is private to `core.rs` (struct line 276, used only in the `extras` map at line 405).

## Anti-goal

Existing `bachuc-client` tests: 0 new failures.

- Baseline (before change), `./cargo.sh test -p bachuc-client`: rpcs-binary 83 passed; sync 21 passed, 1 failed (`one_thousand_workspace_updates_stay_cheap`, timing, known failure). Cargo stopped at that binary, so later binaries did not run.
- Completion (`--no-fail-fast`): rpcs 84 passed (83 + the new test); sync 21 passed, 1 failed (same known timing test, passes alone); all other binaries 0 failed.

## Lessons

- `cargo test` stops after the first failing binary; use `--no-fail-fast` for the group run.
- `upsert_agent` returns early on an older `updatedAt` before any extra is written, so setting the field beside `attention_reason` is safe.
